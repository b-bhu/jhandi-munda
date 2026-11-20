//! The house pool scaffold (D4, D27): a one-shot `initialize_pool` that
//! creates the pool state PDA, its MUSD vault, and the pool share (LP) mint.
//! The pool starts **empty** (D4): the vault holds 0 MUSD and 0 shares are
//! minted, so NAV is $1.00 by definition until the first deposit. Deposit,
//! withdraw, NAV display and everything downstream are later steps — this
//! only puts the accounts in place.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};

use crate::{
    error::ErrorCode, state::{PoolState, ProgramConfig}, CONFIG_SEED, MUSD_DECIMALS,
    MUSD_MINT_SEED, MUSD_MINT_SPACE, POOL_SEED, POOL_VAULT_SEED, SHARE_MINT_SEED,
    SPL_TOKEN_PROGRAM_ID, TOKEN_ACCOUNT_SPACE,
};

/// SPL token `InitializeAccount3` (opcode 18): data [18][authority 32 bytes].
/// Accounts: vault (writable), mint (readonly). No rent sysvar needed because
/// Anchor's `init` funds the vault rent-exempt earlier in the same
/// transaction; like `initialize_mint2` in initialize.rs it is hand built so
/// the pinned dependency set stays untouched.
const TOKEN_IX_INITIALIZE_ACCOUNT3: u8 = 18;

/// SPL token `InitializeMint2` (opcode 20), same as the MUSD mint in
/// initialize.rs. Used here for the pool-share (LP) mint.
const TOKEN_IX_INITIALIZE_MINT2: u8 = 20;

#[derive(Accounts)]
pub struct InitializePool<'info> {
    /// Program config from `initialize` (Step 2): proves the setup exists and
    /// pins which MUSD mint the vault must hold.
    #[account(seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, ProgramConfig>,

    /// The house pool itself. Created here, one per program, and initialized
    /// empty (D4): nothing is vaulted and no shares exist yet.
    #[account(
        init,
        payer = payer,
        space = 8 + PoolState::INIT_SPACE,
        seeds = [POOL_SEED],
        bump
    )]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool share (LP) mint, created here owned by the SPL token
    /// program (82 bytes, Mint::LEN) and set up by the `initialize_mint2` CPI
    /// in the handler below, exactly like the MUSD mint in initialize.rs. Its
    /// mint authority is the pool PDA, so only pool instructions can mint or
    /// burn shares (D27).
    #[account(
        init,
        payer = payer,
        space = MUSD_MINT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [SHARE_MINT_SEED],
        bump
    )]
    pub share_mint: UncheckedAccount<'info>,

    /// CHECK: the raw MUSD vault token account, created here owned by the SPL
    /// token program (165 bytes, Token Account::LEN) and initialized by the
    /// `initialize_account3` CPI in the handler with the pool PDA as its
    /// authority, so only pool instructions ever move the MUSD inside.
    #[account(
        init,
        payer = payer,
        space = TOKEN_ACCOUNT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [POOL_VAULT_SEED],
        bump
    )]
    pub vault: UncheckedAccount<'info>,

    /// CHECK: the Step 2 MUSD mint, reused — its seeds pin the address, the
    /// constraint ties it to `config.musd_mint`, and nothing here writes to
    /// it (no second dollar mint is ever created).
    #[account(
        seeds = [MUSD_MINT_SEED],
        bump,
        owner = SPL_TOKEN_PROGRAM_ID,
        constraint = config.musd_mint == musd_mint.key()
            @ ErrorCode::MusdMintMismatch
    )]
    pub musd_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler before the CPIs.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializePool>) -> Result<()> {
    let pool_key = ctx.accounts.pool.key();

    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );

    // Initialize the empty vault token account with the pool PDA as its
    // authority. Data: [18][authority] with no freeze authority field.
    let mut data = Vec::with_capacity(33);
    data.push(TOKEN_IX_INITIALIZE_ACCOUNT3);
    data.extend_from_slice(pool_key.as_ref());

    let init_vault = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.vault.key(), false),
            AccountMeta::new_readonly(ctx.accounts.musd_mint.key(), false),
        ],
        data,
    };
    invoke_signed(
        &init_vault,
        &[
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.musd_mint.to_account_info(),
        ],
        &[&[POOL_SEED, &[ctx.bumps.pool]]],
    )?;

    // Initialize the share (LP) mint, authority and freeze-free like the MUSD
    // mint. The share uses the same 6 decimals as MUSD (MUSD_DECIMALS) so one
    // share maps to one MUSD unit at the defined empty-pool NAV of $1.00.
    let mut data = Vec::with_capacity(35);
    data.push(TOKEN_IX_INITIALIZE_MINT2);
    data.push(MUSD_DECIMALS);
    data.extend_from_slice(pool_key.as_ref());
    data.push(0u8);

    let init_share_mint = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![AccountMeta::new(ctx.accounts.share_mint.key(), false)],
        data,
    };
    invoke_signed(
        &init_share_mint,
        &[ctx.accounts.share_mint.to_account_info()],
        &[&[POOL_SEED, &[ctx.bumps.pool]]],
    )?;

    let pool = &mut ctx.accounts.pool;
    pool.musd_mint = ctx.accounts.musd_mint.key();
    pool.share_mint = ctx.accounts.share_mint.key();
    pool.vault = ctx.accounts.vault.key();
    pool.bump = ctx.bumps.pool;
    pool.vault_bump = ctx.bumps.vault;
    pool.share_mint_bump = ctx.bumps.share_mint;

    // Nothing is vaulted and no shares exist: NAV is $1.00 by definition
    // (`crate::nav_per_share(0, 0) == 1_000_000`).
    msg!("Pool initialized empty: 0 MUSD in vault, 0 shares, NAV $1.00");
    Ok(())
}
