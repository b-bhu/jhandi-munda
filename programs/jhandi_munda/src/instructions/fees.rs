//! Fee distribution and staker rewards (D15, D18, D38; #35).
//!
//! `distribute_fees` moves the accrued win-fee pots out of the pool vault:
//! - 10% to the dev vault;
//! - 20% to the insurance vault until it holds `INSURANCE_TARGET`, then to the
//!   treasury vault (D18, D38);
//! - 70% is credited to stakers in proportion to their stake, as `pending`
//!   rewards on each Stake. `claim_stake_rewards` pays them out.
//!
//! The crank must be given every Stake account, exactly once: the accounts
//! must add up to `StakePool::total_staked` and their count to
//! `StakePool::stakers`. So no staker can be skipped. Unstaked MUNDA holders
//! have no Stake account and receive nothing. Fee moves wait for the payout
//! queue to be empty (D30), the same rule as LP withdraws.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    error::ErrorCode,
    instructions::claim_redeem::{spl_transfer, token_balance},
    state::{PayoutQueue, PoolState, ProgramConfig, Stake, StakePool, TableConfig},
    CONFIG_SEED, DEV_VAULT_SEED, INSURANCE_TARGET, INSURANCE_VAULT_SEED, MUSD_MINT_SEED,
    POOL_SEED, POOL_VAULT_SEED, QUEUE_SEED, SPL_TOKEN_PROGRAM_ID, STAKE_POOL_SEED, STAKE_SEED,
    TABLE_SEED, TOKEN_ACCOUNT_SPACE, TREASURY_AUTHORITY_SEED, TREASURY_VAULT_SEED,
};

/// SPL token `InitializeAccount3` (opcode 18): data [18][authority 32 bytes].
const TOKEN_IX_INITIALIZE_ACCOUNT3: u8 = 18;

/// Creates a MUSD token account at `vault`, owned by `authority`.
fn init_token_account<'info>(
    vault: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    authority: &Pubkey,
) -> Result<()> {
    let mut data = Vec::with_capacity(33);
    data.push(TOKEN_IX_INITIALIZE_ACCOUNT3);
    data.extend_from_slice(authority.as_ref());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(vault.key(), false),
            AccountMeta::new_readonly(mint.key(), false),
        ],
        data,
    };
    invoke(&ix, &[vault.clone(), mint.clone()])?;
    Ok(())
}

#[derive(Accounts)]
pub struct InitFeeVaults<'info> {
    /// Program config: pins which MUSD mint the vaults hold.
    #[account(seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, ProgramConfig>,

    /// CHECK: the dev treasury vault (10% of each win fee), created here.
    #[account(
        init,
        payer = payer,
        space = TOKEN_ACCOUNT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [DEV_VAULT_SEED],
        bump
    )]
    pub dev_vault: UncheckedAccount<'info>,

    /// CHECK: the treasury vault (the 20% slice once insurance is full), created here.
    #[account(
        init,
        payer = payer,
        space = TOKEN_ACCOUNT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [TREASURY_VAULT_SEED],
        bump
    )]
    pub treasury_vault: UncheckedAccount<'info>,

    /// CHECK: the Step 2 MUSD mint, pinned by the config.
    #[account(
        seeds = [MUSD_MINT_SEED],
        bump,
        owner = SPL_TOKEN_PROGRAM_ID,
        constraint = config.musd_mint == musd_mint.key() @ ErrorCode::MusdMintMismatch
    )]
    pub musd_mint: UncheckedAccount<'info>,

    /// CHECK: the treasury authority PDA, owner of the dev and treasury vaults.
    #[account(seeds = [TREASURY_AUTHORITY_SEED], bump)]
    pub treasury_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_init_fee_vaults(ctx: Context<InitFeeVaults>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    let authority = ctx.accounts.treasury_authority.key();
    init_token_account(
        &ctx.accounts.dev_vault.to_account_info(),
        &ctx.accounts.musd_mint.to_account_info(),
        &authority,
    )?;
    init_token_account(
        &ctx.accounts.treasury_vault.to_account_info(),
        &ctx.accounts.musd_mint.to_account_info(),
        &authority,
    )?;
    msg!("Dev and treasury vaults initialized");
    Ok(())
}

#[derive(Accounts)]
pub struct DistributeFees<'info> {
    /// Table settings: holds the accrued fee pots.
    #[account(mut, seeds = [TABLE_SEED], bump)]
    pub table: Account<'info, TableConfig>,

    /// Staking totals: checks the stake accounts given are the complete set.
    #[account(mut, seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Account<'info, StakePool>,

    /// The house pool: its vault pays the fee slices out.
    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool MUSD vault, seeds pin it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump)]
    pub pool_vault: UncheckedAccount<'info>,

    /// The payout queue: fees wait until every queued winner is paid (D30).
    #[account(seeds = [QUEUE_SEED], bump)]
    pub queue: AccountLoader<'info, PayoutQueue>,

    /// CHECK: the dev treasury vault, seeds pin it.
    #[account(mut, seeds = [DEV_VAULT_SEED], bump)]
    pub dev_vault: UncheckedAccount<'info>,

    /// CHECK: the insurance vault, seeds pin it.
    #[account(mut, seeds = [INSURANCE_VAULT_SEED], bump)]
    pub insurance_vault: UncheckedAccount<'info>,

    /// CHECK: the treasury vault, seeds pin it.
    #[account(mut, seeds = [TREASURY_VAULT_SEED], bump)]
    pub treasury_vault: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,
    // Remaining accounts: every Stake account, once each, writable.
}

/// Eligible stake for a fee distribution (D29): each lot counts once it has
/// matured (see `Stake::eligible`).
fn eligible_amount(stake: &Stake, now: i64) -> u64 {
    stake.eligible(now)
}

pub fn handle_distribute_fees<'info>(ctx: Context<'info, DistributeFees<'info>>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    {
        let queue = ctx.accounts.queue.load()?;
        require!(queue.head == queue.tail, ErrorCode::QueueNotEmpty);
    }

    let vault = ctx.accounts.pool_vault.to_account_info();
    let pool_info = ctx.accounts.pool.to_account_info();
    let seeds: &[&[u8]] = &[POOL_SEED, &[ctx.accounts.pool.bump]];

    // 10% to the dev treasury.
    let dev_pot = ctx.accounts.table.fee_devs;
    if dev_pot > 0 {
        spl_transfer(&vault, &ctx.accounts.dev_vault.to_account_info(), &pool_info, dev_pot, &[seeds])?;
    }
    ctx.accounts.table.fee_devs = 0;

    // 20%: insurance until it holds INSURANCE_TARGET, then the treasury (D18).
    // The target is a demo number: 1,000 MUSD.
    let treasury_pot = ctx.accounts.table.fee_treasury;
    let room = INSURANCE_TARGET.saturating_sub(token_balance(&ctx.accounts.insurance_vault)?);
    let to_insurance = treasury_pot.min(room);
    let to_treasury = treasury_pot - to_insurance;
    if to_insurance > 0 {
        spl_transfer(&vault, &ctx.accounts.insurance_vault.to_account_info(), &pool_info, to_insurance, &[seeds])?;
    }
    if to_treasury > 0 {
        spl_transfer(&vault, &ctx.accounts.treasury_vault.to_account_info(), &pool_info, to_treasury, &[seeds])?;
    }
    ctx.accounts.table.fee_treasury = 0;

    // 70% to stakers, in proportion to their eligible stake. Load every stake
    // account and check the set is complete before crediting anyone.
    let staker_pot = ctx.accounts.table.fee_stakers;
    require!(
        ctx.remaining_accounts.len() == ctx.accounts.stake_pool.stakers as usize,
        ErrorCode::StakerSetMismatch
    );
    let mut stakes: Vec<Account<'info, Stake>> = Vec::with_capacity(ctx.remaining_accounts.len());
    let mut staked_total = 0u64;
    for info in ctx.remaining_accounts.iter() {
        let stake: Account<Stake> = Account::try_from(info)?;
        let expected = Pubkey::find_program_address(&[STAKE_SEED, stake.owner.as_ref()], &crate::ID).0;
        require_keys_eq!(info.key(), expected, ErrorCode::StakerSetMismatch);
        staked_total = staked_total.checked_add(stake.amount).ok_or(ErrorCode::MathOverflow)?;
        stakes.push(stake);
    }
    require!(staked_total == ctx.accounts.stake_pool.total_staked, ErrorCode::StakerSetMismatch);

    let now = Clock::get()?.unix_timestamp;
    let eligible_total: u64 = stakes.iter().map(|s| eligible_amount(s, now)).sum();
    if eligible_total > 0 && staker_pot > 0 {
        let mut distributed = 0u64;
        for stake in stakes.iter_mut() {
            let credit = (staker_pot as u128 * eligible_amount(stake, now) as u128
                / eligible_total as u128) as u64;
            stake.pending = stake.pending.checked_add(credit).ok_or(ErrorCode::MathOverflow)?;
            distributed += credit;
        }
        // Rounding dust stays in the pot for the next distribution.
        ctx.accounts.table.fee_stakers = staker_pot - distributed;
    }
    for stake in stakes.iter() {
        stake.exit(&crate::ID)?;
    }
    msg!("Fees distributed: staker pot {} across {} stakes", staker_pot, stakes.len());
    Ok(())
}

#[derive(Accounts)]
pub struct ClaimStakeRewards<'info> {
    /// The claimant's stake: its seeds bind it to the signer.
    #[account(mut, seeds = [STAKE_SEED, owner.key().as_ref()], bump = stake.bump)]
    pub stake: Account<'info, Stake>,

    /// The staker: signs, and receives the rewards.
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool MUSD vault, seeds pin it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump)]
    pub pool_vault: UncheckedAccount<'info>,

    /// The payout queue: rewards wait until every queued winner is paid (D30).
    #[account(seeds = [QUEUE_SEED], bump)]
    pub queue: AccountLoader<'info, PayoutQueue>,

    /// CHECK: the staker's MUSD ATA, checked against the canonical address.
    #[account(mut)]
    pub owner_musd_ata: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,
}

pub fn handle_claim_stake_rewards(ctx: Context<ClaimStakeRewards>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    {
        let queue = ctx.accounts.queue.load()?;
        require!(queue.head == queue.tail, ErrorCode::QueueNotEmpty);
    }
    let amount = ctx.accounts.stake.pending;
    require!(amount > 0, ErrorCode::NothingToClaim);
    require_keys_eq!(
        ctx.accounts.owner_musd_ata.key(),
        crate::musd_ata(&ctx.accounts.owner.key(), &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );

    ctx.accounts.stake.pending = 0;
    let seeds: &[&[u8]] = &[POOL_SEED, &[ctx.accounts.pool.bump]];
    spl_transfer(
        &ctx.accounts.pool_vault.to_account_info(),
        &ctx.accounts.owner_musd_ata.to_account_info(),
        &ctx.accounts.pool.to_account_info(),
        amount,
        &[seeds],
    )?;
    msg!("Stake rewards claimed: {} base MUSD", amount);
    Ok(())
}
