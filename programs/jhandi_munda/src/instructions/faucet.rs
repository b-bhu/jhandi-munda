//! The MUSD faucet: one instruction that mints a fixed amount of test MUSD
//! to the caller's associated token account (D21, D26). Any wallet can call
//! it, at most once per hour (D33): the wallet's faucet-record PDA stores
//! its last claim, and repeat calls inside the hour fail with
//! `FaucetCooldown`. Devnet demo scaffolding only, never a mainnet faucet.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
};

use crate::{
    error::ErrorCode, state::{FaucetRecord, ProgramConfig}, ATA_PROGRAM_ID, CONFIG_SEED,
    FAUCET_AMOUNT, FAUCET_COOLDOWN_SECS, FAUCET_RECORD_SEED, MUSD_MINT_AUTHORITY_SEED,
    MUSD_MINT_SEED, SPL_TOKEN_PROGRAM_ID,
};

/// SPL token `MintTo`: data [7][amount u64 LE]. Writable mint and
/// destination plus the mint authority as a readonly signer.
const TOKEN_IX_MINT_TO: u8 = 7;

/// ATA program `CreateIdempotent`: data [1]. Accounts: funding (signer),
/// ATA (writable), wallet, mint, system program, SPL token program. Creates
/// the ATA only when it is missing; passes untouched when a correct one
/// already exists.
const ATA_IX_CREATE_IDEMPOTENT: u8 = 1;

#[derive(Accounts)]
pub struct MusdFaucet<'info> {
    /// Program config from `initialize`. Proves the MUSD mint is set up and
    /// binds the mint to the address stored in the config.
    #[account(seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, ProgramConfig>,

    /// CHECK: the MUSD mint. The seeds pin its address, `owner = SPL_TOKEN_PROGRAM_ID`
    /// keeps it a real mint of the SPL token program, and the constraint below ties
    /// it to `config.musd_mint`; the `mint_to` CPI below only raises its supply.
    #[account(
        mut,
        seeds = [MUSD_MINT_SEED],
        bump,
        owner = SPL_TOKEN_PROGRAM_ID,
        constraint = config.musd_mint == musd_mint.key()
            @ ErrorCode::MusdMintMismatch
    )]
    pub musd_mint: UncheckedAccount<'info>,

    /// CHECK: the mint authority PDA. Signed for the `mint_to` CPI with
    /// invoke_signed below; no keypair for it exists anywhere.
    #[account(seeds = [MUSD_MINT_AUTHORITY_SEED], bump)]
    pub musd_mint_authority: UncheckedAccount<'info>,

    /// The caller: pays the ATA rent when one has to be created, pays the
    /// faucet-record rent on the wallet's first call, and receives the
    /// minted MUSD.
    #[account(mut)]
    pub wallet: Signer<'info>,

    /// The wallet's faucet cooldown record (D33). Created by this same call
    /// when it is missing, then keeps the wallet's last claim timestamp. The
    /// seeds bind the PDA to the caller, so a wallet can only ever touch its
    /// own record.
    #[account(
        init_if_needed,
        payer = wallet,
        space = 8 + FaucetRecord::INIT_SPACE,
        seeds = [FAUCET_RECORD_SEED, wallet.key().as_ref()],
        bump
    )]
    pub faucet_record: Account<'info, FaucetRecord>,

    /// CHECK: must be the wallet's canonical MUSD ATA (checked in the
    /// handler) and is created there if missing.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,

    /// CHECK: the associated token account program, invoked by the
    /// `create_idempotent` CPI below.
    pub associated_token_program: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler before
    /// the CPIs.
    pub token_program: UncheckedAccount<'info>,

    /// CHECK: the system program, needed in the account list by the ATA
    /// program's own `create_account` CPI.
    pub system_program: Program<'info, System>,
}

/// Canonical MUSD associated token account for a wallet. Kept here so the
/// program and the tests derive it in exactly one place.
pub fn musd_ata(wallet: &Pubkey, musd_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            wallet.as_ref(),
            SPL_TOKEN_PROGRAM_ID.as_ref(),
            musd_mint.as_ref(),
        ],
        &ATA_PROGRAM_ID,
    )
    .0
}

pub fn handler(ctx: Context<MusdFaucet>) -> Result<()> {
    let wallet = ctx.accounts.wallet.key();
    let musd_mint = ctx.accounts.musd_mint.key();
    let destination = ctx.accounts.destination.key();

    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );

    // Minting is allowed only into the wallet's canonical MUSD ATA — same
    // derivation the ATA program uses.
    require_keys_eq!(
        destination,
        musd_ata(&wallet, &musd_mint),
        ErrorCode::InvalidAtaAddress
    );

    // One claim per wallet per hour (D33). The record stores the last
    // successful claim's `unix_timestamp`; on the wallet's very first call
    // the freshly created record is all zeros, so the check passes and this
    // call stamps the current time. Failed calls roll back everything.
    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp
            >= ctx.accounts.faucet_record.last_claim_time + FAUCET_COOLDOWN_SECS,
        ErrorCode::FaucetCooldown
    );
    ctx.accounts.faucet_record.last_claim_time = clock.unix_timestamp;
    ctx.accounts.faucet_record.bump = ctx.bumps.faucet_record;

    // Create the ATA if the wallet does not have one yet (the wallet pays the
    // rent). Repeating this call is safe: the ATA program turns
    // `create_idempotent` into a no-op when the account already exists
    // with the right owner and mint.
    let create_ata = Instruction {
        program_id: ATA_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(wallet, true),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(musd_mint, false),
            AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
            AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
        ],
        data: vec![ATA_IX_CREATE_IDEMPOTENT],
    };
    invoke(
        &create_ata,
        &[
            ctx.accounts.wallet.to_account_info(),
            ctx.accounts.destination.to_account_info(),
            ctx.accounts.wallet.to_account_info(),
            ctx.accounts.musd_mint.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
    )?;

    // Mint the fixed faucet amount, signed by the mint authority PDA. Hand
    // built like the `initialize_mint2` CPI in initialize.rs so the pinned
    // dependency set stays untouched.
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_MINT_TO);
    data.extend_from_slice(&FAUCET_AMOUNT.to_le_bytes());
    let mint_to = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(musd_mint, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(ctx.accounts.musd_mint_authority.key(), true),
        ],
        data,
    };
    invoke_signed(
        &mint_to,
        &[
            ctx.accounts.musd_mint.to_account_info(),
            ctx.accounts.destination.to_account_info(),
            ctx.accounts.musd_mint_authority.to_account_info(),
        ],
        &[&[MUSD_MINT_AUTHORITY_SEED, &[ctx.bumps.musd_mint_authority]]],
    )?;

    msg!("MUSD faucet minted {} base units", FAUCET_AMOUNT);
    Ok(())
}
