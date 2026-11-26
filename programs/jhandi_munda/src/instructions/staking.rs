//! MUNDA staking (D15, D26, D27, D29; #33, #34). Staked MUNDA sits in a program
//! vault and is the only MUNDA that earns the 70% fee slice. `unstake` returns
//! principal at any time (D29); rewards come in Steps 9.3 and 10.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
};

use crate::{
    error::ErrorCode,
    state::{ProgramConfig, Stake, StakeLot, StakePool},
    CONFIG_SEED, MUNDA_MINT_SEED, SPL_TOKEN_PROGRAM_ID, STAKE_AUTHORITY_SEED, STAKE_POOL_SEED, STAKE_SEED,
    STAKE_VAULT_SEED, TOKEN_ACCOUNT_SPACE,
};

/// SPL token `InitializeAccount3` (opcode 18): data [18][authority 32 bytes].
const TOKEN_IX_INITIALIZE_ACCOUNT3: u8 = 18;

#[derive(Accounts)]
pub struct InitStaking<'info> {
    /// Pool-wide staking totals, created empty.
    #[account(
        init,
        payer = payer,
        space = 8 + StakePool::INIT_SPACE,
        seeds = [STAKE_POOL_SEED],
        bump
    )]
    pub stake_pool: Account<'info, StakePool>,

    /// CHECK: the staked MUNDA vault, created here and owned by the stake authority PDA.
    #[account(
        init,
        payer = payer,
        space = TOKEN_ACCOUNT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [STAKE_VAULT_SEED],
        bump
    )]
    pub stake_vault: UncheckedAccount<'info>,

    /// CHECK: the MUNDA mint, pinned by its seeds.
    #[account(seeds = [MUNDA_MINT_SEED], bump, owner = SPL_TOKEN_PROGRAM_ID)]
    pub munda_mint: UncheckedAccount<'info>,

    /// CHECK: the stake authority PDA; signs unstakes out of the vault.
    #[account(seeds = [STAKE_AUTHORITY_SEED], bump)]
    pub stake_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_init_staking(ctx: Context<InitStaking>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    let mut data = Vec::with_capacity(33);
    data.push(TOKEN_IX_INITIALIZE_ACCOUNT3);
    data.extend_from_slice(ctx.accounts.stake_authority.key().as_ref());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.stake_vault.key(), false),
            AccountMeta::new_readonly(ctx.accounts.munda_mint.key(), false),
        ],
        data,
    };
    invoke(
        &ix,
        &[
            ctx.accounts.stake_vault.to_account_info(),
            ctx.accounts.munda_mint.to_account_info(),
        ],
    )?;

    let pool = &mut ctx.accounts.stake_pool;
    pool.total_staked = 0;
    pool.stakers = 0;
    pool.bump = ctx.bumps.stake_pool;
    msg!("Staking initialized");
    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct StakeMunda<'info> {
    #[account(mut, seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Account<'info, StakePool>,

    /// The wallet's stake, created on its first stake.
    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + Stake::INIT_SPACE,
        seeds = [STAKE_SEED, owner.key().as_ref()],
        bump
    )]
    pub stake: Account<'info, Stake>,

    /// CHECK: the staked MUNDA vault, seeds pin it.
    #[account(mut, seeds = [STAKE_VAULT_SEED], bump)]
    pub stake_vault: UncheckedAccount<'info>,

    /// The staker: signs the MUNDA transfer into the vault.
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: the staker's MUNDA ATA, checked against the canonical address.
    #[account(mut)]
    pub owner_munda_ata: UncheckedAccount<'info>,

    /// CHECK: the MUNDA mint, pinned by its seeds.
    #[account(seeds = [MUNDA_MINT_SEED], bump)]
    pub munda_mint: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_stake(ctx: Context<StakeMunda>, amount: u64) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    require!(amount > 0, ErrorCode::ZeroStake);
    let owner = ctx.accounts.owner.key();
    require_keys_eq!(
        ctx.accounts.owner_munda_ata.key(),
        crate::musd_ata(&owner, &ctx.accounts.munda_mint.key()),
        ErrorCode::InvalidAtaAddress
    );
    let now = Clock::get()?.unix_timestamp;

    let pool = &mut ctx.accounts.stake_pool;
    let stake = &mut ctx.accounts.stake;
    if stake.owner == Pubkey::default() {
        stake.owner = owner;
        stake.bump = ctx.bumps.stake;
        pool.stakers += 1;
    }
    // Each stake makes its own lot with its own clock (Step 10.2). Topping up never
    // resets an older lot.
    let slot = stake
        .lots
        .iter()
        .position(|lot| lot.amount == 0)
        .ok_or(ErrorCode::TooManyLots)?;
    stake.lots[slot] = StakeLot { amount, staked_at: now };
    stake.amount = stake.amount.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    pool.total_staked = pool.total_staked.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;

    // Move the MUNDA into the vault, signed by the staker.
    let mut data = Vec::with_capacity(9);
    data.push(3u8);
    data.extend_from_slice(&amount.to_le_bytes());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.owner_munda_ata.key(), false),
            AccountMeta::new(ctx.accounts.stake_vault.key(), false),
            AccountMeta::new_readonly(owner, true),
        ],
        data,
    };
    invoke(
        &ix,
        &[
            ctx.accounts.owner_munda_ata.to_account_info(),
            ctx.accounts.stake_vault.to_account_info(),
            ctx.accounts.owner.to_account_info(),
        ],
    )?;
    msg!("Staked {} base MUNDA", amount);
    Ok(())
}

#[derive(Accounts)]
pub struct UnstakeMunda<'info> {
    /// The program config: its `transfers_enabled` flag gates unstake (D46).
    #[account(seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, ProgramConfig>,

    #[account(mut, seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Account<'info, StakePool>,

    /// The wallet's stake. Its seeds bind it to the signer.
    #[account(mut, seeds = [STAKE_SEED, owner.key().as_ref()], bump = stake.bump)]
    pub stake: Account<'info, Stake>,

    /// CHECK: the staked MUNDA vault, seeds pin it.
    #[account(mut, seeds = [STAKE_VAULT_SEED], bump)]
    pub stake_vault: UncheckedAccount<'info>,

    /// CHECK: the stake authority PDA; signs the vault's transfer back to the staker.
    #[account(seeds = [STAKE_AUTHORITY_SEED], bump)]
    pub stake_authority: UncheckedAccount<'info>,

    /// The staker: receives the principal back.
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: the staker's MUNDA ATA, checked against the canonical address.
    #[account(mut)]
    pub owner_munda_ata: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,
}

/// Returns `amount` staked MUNDA to the staker at any time (D29). Unstaking more
/// than is staked is rejected before anything moves.
pub fn handle_unstake(ctx: Context<UnstakeMunda>, amount: u64) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    // Launch phase: MUNDA stays staked until the admin turns transfers on (D46).
    require!(ctx.accounts.config.transfers_enabled, ErrorCode::TransfersDisabled);
    require!(amount > 0, ErrorCode::ZeroStake);
    let owner = ctx.accounts.owner.key();
    require_keys_eq!(
        ctx.accounts.owner_munda_ata.key(),
        crate::musd_ata(&owner, &pda_munda_mint()),
        ErrorCode::InvalidAtaAddress
    );
    require!(amount <= ctx.accounts.stake.amount, ErrorCode::InsufficientStake);

    // Unstake from the newest lots first, so the mature lots keep earning.
    let mut left = amount;
    while left > 0 {
        let lots = &mut ctx.accounts.stake.lots;
        let newest = lots
            .iter_mut()
            .filter(|lot| lot.amount > 0)
            .max_by_key(|lot| lot.staked_at)
            .ok_or(ErrorCode::InsufficientStake)?;
        let take = left.min(newest.amount);
        newest.amount -= take;
        left -= take;
    }
    ctx.accounts.stake.amount -= amount;
    ctx.accounts.stake_pool.total_staked -= amount;

    let mut data = Vec::with_capacity(9);
    data.push(3u8);
    data.extend_from_slice(&amount.to_le_bytes());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.stake_vault.key(), false),
            AccountMeta::new(ctx.accounts.owner_munda_ata.key(), false),
            AccountMeta::new_readonly(ctx.accounts.stake_authority.key(), true),
        ],
        data,
    };
    let seeds: &[&[u8]] = &[STAKE_AUTHORITY_SEED, &[ctx.bumps.stake_authority]];
    invoke_signed(
        &ix,
        &[
            ctx.accounts.stake_vault.to_account_info(),
            ctx.accounts.owner_munda_ata.to_account_info(),
            ctx.accounts.stake_authority.to_account_info(),
        ],
        &[seeds],
    )?;
    msg!("Unstaked {} base MUNDA", amount);
    Ok(())
}

#[derive(Accounts)]
pub struct SetTransfersEnabled<'info> {
    #[account(mut, seeds = [CONFIG_SEED], bump, has_one = admin)]
    pub config: Account<'info, ProgramConfig>,

    /// The config admin: the only key that can turn MUNDA transfers on or off (D46).
    pub admin: Signer<'info>,
}

pub fn handle_set_transfers_enabled(ctx: Context<SetTransfersEnabled>, enabled: bool) -> Result<()> {
    ctx.accounts.config.transfers_enabled = enabled;
    msg!("MUNDA transfers enabled: {}", enabled);
    Ok(())
}

/// The MUNDA mint's address, from its seeds.
fn pda_munda_mint() -> Pubkey {
    Pubkey::find_program_address(&[MUNDA_MINT_SEED], &crate::ID).0
}
