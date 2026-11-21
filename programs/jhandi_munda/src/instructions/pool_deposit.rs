//! The pool deposit instruction (D4, D27): the depositor sends MUSD into the
//! vault and is minted shares at the current NAV. The empty pool defines NAV
//! as $1.00, so the first deposit mints 1:1: 100 MUSD in = 100 shares, vault
//! holds 100 MUSD. Zero-amount deposits are rejected.
//!
//! D43: a pool with no shares but MUSD in its vault (orphan money, e.g. a lost
//! stake that landed after the last LP withdrew) is swept to the insurance vault
//! first, then the new deposit mints 1:1. No first depositor can take the
//! leftover house money.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
};

use crate::{
    error::ErrorCode, state::PoolState, ATA_PROGRAM_ID, INSURANCE_VAULT_SEED, POOL_SEED, POOL_VAULT_SEED,
    SHARE_MINT_SEED, SPL_TOKEN_PROGRAM_ID,
};

/// SPL token `Transfer` (3), `MintTo` (7) and ATA `CreateIdempotent` (1)
/// discriminators, hand built like the faucet's CPIs so the pinned
/// dependency set stays untouched. Transfer data ends with the amount u64,
/// mint-to likewise; the pool PDA signs the mint.
const TOKEN_IX_TRANSFER: u8 = 3;
const TOKEN_IX_MINT_TO: u8 = 7;
const ATA_IX_CREATE_IDEMPOTENT: u8 = 1;

/// Offsets inside SPL token accounts: a token account stores its balance in
/// 64..72, a mint its supply in 36..44. Deposits price off these two
/// numbers; the token accounts are the source of truth.
const TOKEN_AMOUNT_OFFSET: usize = 64;
const MINT_SUPPLY_OFFSET: usize = 36;

#[derive(Accounts)]
pub struct Deposit<'info> {
    /// The house pool (D4, D27). Read only: a deposit changes token
    /// accounts, not `PoolState`.
    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool share (LP) mint from `initialize_pool`; the seeds pin
    /// its address, `owner` keeps it a real SPL mint, and the `mint_to` CPI
    /// below raises its supply.
    #[account(mut, seeds = [SHARE_MINT_SEED], bump, owner = SPL_TOKEN_PROGRAM_ID)]
    pub share_mint: UncheckedAccount<'info>,

    /// CHECK: the MUSD vault from `initialize_pool`, owned by the pool PDA;
    /// the seeds pin its address and the transfer CPI below credits it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump, owner = SPL_TOKEN_PROGRAM_ID)]
    pub vault: UncheckedAccount<'info>,

    /// The depositor: signs the MUSD transfer and pays the share ATA rent.
    #[account(mut)]
    pub depositor: Signer<'info>,

    /// CHECK: the depositor's canonical MUSD ATA (checked in the handler
    /// against `pool.musd_mint`) and must already hold the MUSD deposited.
    #[account(mut)]
    pub depositor_ata: UncheckedAccount<'info>,

    /// CHECK: the depositor's canonical share ATA (checked in the handler)
    /// and created there if missing.
    #[account(mut)]
    pub depositor_shares: UncheckedAccount<'info>,

    /// CHECK: the insurance vault (D18, D43). Receives orphan MUSD when the pool has no
    /// shares. Its seeds pin the address; it is only touched in that case.
    #[account(mut, seeds = [INSURANCE_VAULT_SEED], bump)]
    pub insurance_vault: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: invoked by the `create_idempotent` CPI below.
    pub associated_token_program: UncheckedAccount<'info>,
}

/// Canonical pool-share ATA for a wallet, like `musd_ata` in faucet.rs.
pub fn share_ata(wallet: &Pubkey, share_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            wallet.as_ref(),
            SPL_TOKEN_PROGRAM_ID.as_ref(),
            share_mint.as_ref(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0
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

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let depositor = ctx.accounts.depositor.key();
    let share_mint = ctx.accounts.share_mint.key();

    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    // Zero-amount deposits are rejected (issue Done-when).
    require!(amount > 0, ErrorCode::ZeroDeposit);
    // MUSD may only leave the canonical MUSD ATA, shares may only land in
    // the canonical share ATA — both checked like the faucet's destination.
    require_keys_eq!(
        ctx.accounts.depositor_ata.key(),
        crate::musd_ata(&depositor, &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );
    require_keys_eq!(
        ctx.accounts.depositor_shares.key(),
        share_ata(&depositor, &share_mint),
        ErrorCode::InvalidAtaAddress
    );

    // Price before any token moves (D27). With no shares the pool's NAV is $1.00 by
    // definition, so the deposit mints 1:1. Any MUSD still in the vault then is orphan
    // money (D43): it is swept to insurance below, before the deposit lands. With shares
    // outstanding, price pro rata (`amount × supply / vault`), floored in the pool's favour.
    // Shares outstanding with an empty vault cannot be produced here and is refused.
    let vault_amount = token_u64(&ctx.accounts.vault.to_account_info(), TOKEN_AMOUNT_OFFSET)?;
    let share_supply = token_u64(
        &ctx.accounts.share_mint.to_account_info(),
        MINT_SUPPLY_OFFSET,
    )?;
    let minted: u128 = if share_supply == 0 {
        amount as u128
    } else {
        require!(vault_amount > 0, ErrorCode::InvalidPoolState);
        amount as u128 * share_supply as u128 / vault_amount as u128
    };
    // A deposit flooring to zero shares pays MUSD for nothing: reject it.
    require!(minted > 0, ErrorCode::ZeroDeposit);
    let minted = u64::try_from(minted).map_err(|_| ErrorCode::InvalidPoolState)?;

    // Create the depositor's share ATA if missing (depositor pays rent);
    // idempotent, so repeat deposits pass it untouched.
    let create_shares_ata = Instruction {
        program_id: ATA_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(depositor, true),
            AccountMeta::new(ctx.accounts.depositor_shares.key(), false),
            AccountMeta::new_readonly(depositor, false),
            AccountMeta::new_readonly(share_mint, false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
            AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
        ],
        data: vec![ATA_IX_CREATE_IDEMPOTENT],
    };
    invoke(
        &create_shares_ata,
        &[
            ctx.accounts.depositor.to_account_info(),
            ctx.accounts.depositor_shares.to_account_info(),
            ctx.accounts.depositor.to_account_info(),
            ctx.accounts.share_mint.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
    )?;

    // D43: no shares, but MUSD in the vault: sweep it to insurance before the new deposit lands,
    // so the new LP owns exactly what they put in. UNVERIFIED on devnet: the insurance vault exists there.
    if share_supply == 0 && vault_amount > 0 {
        require_keys_eq!(
            *ctx.accounts.insurance_vault.owner,
            SPL_TOKEN_PROGRAM_ID,
            ErrorCode::InvalidPoolState
        );
        let mut data = Vec::with_capacity(9);
        data.push(TOKEN_IX_TRANSFER);
        data.extend_from_slice(&vault_amount.to_le_bytes());
        let sweep = Instruction {
            program_id: SPL_TOKEN_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(ctx.accounts.vault.key(), false),
                AccountMeta::new(ctx.accounts.insurance_vault.key(), false),
                AccountMeta::new_readonly(ctx.accounts.pool.key(), true),
            ],
            data,
        };
        invoke_signed(
            &sweep,
            &[
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.insurance_vault.to_account_info(),
                ctx.accounts.pool.to_account_info(),
            ],
            &[&[POOL_SEED, &[ctx.bumps.pool]]],
        )?;
        msg!("D43: {} orphan base MUSD moved from the pool vault to insurance", vault_amount);
    }

    // Pull the MUSD into the vault; the depositor's transaction signature
    // carries through `invoke`, and the token program checks balance/mint.
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_TRANSFER);
    data.extend_from_slice(&amount.to_le_bytes());
    let transfer = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.depositor_ata.key(), false),
            AccountMeta::new(ctx.accounts.vault.key(), false),
            AccountMeta::new_readonly(depositor, true),
        ],
        data,
    };
    invoke(
        &transfer,
        &[
            ctx.accounts.depositor_ata.to_account_info(),
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.depositor.to_account_info(),
        ],
    )?;

    // Mint the priced shares, signed by the pool PDA — the share mint's
    // authority.
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_MINT_TO);
    data.extend_from_slice(&minted.to_le_bytes());
    let mint_to = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(share_mint, false),
            AccountMeta::new(ctx.accounts.depositor_shares.key(), false),
            AccountMeta::new_readonly(ctx.accounts.pool.key(), true),
        ],
        data,
    };
    invoke_signed(
        &mint_to,
        &[
            ctx.accounts.share_mint.to_account_info(),
            ctx.accounts.depositor_shares.to_account_info(),
            ctx.accounts.pool.to_account_info(),
        ],
        &[&[POOL_SEED, &[ctx.bumps.pool]]],
    )?;

    msg!("Deposit of {} base MUSD minted {} shares", amount, minted);
    Ok(())
}
