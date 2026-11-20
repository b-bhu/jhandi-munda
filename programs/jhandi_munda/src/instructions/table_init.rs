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
    let table = &mut ctx.accounts.table;
    table.round_length_secs = ROUND_LENGTH_SECS;
    table.min_bet = MIN_BET;
    // Max bet exists as config but is off by default on devnet (D6).
    table.max_bet = 0;
    table.max_bet_enabled = false;
    table.bump = ctx.bumps.table;

    let round = &mut ctx.accounts.round;
    round.status = RoundStatus::Idle;
    round.opened_at = 0;
    round.round_id = 0;
    round.players = Vec::new();
    round.bump = ctx.bumps.round;

    msg!("Table config and idle round initialized");
    Ok(())
}
