//! `place_bet` (D3, D5, D10, D25, D31): a bet on one die face. The first bet of
//! an idle round opens it and records `opened_at` from the Clock. Later bets
//! join the open round: a wallet can bet on several faces (D10), and at most
//! `MAX_PLAYERS` distinct wallets sit in one round (D25). The stake moves from
//! the bettor's MUSD ATA into the pool vault. Bets close once the round has run
//! `round_length_secs` (90 s, D8) from `opened_at`: the window is
//! [opened_at, opened_at + 90), checked against the Clock, not slots (D8).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    error::ErrorCode,
    state::{Bet, PoolState, Round, RoundStatus, TableConfig},
    BET_SEED, MAX_PLAYERS, POOL_SEED, POOL_VAULT_SEED, ROUND_SEED, SPL_TOKEN_PROGRAM_ID,
    TABLE_SEED,
};

/// SPL token `Transfer` (opcode 3): data [3][amount u64 LE].
const TOKEN_IX_TRANSFER: u8 = 3;

#[derive(Accounts)]
#[instruction(symbol: u8, amount: u64)]
pub struct PlaceBet<'info> {
    /// Table settings: min bet, max-bet switch (D3, D6).
    #[account(seeds = [TABLE_SEED], bump)]
    pub table: Account<'info, TableConfig>,

    /// The table's round account. Opened by the first bet, joined by the rest.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,

    /// This wallet's bet on this die face in this round, created here. Its
    /// seeds include the round id and the symbol, so one wallet can hold one
    /// bet per face per round.
    #[account(
        init,
        payer = bettor,
        space = 8 + Bet::INIT_SPACE,
        seeds = [
            BET_SEED,
            round.round_id.to_le_bytes().as_ref(),
            bettor.key().as_ref(),
            &[symbol],
        ],
        bump
    )]
    pub bet: Account<'info, Bet>,

    /// The house pool: read for the MUSD mint the bet is paid in.
    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool MUSD vault from `initialize_pool`; the seeds pin its
    /// address and the transfer CPI below credits it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump, owner = SPL_TOKEN_PROGRAM_ID)]
    pub vault: UncheckedAccount<'info>,

    /// The bettor: signs the stake transfer and pays the bet record's rent.
    #[account(mut)]
    pub bettor: Signer<'info>,

    /// CHECK: the bettor's canonical MUSD ATA, checked in the handler.
    #[account(mut)]
    pub bettor_ata: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Checks shared by both bet paths (base-layer MUSD and chips, D39): the symbol,
/// the table limits, the join window (D8) and the seats (D25). Returns whether
/// the bettor takes a new seat.
pub fn check_bet(
    round: &Round,
    table: &TableConfig,
    bettor: &Pubkey,
    symbol: u8,
    amount: u64,
    now: i64,
) -> Result<bool> {
    require!((1..=6).contains(&symbol), ErrorCode::InvalidSymbol);
    require!(amount >= table.min_bet, ErrorCode::BetBelowMinimum);
    if table.max_bet_enabled {
        require!(amount <= table.max_bet, ErrorCode::BetAboveMaximum);
    }
    // The join window closes 90 s after the round opened (D8). Clock time only.
    if round.status != RoundStatus::Idle {
        require!(now < round.opened_at + table.round_length_secs, ErrorCode::BettingClosed);
    }
    // A wallet already in the round does not take a seat; a new wallet needs one
    // of the MAX_PLAYERS seats (D25).
    let is_new_player = !round.players.contains(bettor);
    require!(!is_new_player || round.players.len() < MAX_PLAYERS, ErrorCode::TableFull);
    Ok(is_new_player)
}

/// Records a checked bet on the round and on its bet account.
#[allow(clippy::too_many_arguments)]
pub fn record_bet(
    round: &mut Round,
    bet: &mut Bet,
    bettor: Pubkey,
    symbol: u8,
    amount: u64,
    bump: u8,
    is_new_player: bool,
    now: i64,
) {
    if is_new_player {
        round.players.push(bettor);
    }
    // The first bet opens an idle round and starts the clock (D31, D8).
    if round.status == RoundStatus::Idle {
        round.status = RoundStatus::Open;
        round.opened_at = now;
    }
    bet.player = bettor;
    bet.amount = amount;
    bet.symbol = symbol;
    bet.round_id = round.round_id;
    bet.owed = 0;
    bet.settled = false;
    bet.settled_at = 0;
    bet.bump = bump;
    round.bet_count += 1;
}

pub fn handler(ctx: Context<PlaceBet>, symbol: u8, amount: u64) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    let bettor = ctx.accounts.bettor.key();
    require_keys_eq!(
        ctx.accounts.bettor_ata.key(),
        crate::musd_ata(&bettor, &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );
    let now = Clock::get()?.unix_timestamp;
    let is_new_player = check_bet(&ctx.accounts.round, &ctx.accounts.table, &bettor, symbol, amount, now)?;

    // Stake into the pool vault, signed by the bettor.
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_TRANSFER);
    data.extend_from_slice(&amount.to_le_bytes());
    let transfer = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.bettor_ata.key(), false),
            AccountMeta::new(ctx.accounts.vault.key(), false),
            AccountMeta::new_readonly(bettor, true),
        ],
        data,
    };
    invoke(
        &transfer,
        &[
            ctx.accounts.bettor_ata.to_account_info(),
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.bettor.to_account_info(),
        ],
    )?;

    record_bet(
        &mut ctx.accounts.round,
        &mut ctx.accounts.bet,
        bettor,
        symbol,
        amount,
        ctx.bumps.bet,
        is_new_player,
        now,
    );
    msg!("Bet of {} base MUSD on face {} in round {}", amount, symbol, ctx.accounts.round.round_id);
    Ok(())
}
