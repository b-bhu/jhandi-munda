//! The pool deposit instruction (D4, D27): the depositor sends MUSD into the
//! house pool's vault and is minted pool shares at the current NAV. The empty
//! pool (D4) defines NAV as $1.00, so the first deposit mints shares 1:1 with
//! MUSD: 100 MUSD deposited = 100 shares minted, vault holds 100 MUSD. NAV is
//! read as an integer, `$1.00 == 1_000_000` (6 decimals). Zero-amount
//! deposits are rejected. Withdraw and post-profit pricing are later steps.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
};

use crate::{
    error::ErrorCode, state::PoolState, ATA_PROGRAM_ID, POOL_SEED, SHARE_MINT_SEED,
    SPL_TOKEN_PROGRAM_ID,
};

/// SPL token `Transfer`: data [3][amount u64 LE]. Writable source and
/// destination plus the authority as a readonly signer. The depositor signs
/// as a transaction signer, and `invoke` keeps that signature alive in the
/// CPI. Hand built like the faucet's `mint_to` so the pinned dependency set
/// stays untouched.
const TOKEN_IX_TRANSFER: u8 = 3;

/// SPL token `MintTo`: data [7][amount u64 LE]. Signed by the pool PDA, the
/// share mint's authority, so only pool instructions can mint shares (D27).
const TOKEN_IX_MINT_TO: u8 = 7;

/// ATA program `CreateIdempotent`: data [1] — same as the faucet's ATA CPI.
/// Creates the depositor's share token account only when it is missing.
const ATA_IX_CREATE_IDEMPOTENT: u8 = 1;

// SPL account layouts, the same offsets the LiteSVM tests read: a token
// account stores its balance in 64..72, a mint stores supply in 36..44.
// Deposits price off these two numbers — the vault balance and the share
// supply — because PoolState stores no totals. The token accounts are the
// source of truth.
const TOKEN_AMOUNT_OFFSET: usize = 64;
const MINT_SUPPLY_OFFSET: usize = 36;

#[derive(Accounts)]
pub struct Deposit<'info> {
    /// The house pool (D4, D27). Read only: a deposit changes token
    /// accounts, not `PoolState` — the vault balance and the share supply
    /// are the totals everyone reads.
    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool share (LP) mint from `initialize_pool`. The seeds pin
    /// its address, `owner` keeps it a real SPL mint, and the `mint_to` CPI
    /// below raises its supply.
    #[account(mut, seeds = [SHARE_MINT_SEED], bump, owner = SPL_TOKEN_PROGRAM_ID)]
    pub share_mint: UncheckedAccount<'info>,

    /// CHECK: the MUSD vault from `initialize_pool`, owned by the pool PDA.
    /// The seeds pin its address; the transfer CPI below credits it. Its mint
    /// is checked by the token program itself: the transfer only lands if the
    /// vault holds exactly the MUSD the source account carries.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump, owner = SPL_TOKEN_PROGRAM_ID)]
    pub vault: UncheckedAccount<'info>,

    /// The depositor: signs the MUSD transfer into the vault and pays the
    /// rent for their share token account on the first deposit.
    #[account(mut)]
    pub depositor: Signer<'info>,

    /// CHECK: must be the depositor's canonical MUSD ATA (checked in the
    /// handler against `pool.musd_mint`, the same derivation the ATA program
    /// uses), and must already hold the MUSD being deposited.
    #[account(mut)]
    pub depositor_ata: UncheckedAccount<'info>,

    /// CHECK: must be the depositor's canonical share (LP) ATA (checked in
    /// the handler) and is created there if missing, exactly like the
    /// faucet's MUSD ATA.
    #[account(mut)]
    pub depositor_shares: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler before
    /// the CPIs.
    pub token_program: UncheckedAccount<'info>,

    /// CHECK: the system program, needed in the account list by the ATA
    /// program's own `create_account` CPI.
    pub system_program: Program<'info, System>,

    /// CHECK: the associated token account program, invoked by the
    /// `create_idempotent` CPI below.
    pub associated_token_program: UncheckedAccount<'info>,
}

/// The canonical pool-share associated token account for a wallet. Kept here
/// so the program and the tests derive it in exactly one place, like
/// `musd_ata` in faucet.rs.
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

/// Reads the SPL token account balance (bytes 64..72).
fn token_amount(info: &AccountInfo) -> Result<u64> {
    let data = info
        .try_borrow_data()
        .map_err(|_| ErrorCode::InvalidPoolState)?;
    let bytes = data
        .get(TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8)
        .ok_or(ErrorCode::InvalidPoolState)?;
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| ErrorCode::InvalidPoolState)?))
}

/// Reads the SPL mint supply (bytes 36..44).
fn mint_supply(info: &AccountInfo) -> Result<u64> {
    let data = info
        .try_borrow_data()
        .map_err(|_| ErrorCode::InvalidPoolState)?;
    let bytes = data
        .get(MINT_SUPPLY_OFFSET..MINT_SUPPLY_OFFSET + 8)
        .ok_or(ErrorCode::InvalidPoolState)?;
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| ErrorCode::InvalidPoolState)?))
}

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let depositor = ctx.accounts.depositor.key();
    let share_mint = ctx.accounts.share_mint.key();

    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );

    // A zero-amount deposit is rejected outright (it would also mint zero
    // shares below, but the caller should fail on the amount, not on NAV).
    require!(amount > 0, ErrorCode::ZeroDeposit);

    // MUSD can only come from the depositor's canonical MUSD ATA (same check
    // as the faucet): owner and mint are pinned by that derivation, so the
    // transfer cannot be fed a look-alike account.
    require_keys_eq!(
        ctx.accounts.depositor_ata.key(),
        crate::musd_ata(&depositor, &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );

    // Shares can only land in the depositor's canonical share ATA.
    require_keys_eq!(
        ctx.accounts.depositor_shares.key(),
        share_ata(&depositor, &share_mint),
        ErrorCode::InvalidAtaAddress
    );

    // Price the deposit BEFORE any token moves: the share supply and the
    // vault balance as they stand are this depositor's entry price (D27,
    // shares minted at the current NAV). `(0, 0)` is the empty pool (D4)
    // whose NAV is *defined* as $1.00, so shares mint 1:1 with MUSD. Any
    // other vault/supply pair prices pro rata: shares = amount × supply /
    // vault — the same `amount × 1_000_000 / NAV` as `nav_per_share`, with
    // no extra rounding in between — floored in the pool's favour, like
    // every standard vault. Any other combination (shares without vaulted
    // MUSD or the reverse) cannot be produced by this instruction and would
    // corrupt the NAV, so it is rejected instead of divided by zero.
    let vault_amount = token_amount(&ctx.accounts.vault.to_account_info())?;
    let share_supply = mint_supply(&ctx.accounts.share_mint.to_account_info())?;
    let minted: u128 = match (vault_amount, share_supply) {
        (0, 0) => amount as u128,
        (vault, supply) => {
            require!(vault > 0 && supply > 0, ErrorCode::InvalidPoolState);
            amount as u128 * supply as u128 / vault as u128
        }
    };
    // A deposit that floors to zero shares would pay MUSD and get nothing
    // back — reject it like a zero-amount deposit.
    require!(minted > 0, ErrorCode::ZeroShareMint);
    let minted = u64::try_from(minted).map_err(|_| ErrorCode::ShareMintOverflow)?;

    // Create the depositor's share token account if the wallet does not have
    // one yet (the depositor pays the rent). Repeating a deposit is safe:
    // `create_idempotent` passes an existing correct account untouched.
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

    // Pull the MUSD into the vault. Signed by the depositor as a transaction
    // signer; the token program checks that the source is a real MUSD token
    // account owned by the depositor and has the balance.
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

    // Mint the shares at the priced rate, signed by the pool PDA — the share
    // mint's authority from `initialize_pool`.
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
