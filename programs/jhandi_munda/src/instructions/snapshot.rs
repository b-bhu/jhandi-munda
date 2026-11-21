//! Daily snapshot of the house (#50, D28). Anyone can take today's snapshot: each one stores NAV per
//! share, the pool size, the MUNDA staked, the open claims and the payout queue size. A day is a UTC
//! day (Unix time / 86400, so the boundary is UTC midnight). One snapshot per day: the PDA is keyed by
//! the day, so a second crank for the same day fails at `init`.
//!
//! LIMITATION (UNVERIFIED follow-up): no on-chain counter of unredeemed claims exists, and adding one
//! would change `PoolState`'s layout (a migration). So `open_claims` counts the claims waiting in the
//! payout queue, not every unredeemed claim.

use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    state::{PayoutQueue, PoolState, StakePool},
    POOL_SEED, POOL_VAULT_SEED, QUEUE_SEED, SHARE_MINT_SEED, SNAPSHOT_SEED, SPL_TOKEN_PROGRAM_ID,
    STAKE_POOL_SEED,
};

/// One UTC day in seconds.
pub const SECS_PER_DAY: i64 = 86_400;
/// NAV per share is in MUSD base units (6 decimals): $1.00 when the pool has no shares (D4).
pub const NAV_ONE: u64 = 1_000_000;

/// The house on one UTC day (D28).
#[account]
#[derive(InitSpace)]
pub struct DailySnapshot {
    /// The UTC day this snapshot is for: Unix time / 86400.
    pub day: u64,
    /// When it was taken (Unix time).
    pub taken_at: i64,
    /// MUSD per LP share, in base units (vault / share supply; NAV_ONE with no shares).
    pub nav_per_share: u64,
    /// MUSD in the pool vault, in base units.
    pub pool_size: u64,
    /// MUNDA staked across all wallets, in base units.
    pub munda_staked: u64,
    /// Claims waiting in the payout queue (see the module note).
    pub open_claims: u32,
    /// Payout queue length.
    pub queue_size: u32,
    /// Bump seed for the snapshot PDA.
    pub bump: u8,
}

/// Reads a u64 at `offset` in an SPL account (token balance at 64, mint supply at 36).
fn spl_u64(info: &AccountInfo, offset: usize) -> Result<u64> {
    let data = info.try_borrow_data()?;
    let bytes = data.get(offset..offset + 8).ok_or(ErrorCode::InvalidPoolState)?;
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| ErrorCode::InvalidPoolState)?))
}

#[derive(Accounts)]
#[instruction(day: u64)]
pub struct TakeSnapshot<'info> {
    /// Anyone may pay for the snapshot.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Today's snapshot, created here. Its seeds include the day, so a second one for the day fails.
    #[account(
        init,
        payer = payer,
        space = 8 + DailySnapshot::INIT_SPACE,
        seeds = [SNAPSHOT_SEED, day.to_le_bytes().as_ref()],
        bump
    )]
    pub snapshot: Account<'info, DailySnapshot>,

    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the LP share mint, seeds pin it; its supply is read.
    #[account(seeds = [SHARE_MINT_SEED], bump)]
    pub share_mint: UncheckedAccount<'info>,

    /// CHECK: the pool MUSD vault, seeds pin it; its balance is read.
    #[account(seeds = [POOL_VAULT_SEED], bump)]
    pub vault: UncheckedAccount<'info>,

    #[account(seeds = [STAKE_POOL_SEED], bump)]
    pub stake_pool: Account<'info, StakePool>,

    #[account(seeds = [QUEUE_SEED], bump)]
    pub queue: AccountLoader<'info, PayoutQueue>,

    pub system_program: Program<'info, System>,
}

pub fn handle_take_snapshot(ctx: Context<TakeSnapshot>, day: u64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    // Only the current UTC day: the snapshot is what the house looks like now.
    require!(now.div_euclid(SECS_PER_DAY) as u64 == day, ErrorCode::SnapshotNotToday);
    require_keys_eq!(ctx.accounts.vault.owner.clone(), SPL_TOKEN_PROGRAM_ID, ErrorCode::InvalidPoolState);

    let pool_size = spl_u64(&ctx.accounts.vault.to_account_info(), 64)?;
    let supply = spl_u64(&ctx.accounts.share_mint.to_account_info(), 36)?;
    // NAV: vault / supply, floored in the pool's favour. With no shares the NAV is $1.00 (D4).
    let nav_per_share = if supply == 0 {
        NAV_ONE
    } else {
        ((pool_size as u128 * NAV_ONE as u128) / supply as u128) as u64
    };
    let queue = ctx.accounts.queue.load()?;
    let queue_size = (queue.tail - queue.head) as u32;

    let snapshot = &mut ctx.accounts.snapshot;
    snapshot.day = day;
    snapshot.taken_at = now;
    snapshot.nav_per_share = nav_per_share;
    snapshot.pool_size = pool_size;
    snapshot.munda_staked = ctx.accounts.stake_pool.total_staked;
    snapshot.open_claims = queue_size;
    snapshot.queue_size = queue_size;
    snapshot.bump = ctx.bumps.snapshot;
    msg!("Snapshot day {}: NAV {} per share, pool {}, staked {}, queue {}", day, nav_per_share, pool_size,
        snapshot.munda_staked, queue_size);
    Ok(())
}
