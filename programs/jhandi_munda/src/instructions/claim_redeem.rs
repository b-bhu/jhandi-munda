//! Redeeming claims (D7, D13, D23, D26; #26, #27).
//!
//! Win claims (MUSD):
//! - **Wait**: after the claim unlocks (settle + 90 s), pay 100% of the owed
//!   MUSD from the pool vault.
//! - **Take now**: pay 95% at once from the insurance vault. The claim then owes
//!   the full 100% back, which `collect_insurance` takes from the pool after the
//!   unlock. The insurance fund keeps the 5%.
//!
//! Loss claims (MUNDA):
//! - **Wait**: after 24 hours, mint the full amount (1 MUNDA per 1 MUSD lost).
//! - **Take now**: mint 90% at once. The 10% tax is never minted, so it stays
//!   out of supply (a burn in effect).
//! - Either way the MUNDA is minted straight into the player's stake, not their
//!   wallet (D47). The stake lot's clock starts at the loss's settle time (D29).
//!
//! MUNDA is minted only on redeem, never at settle (D26). No FIFO queue here:
//! if the pool cannot pay a win, the redeem is rejected (Step 8). Token moves are
//! hand-built SPL instructions, as in the earlier steps.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
};
use anchor_lang::system_program::{create_account, CreateAccount};

use crate::{
    error::ErrorCode,
    instructions::payout_queue::enqueue,
    instructions::settle::save_account,
    payout::take_now_payout,
    state::{Claim, ClaimAsset, PayoutQueue, PoolState, ProgramConfig, Stake, StakeLot, StakePool},
    CLAIM_SEED, CONFIG_SEED, INSURANCE_SEED, INSURANCE_VAULT_SEED, LOSS_UNLOCK_SECS, MUNDA_DECIMALS, QUEUE_SEED,
    MUNDA_MINT_AUTHORITY_SEED, MUNDA_MINT_SEED, MUSD_MINT_SEED, MUSD_MINT_SPACE, POOL_SEED,
    POOL_VAULT_SEED, SPL_TOKEN_PROGRAM_ID, STAKE_POOL_SEED, STAKE_SEED, STAKE_VAULT_SEED, TOKEN_ACCOUNT_SPACE,
};

/// SPL token `Transfer` (opcode 3): data [3][amount u64 LE].
const TOKEN_IX_TRANSFER: u8 = 3;
/// SPL token `MintTo` (opcode 7): data [7][amount u64 LE].
const TOKEN_IX_MINT_TO: u8 = 7;
/// SPL token `InitializeAccount3` (opcode 18): data [18][authority 32 bytes].
const TOKEN_IX_INITIALIZE_ACCOUNT3: u8 = 18;
/// SPL token `InitializeMint2` (opcode 20): data [20][decimals][authority][0].
const TOKEN_IX_INITIALIZE_MINT2: u8 = 20;
/// A token account keeps its balance at bytes 64..72.
const TOKEN_AMOUNT_OFFSET: usize = 64;

pub(crate) fn token_balance(info: &AccountInfo) -> Result<u64> {
    let data = info.try_borrow_data()?;
    let bytes = data
        .get(TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8)
        .ok_or(ErrorCode::InvalidTokenAccount)?;
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| ErrorCode::InvalidTokenAccount)?))
}

/// SPL `Transfer` of `amount` from `from` to `to`, authorised by `authority`
/// (a wallet that signed the transaction, or a PDA via `signer_seeds`).
pub(crate) fn spl_transfer<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_TRANSFER);
    data.extend_from_slice(&amount.to_le_bytes());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(from.key(), false),
            AccountMeta::new(to.key(), false),
            AccountMeta::new_readonly(authority.key(), true),
        ],
        data,
    };
    invoke_signed(&ix, &[from.clone(), to.clone(), authority.clone()], signer_seeds)?;
    Ok(())
}

#[derive(Accounts)]
pub struct InitInsurance<'info> {
    /// Program config: pins which MUSD mint the insurance vault holds.
    #[account(seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, ProgramConfig>,

    /// CHECK: the insurance vault, a token account owned by the insurance PDA.
    /// Created here, then initialised by the handler.
    #[account(
        init,
        payer = payer,
        space = TOKEN_ACCOUNT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [INSURANCE_VAULT_SEED],
        bump
    )]
    pub insurance_vault: UncheckedAccount<'info>,

    /// CHECK: the Step 2 MUSD mint, pinned by the config.
    #[account(
        seeds = [MUSD_MINT_SEED],
        bump,
        owner = SPL_TOKEN_PROGRAM_ID,
        constraint = config.musd_mint == musd_mint.key() @ ErrorCode::MusdMintMismatch
    )]
    pub musd_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_init_insurance(ctx: Context<InitInsurance>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    // The insurance PDA is the vault's authority; only "Take now" signs as it.
    let authority = Pubkey::find_program_address(&[INSURANCE_SEED], &crate::ID).0;
    let mut data = Vec::with_capacity(33);
    data.push(TOKEN_IX_INITIALIZE_ACCOUNT3);
    data.extend_from_slice(authority.as_ref());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.insurance_vault.key(), false),
            AccountMeta::new_readonly(ctx.accounts.musd_mint.key(), false),
        ],
        data,
    };
    invoke(
        &ix,
        &[
            ctx.accounts.insurance_vault.to_account_info(),
            ctx.accounts.musd_mint.to_account_info(),
        ],
    )?;
    msg!("Insurance vault initialized");
    Ok(())
}

#[derive(Accounts)]
pub struct InitMunda<'info> {
    /// CHECK: the MUNDA mint, created here (82 bytes, owned by the SPL token
    /// program) and initialised by the handler. Its authority is a PDA.
    #[account(
        init,
        payer = payer,
        space = MUSD_MINT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [MUNDA_MINT_SEED],
        bump
    )]
    pub munda_mint: UncheckedAccount<'info>,

    /// CHECK: the MUNDA mint authority PDA. Only the program signs MUNDA mints.
    #[account(seeds = [MUNDA_MINT_AUTHORITY_SEED], bump)]
    pub munda_mint_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_init_munda(ctx: Context<InitMunda>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    // Six decimals like MUSD (D26). Freeze authority none.
    let mut data = Vec::with_capacity(35);
    data.push(TOKEN_IX_INITIALIZE_MINT2);
    data.push(MUNDA_DECIMALS);
    data.extend_from_slice(ctx.accounts.munda_mint_authority.key().as_ref());
    data.push(0u8);
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![AccountMeta::new(ctx.accounts.munda_mint.key(), false)],
        data,
    };
    invoke(&ix, &[ctx.accounts.munda_mint.to_account_info()])?;
    msg!("MUNDA mint initialized");
    Ok(())
}

#[derive(Accounts)]
pub struct RedeemClaim<'info> {
    /// The bet the claim settles. The claim's seeds bind it.
    /// CHECK: only its key is used, to find the claim PDA.
    pub bet: UncheckedAccount<'info>,

    /// The claim being paid. Its player must sign.
    #[account(
        mut,
        seeds = [CLAIM_SEED, bet.key().as_ref()],
        bump = claim.bump,
        has_one = player
    )]
    pub claim: Account<'info, Claim>,

    /// The claim's wallet: signs, pays any ATA rent, and receives the payout.
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// The payout queue: a win the pool cannot pay now waits here (D4, D30).
    #[account(mut, seeds = [QUEUE_SEED], bump)]
    pub queue: AccountLoader<'info, PayoutQueue>,

    /// CHECK: the pool MUSD vault, seeds pin it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump)]
    pub pool_vault: UncheckedAccount<'info>,

    /// CHECK: the insurance vault, seeds pin it.
    #[account(mut, seeds = [INSURANCE_VAULT_SEED], bump)]
    pub insurance_vault: UncheckedAccount<'info>,

    /// CHECK: the insurance authority PDA; signs "Take now" payouts.
    #[account(seeds = [INSURANCE_SEED], bump)]
    pub insurance_authority: UncheckedAccount<'info>,

    /// CHECK: the player's MUSD ATA (win payouts), checked in the handler.
    #[account(mut)]
    pub player_ata: UncheckedAccount<'info>,

    /// CHECK: the MUNDA mint (loss payouts), seeds pin it.
    #[account(mut, seeds = [MUNDA_MINT_SEED], bump)]
    pub munda_mint: UncheckedAccount<'info>,

    /// CHECK: the MUNDA mint authority PDA; signs MUNDA mints.
    #[account(seeds = [MUNDA_MINT_AUTHORITY_SEED], bump)]
    pub munda_mint_authority: UncheckedAccount<'info>,

    /// CHECK: the stake pool (loss payouts only), seeds checked in the handler (D47).
    #[account(mut)]
    pub stake_pool: UncheckedAccount<'info>,

    /// CHECK: the player's stake (loss payouts only), seeds checked in the handler. Created on
    /// the player's first loss, so the minted MUNDA lands in it, not in their wallet (D47).
    #[account(mut)]
    pub stake: UncheckedAccount<'info>,

    /// CHECK: the staked MUNDA vault (loss payouts only), seeds checked in the handler.
    #[account(mut)]
    pub stake_vault: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

fn check_token_program(ctx: &Context<RedeemClaim>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    Ok(())
}

/// The player's MUSD ATA must be the canonical one (win payouts).
fn check_player_musd_ata(ctx: &Context<RedeemClaim>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.player_ata.key(),
        crate::musd_ata(&ctx.accounts.player.key(), &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );
    Ok(())
}

/// Loss MUNDA goes straight into the player's stake, not their wallet (D47). The
/// stake is created on the player's first loss. The MUNDA is minted to the stake vault
/// and added to a lot whose clock starts at `settled_at` (the loss's settle time), so a Wait
/// claim counts at once and a Take now claim after 24 hours (D29). When all lots are in use,
/// the MUNDA is folded into the newest lot, which keeps its own clock.
fn credit_loss_munda(ctx: &Context<RedeemClaim>, amount: u64, settled_at: i64) -> Result<()> {
    let player = ctx.accounts.player.key();
    let pool_info = ctx.accounts.stake_pool.to_account_info();
    let stake_info = ctx.accounts.stake.to_account_info();
    let vault_info = ctx.accounts.stake_vault.to_account_info();
    require_keys_eq!(
        pool_info.key(),
        Pubkey::find_program_address(&[STAKE_POOL_SEED], &crate::ID).0,
        ErrorCode::InvalidStakeAccount
    );
    require_keys_eq!(
        vault_info.key(),
        Pubkey::find_program_address(&[STAKE_VAULT_SEED], &crate::ID).0,
        ErrorCode::InvalidStakeAccount
    );
    require_keys_eq!(*pool_info.owner, crate::ID, ErrorCode::InvalidStakeAccount);
    let mut pool = {
        let data = pool_info.try_borrow_data()?;
        StakePool::try_deserialize(&mut &data[..])?
    };

    let (stake_key, stake_bump) = Pubkey::find_program_address(&[STAKE_SEED, player.as_ref()], &crate::ID);
    require_keys_eq!(stake_info.key(), stake_key, ErrorCode::InvalidStakeAccount);
    let mut stake = if stake_info.data_is_empty() {
        // The first loss for this wallet: create its stake account, paid by the player.
        let space = 8 + Stake::INIT_SPACE;
        let lamports = Rent::get()?.minimum_balance(space);
        let seeds: &[&[u8]] = &[STAKE_SEED, player.as_ref(), &[stake_bump]];
        create_account(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.key(),
                CreateAccount { from: ctx.accounts.player.to_account_info(), to: stake_info.clone() },
                &[seeds],
            ),
            lamports,
            space as u64,
            &crate::ID,
        )?;
        pool.stakers = pool.stakers.checked_add(1).ok_or(ErrorCode::MathOverflow)?;
        Stake { owner: player, amount: 0, lots: Default::default(), pending: 0, bump: stake_bump }
    } else {
        require_keys_eq!(*stake_info.owner, crate::ID, ErrorCode::InvalidStakeAccount);
        let data = stake_info.try_borrow_data()?;
        Stake::try_deserialize(&mut &data[..])?
    };

    match stake.lots.iter().position(|lot| lot.amount == 0) {
        Some(slot) => stake.lots[slot] = StakeLot { amount, staked_at: settled_at },
        None => {
            let newest = stake
                .lots
                .iter_mut()
                .filter(|lot| lot.amount > 0)
                .max_by_key(|lot| lot.staked_at)
                .ok_or(ErrorCode::TooManyLots)?;
            newest.amount = newest.amount.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
        }
    }
    stake.amount = stake.amount.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    pool.total_staked = pool.total_staked.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    save_account(&stake_info, &stake)?;
    save_account(&pool_info, &pool)?;

    mint_munda(ctx, amount)
}

/// Mints `amount` MUNDA to the staked MUNDA vault, signed by the MUNDA mint authority (D47).
fn mint_munda(ctx: &Context<RedeemClaim>, amount: u64) -> Result<()> {
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_MINT_TO);
    data.extend_from_slice(&amount.to_le_bytes());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.munda_mint.key(), false),
            AccountMeta::new(ctx.accounts.stake_vault.key(), false),
            AccountMeta::new_readonly(ctx.accounts.munda_mint_authority.key(), true),
        ],
        data,
    };
    let seeds: &[&[u8]] = &[MUNDA_MINT_AUTHORITY_SEED, &[ctx.bumps.munda_mint_authority]];
    invoke_signed(
        &ix,
        &[
            ctx.accounts.munda_mint.to_account_info(),
            ctx.accounts.stake_vault.to_account_info(),
            ctx.accounts.munda_mint_authority.to_account_info(),
        ],
        &[seeds],
    )?;
    Ok(())
}

/// Wait: after the unlock time, pay the claim in full. A win pays owed MUSD
/// from the pool; a loss mints its MUNDA.
pub fn handle_redeem_wait(ctx: Context<RedeemClaim>) -> Result<()> {
    check_token_program(&ctx)?;
    let claim = &ctx.accounts.claim;
    require!(!claim.redeemed, ErrorCode::ClaimAlreadyRedeemed);
    require!(!claim.queued, ErrorCode::ClaimAlreadyQueued);
    require!(Clock::get()?.unix_timestamp >= claim.unlock_at, ErrorCode::ClaimLocked);
    let amount = claim.amount;
    // A loss's stake lot starts its clock at the settle time (D29): unlock_at less the 24 hours.
    let settled_at = claim.unlock_at - LOSS_UNLOCK_SECS;
    match claim.asset {
        ClaimAsset::Musd => {
            check_player_musd_ata(&ctx)?;
            {
                // A win waits in the queue if the pool cannot pay it now, or if
                // anyone is already waiting: nobody is paid ahead of the queue (D30).
                let mut queue = ctx.accounts.queue.load_mut()?;
                if queue.tail > queue.head || token_balance(&ctx.accounts.pool_vault)? < amount {
                    enqueue(&mut queue, ctx.accounts.claim.key(), amount)?;
                    ctx.accounts.claim.queued = true;
                    msg!("Wait: {} base MUSD queued behind the pool", amount);
                    return Ok(());
                }
            }
            ctx.accounts.claim.redeemed = true;
            let seeds: &[&[u8]] = &[POOL_SEED, &[ctx.accounts.pool.bump]];
            spl_transfer(
                &ctx.accounts.pool_vault,
                &ctx.accounts.player_ata,
                &ctx.accounts.pool.to_account_info(),
                amount,
                &[seeds],
            )?;
            msg!("Wait: paid {} base MUSD from the pool", amount);
        }
        ClaimAsset::Munda => {
            ctx.accounts.claim.redeemed = true;
            credit_loss_munda(&ctx, amount, settled_at)?;
            msg!("Wait: {} base MUNDA staked", amount);
        }
    }
    Ok(())
}

/// Take now: a win pays 95% at once from the insurance fund, which the pool
/// repays in full later; a loss mints 90% of its MUNDA and never mints the 10%.
pub fn handle_redeem_take_now(ctx: Context<RedeemClaim>) -> Result<()> {
    check_token_program(&ctx)?;
    let claim = &ctx.accounts.claim;
    require!(!claim.redeemed, ErrorCode::ClaimAlreadyRedeemed);
    require!(!claim.queued, ErrorCode::ClaimAlreadyQueued);
    let amount = claim.amount;
    let settled_at = claim.unlock_at - LOSS_UNLOCK_SECS;
    let payout = take_now_payout(amount, claim.early_exit_bps);
    match claim.asset {
        ClaimAsset::Musd => {
            check_player_musd_ata(&ctx)?;
            require!(
                token_balance(&ctx.accounts.insurance_vault)? >= payout,
                ErrorCode::InsufficientInsuranceFunds
            );
            ctx.accounts.claim.redeemed = true;
            ctx.accounts.claim.insurance_owed = amount;
            let seeds: &[&[u8]] = &[INSURANCE_SEED, &[ctx.bumps.insurance_authority]];
            spl_transfer(
                &ctx.accounts.insurance_vault,
                &ctx.accounts.player_ata,
                &ctx.accounts.insurance_authority.to_account_info(),
                payout,
                &[seeds],
            )?;
            msg!("Take now: paid {} base MUSD, insurance is owed {}", payout, amount);
        }
        ClaimAsset::Munda => {
            ctx.accounts.claim.redeemed = true;
            credit_loss_munda(&ctx, payout, settled_at)?;
            msg!("Take now: {} base MUNDA staked; the 10% tax is never minted", payout);
        }
    }
    Ok(())
}

#[derive(Accounts)]
pub struct CollectInsurance<'info> {
    /// CHECK: the bet the claim settles; only its key is used.
    pub bet: UncheckedAccount<'info>,

    /// The claim whose "Take now" the insurance fund advanced.
    #[account(mut, seeds = [CLAIM_SEED, bet.key().as_ref()], bump = claim.bump)]
    pub claim: Account<'info, Claim>,

    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool MUSD vault, seeds pin it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump)]
    pub pool_vault: UncheckedAccount<'info>,

    /// CHECK: the insurance vault, seeds pin it.
    #[account(mut, seeds = [INSURANCE_VAULT_SEED], bump)]
    pub insurance_vault: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,
}

/// After the claim unlocks, the pool repays the insurance fund the full 100% it
/// advanced. Anyone can call it; the insurance fund keeps the 5% it took.
pub fn handle_collect_insurance(ctx: Context<CollectInsurance>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    let claim = &ctx.accounts.claim;
    let owed = claim.insurance_owed;
    require!(owed > 0, ErrorCode::NothingToCollect);
    require!(Clock::get()?.unix_timestamp >= claim.unlock_at, ErrorCode::ClaimLocked);
    require!(token_balance(&ctx.accounts.pool_vault)? >= owed, ErrorCode::InsufficientPoolFunds);

    ctx.accounts.claim.insurance_owed = 0;
    let seeds: &[&[u8]] = &[POOL_SEED, &[ctx.accounts.pool.bump]];
    spl_transfer(
        &ctx.accounts.pool_vault,
        &ctx.accounts.insurance_vault,
        &ctx.accounts.pool.to_account_info(),
        owed,
        &[seeds],
    )?;
    msg!("Insurance collected {} base MUSD from the pool", owed);
    Ok(())
}
