//! `place_bet` (D3, D5, D31): the first bet of a round opens it. The round
//! must be idle; the bet must be at least the table minimum (and under the max
//! when that cap is switched on, D6). The stake moves from the bettor's MUSD
//! ATA into the pool vault, and the round records `opened_at` from the Clock,
//! the 90 s timer's start (D8). Joining an open round is a later step.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};

use crate::{
    error::ErrorCode,
    state::{Bet, PoolState, Round, RoundStatus, TableConfig},
    BET_SEED, POOL_SEED, POOL_VAULT_SEED, ROUND_SEED, SPL_TOKEN_PROGRAM_ID, TABLE_SEED,
};

/// SPL token `Transfer` (opcode 3): data [3][amount u64 LE].
const TOKEN_IX_TRANSFER: u8 = 3;

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    /// Table settings: round length, min bet, max-bet switch (D3, D6).
    #[account(seeds = [TABLE_SEED], bump)]
    pub table: Account<'info, TableConfig>,

    /// The table's round account. Opened by this bet when idle.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,

    /// The bettor's bet record, created here.
    #[account(
        init,
        payer = bettor,
        space = 8 + Bet::INIT_SPACE,
        seeds = [BET_SEED, bettor.key().as_ref()],
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

pub fn handler(ctx: Context<PlaceBet>, symbol: u8, amount: u64) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    // Only a round with nobody in it can be opened; joining is a later step.
    require!(
        ctx.accounts.round.status == RoundStatus::Idle,
        ErrorCode::RoundAlreadyOpen
    );
    require!((1..=6).contains(&symbol), ErrorCode::InvalidSymbol);
    let table = &ctx.accounts.table;
    require!(amount >= table.min_bet, ErrorCode::BetBelowMinimum);
    if table.max_bet_enabled {
        require!(amount <= table.max_bet, ErrorCode::BetAboveMaximum);
    }
    let bettor = ctx.accounts.bettor.key();
    require_keys_eq!(
        ctx.accounts.bettor_ata.key(),
        crate::musd_ata(&bettor, &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );

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

    // The first bet opens the round and starts the 90 s timer (D31, D8).
    let now = Clock::get()?.unix_timestamp;
    let round = &mut ctx.accounts.round;
    round.status = RoundStatus::Open;
    round.opened_at = now;
    round.player_count = 1;

    let bet = &mut ctx.accounts.bet;
    bet.player = bettor;
    bet.amount = amount;
    bet.symbol = symbol;
    bet.bump = ctx.bumps.bet;

    msg!("Round opened at {} by a bet of {} base MUSD", now, amount);
    Ok(())
}
