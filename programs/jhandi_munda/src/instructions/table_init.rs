//! Table scaffold (D5, D8, D25): a one-shot `initialize_table` that creates the
//! table config PDA (round length, min bet, max bet off by default) and the
//! vacant round PDA. No bets are accepted here; placing bets, the timer and
//! settlement are later steps.

use anchor_lang::prelude::*;

use crate::{
    state::{Round, RoundStatus, TableConfig},
    MIN_BET, ROUND_LENGTH_SECS, ROUND_SEED, TABLE_SEED,
};

#[derive(Accounts)]
pub struct InitializeTable<'info> {
    /// The program config from `initialize`. Only its admin (the table authority, D40) may set up the
    /// table: otherwise anyone could set it up first (#111).
    #[account(seeds = [crate::CONFIG_SEED], bump)]
    pub config: Account<'info, crate::state::ProgramConfig>,

    /// Table settings. One per program; seeds include no arguments.
    #[account(
        init,
        payer = payer,
        space = 8 + TableConfig::INIT_SPACE,
        seeds = [TABLE_SEED],
        bump
    )]
    pub table: Account<'info, TableConfig>,

    /// The table's round account, created vacant (`Idle`).
    #[account(
        init,
        payer = payer,
        space = 8 + Round::INIT_SPACE,
        seeds = [ROUND_SEED],
        bump
    )]
    pub round: Account<'info, Round>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializeTable>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.config.admin,
        ctx.accounts.payer.key(),
        crate::error::ErrorCode::NotTableAuthority
    );
    let table = &mut ctx.accounts.table;
    table.round_length_secs = ROUND_LENGTH_SECS;
    table.min_bet = MIN_BET;
    // Max bet exists as config but is off by default on devnet (D6).
    table.max_bet = 0;
    table.max_bet_enabled = false;
    table.fee_treasury = 0;
    table.fee_devs = 0;
    table.fee_stakers = 0;
    table.bump = ctx.bumps.table;

    let round = &mut ctx.accounts.round;
    round.status = RoundStatus::Idle;
    round.opened_at = 0;
    round.round_id = 0;
    round.players = Vec::new();
    round.request_id = [0u8; 32];
    round.faces = [0u8; 6];
    round.bet_count = 0;
    round.settled_count = 0;
    round.bump = ctx.bumps.round;

    msg!("Table config and idle round initialized");
    Ok(())
}

/// Creates the round PDA (`ROUND_SEED`, D41) on a table that was set up before the round was
/// versioned. Table authority only (D40). `init` fails if the account already exists.
#[derive(Accounts)]
pub struct InitRound<'info> {
    #[account(
        seeds = [crate::CONFIG_SEED],
        bump,
        constraint = config.admin == payer.key() @ crate::error::ErrorCode::NotTableAuthority
    )]
    pub config: Account<'info, crate::state::ProgramConfig>,

    #[account(
        init,
        payer = payer,
        space = 8 + Round::INIT_SPACE,
        seeds = [ROUND_SEED],
        bump
    )]
    pub round: Account<'info, Round>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_init_round(ctx: Context<InitRound>) -> Result<()> {
    let round = &mut ctx.accounts.round;
    round.status = RoundStatus::Idle;
    round.opened_at = 0;
    round.round_id = 0;
    round.players = Vec::new();
    round.request_id = [0u8; 32];
    round.faces = [0u8; 6];
    round.bet_count = 0;
    round.settled_count = 0;
    round.bump = ctx.bumps.round;
    msg!("Round (round_v2) initialized, idle");
    Ok(())
}
