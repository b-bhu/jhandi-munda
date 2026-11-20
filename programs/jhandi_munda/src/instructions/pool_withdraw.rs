//! The pool withdraw instruction (D4, D27): the holder burns pool shares and
//! receives MUSD from the vault at the current NAV: `shares × vault / supply`,
//! floored in the pool's favour. The payout is 1:1 only while the pool itself
//! is 1:1 (40 shares back = 40 MUSD); MUSD transferred straight into the
//! vault (Step 3.4's mocked pool profit) reprices every withdraw above 1:1.
//! Zero and over-balance withdraws are rejected.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
};

use crate::{
    error::ErrorCode, state::{PayoutQueue, PoolState}, POOL_SEED, POOL_VAULT_SEED, QUEUE_SEED,
    SHARE_MINT_SEED, SPL_TOKEN_PROGRAM_ID,
};

/// SPL token `Burn` (8) and `Transfer` (3) discriminators, hand built like the
/// deposit's CPIs so the pinned dependency set stays untouched. Burn data ends
/// with the amount u64, transfer likewise; the holder signs the burn of their
/// own shares and the pool PDA signs the vault payout.
const TOKEN_IX_BURN: u8 = 8;
const TOKEN_IX_TRANSFER: u8 = 3;

/// Offsets inside SPL token accounts: a token account stores its balance in
/// 64..72, a mint its supply in 36..44. Withdraws price off these two
/// numbers; the token accounts are the source of truth.
const TOKEN_AMOUNT_OFFSET: usize = 64;
const MINT_SUPPLY_OFFSET: usize = 36;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// The house pool (D4, D27). Read only: a withdraw changes token
    /// accounts, not `PoolState`.
    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool share (LP) mint from `initialize_pool`; the seeds pin
    /// its address, `owner` keeps it a real SPL mint, and the `burn` CPI
    /// below lowers its supply.
    #[account(mut, seeds = [SHARE_MINT_SEED], bump, owner = SPL_TOKEN_PROGRAM_ID)]
    pub share_mint: UncheckedAccount<'info>,

    /// CHECK: the MUSD vault from `initialize_pool`, owned by the pool PDA;
    /// the seeds pin its address and the transfer CPI below debits it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump, owner = SPL_TOKEN_PROGRAM_ID)]
    pub vault: UncheckedAccount<'info>,

    /// The withdrawer: signs the burn of their own shares.
    #[account(mut)]
    pub withdrawer: Signer<'info>,

    /// CHECK: the withdrawer's canonical share ATA (checked in the handler
    /// against the seed-pinned share mint) and must hold the burnt shares.
    #[account(mut)]
    pub withdrawer_shares: UncheckedAccount<'info>,

    /// CHECK: the withdrawer's canonical MUSD ATA (checked in the handler
    /// against `pool.musd_mint`) and must already exist.
    #[account(mut)]
    pub withdrawer_ata: UncheckedAccount<'info>,

    /// The payout queue (Step 8). Read only: LPs cannot withdraw while winners
    /// are queued for payment, so they cannot jump the line (D30).
    #[account(seeds = [QUEUE_SEED], bump)]
    pub queue: AccountLoader<'info, PayoutQueue>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,
}

/// Reads an SPL token account balance (64..72) or mint supply (36..44).
fn token_u64(info: &AccountInfo, offset: usize) -> Result<u64> {
    let data = info
        .try_borrow_data()
        .map_err(|_| ErrorCode::InvalidPoolState)?;
    let bytes = data.get(offset..offset + 8).ok_or(ErrorCode::InvalidPoolState)?;
    Ok(u64::from_le_bytes(
        bytes.try_into().map_err(|_| ErrorCode::InvalidPoolState)?,
    ))
}

/// The withdrawer's share balance, with a share account that does not exist
/// yet reading as zero shares so the over-withdraw check below rejects before
/// any CPI runs.
fn held_shares(info: &AccountInfo) -> Result<u64> {
    let data = info
        .try_borrow_data()
        .map_err(|_| ErrorCode::InvalidPoolState)?;
    if data.len() < TOKEN_AMOUNT_OFFSET + 8 {
        return Ok(0);
    }
    let bytes = data[TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8]
        .try_into()
        .map_err(|_| ErrorCode::InvalidPoolState)?;
    Ok(u64::from_le_bytes(bytes))
}

pub fn handler(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let withdrawer = ctx.accounts.withdrawer.key();
    let share_mint = ctx.accounts.share_mint.key();

    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    // Winners queued for payment come before any LP withdraw (D30).
    {
        let queue = ctx.accounts.queue.load()?;
        require!(queue.head == queue.tail, ErrorCode::QueueNotEmpty);
    }
    // Zero-amount withdraws are rejected, like zero-amount deposits.
    require!(amount > 0, ErrorCode::ZeroWithdraw);
    // Shares only burn from the canonical share ATA; MUSD only lands in the
    // canonical MUSD ATA — both checked like the deposit's destinations.
    require_keys_eq!(
        ctx.accounts.withdrawer_shares.key(),
        crate::share_ata(&withdrawer, &share_mint),
        ErrorCode::InvalidAtaAddress
    );
    require_keys_eq!(
        ctx.accounts.withdrawer_ata.key(),
        crate::musd_ata(&withdrawer, &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );
    // A withdraw of more shares than the holder owns is rejected before
    // anything moves (issue Done-when).
    let held = held_shares(&ctx.accounts.withdrawer_shares.to_account_info())?;
    require!(amount <= held, ErrorCode::InsufficientShares);

    // Price before any token moves (D27): `amount × vault / supply`, floored
    // in the pool's favour. The supply cannot be zero here — the balance
    // check above proves the holder owns real shares — but an odd state is
    // guarded anyway. The payout is 1:1 only while vault == supply; any extra
    // vaulted MUSD (Step 3.4's mocked profit, later a real one) prices it
    // above 1:1.
    let vault_amount = token_u64(&ctx.accounts.vault.to_account_info(), TOKEN_AMOUNT_OFFSET)?;
    let share_supply = token_u64(
        &ctx.accounts.share_mint.to_account_info(),
        MINT_SUPPLY_OFFSET,
    )?;
    require!(share_supply > 0, ErrorCode::InvalidPoolState);
    let paid: u128 = amount as u128 * vault_amount as u128 / share_supply as u128;
    // A withdraw flooring to zero MUSD burns shares for nothing: reject it.
    require!(paid > 0, ErrorCode::ZeroWithdraw);
    let paid = u64::try_from(paid).map_err(|_| ErrorCode::InvalidPoolState)?;

    // Burn the shares; the withdrawer's transaction signature carries through
    // `invoke`, and the token program checks balance, mint and authority.
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_BURN);
    data.extend_from_slice(&amount.to_le_bytes());
    let burn = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.withdrawer_shares.key(), false),
            AccountMeta::new(share_mint, false),
            AccountMeta::new_readonly(withdrawer, true),
        ],
        data,
    };
    invoke(
        &burn,
        &[
            ctx.accounts.withdrawer_shares.to_account_info(),
            ctx.accounts.share_mint.to_account_info(),
            ctx.accounts.withdrawer.to_account_info(),
        ],
    )?;

    // Pay out the priced MUSD from the vault, signed by the pool PDA — the
    // vault's token authority.
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_TRANSFER);
    data.extend_from_slice(&paid.to_le_bytes());
    let transfer = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.vault.key(), false),
            AccountMeta::new(ctx.accounts.withdrawer_ata.key(), false),
            AccountMeta::new_readonly(ctx.accounts.pool.key(), true),
        ],
        data,
    };
    invoke_signed(
        &transfer,
        &[
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.withdrawer_ata.to_account_info(),
            ctx.accounts.pool.to_account_info(),
        ],
        &[&[POOL_SEED, &[ctx.bumps.pool]]],
    )?;

    msg!("Withdraw of {} shares paid {} base MUSD", amount, paid);
    Ok(())
}
