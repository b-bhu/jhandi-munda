//! Settling a rolled round (D4, D9, D15, D17, D31), in two steps so a full
//! table fits Solana's transaction size (#102).
//!
//! - `settle_bets`: anyone passes any slice of the round's bets as remaining
//!   accounts. Each bet gets `owed` (stake back plus 98% of the profit, or 0 on
//!   a loss) and is marked settled, so it can never be counted twice. Each
//!   win's 2% fee accrues to the three pots on the table. No tokens move: the
//!   stakes already sit in the pool vault from `place_bet`.
//! - `finish_round`: only once every bet of the round is settled. The round
//!   goes idle, its `round_id` is bumped, and it is ready for the next round.

use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    payout::{gross_return, matches, settle_win, split_fee},
    state::{Bet, Round, RoundStatus, TableConfig},
    BET_SEED, ROUND_SEED, TABLE_SEED,
};

#[derive(Accounts)]
pub struct SettleBets<'info> {
    /// Table settings, and where the fee pots accrue.
    #[account(mut, seeds = [TABLE_SEED], bump)]
    pub table: Account<'info, TableConfig>,

    /// The rolled round whose bets are settled.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,
    // Remaining accounts: any slice of this round's bets, writable.
}

pub fn handle_settle_bets(ctx: Context<SettleBets>) -> Result<()> {
    let round = &mut ctx.accounts.round;
    require!(round.status == RoundStatus::Rolled, ErrorCode::RoundNotRolled);
    let round_id = round.round_id;
    let faces = round.faces;

    let mut processed: u16 = 0;
    let (mut treasury, mut devs, mut stakers) = (0u64, 0u64, 0u64);
    for info in ctx.remaining_accounts.iter() {
        // Each account must be this program's writable bet of this round, at
        // the PDA its own fields derive, and not settled yet.
        require_keys_eq!(*info.owner, crate::ID, ErrorCode::BetAccountMismatch);
        require!(info.is_writable, ErrorCode::BetAccountMismatch);
        let mut bet = {
            let data = info.try_borrow_data()?;
            Bet::try_deserialize(&mut &data[..])?
        };
        require!(bet.round_id == round_id, ErrorCode::BetAccountMismatch);
        require!(!bet.settled, ErrorCode::BetAlreadySettled);
        let expected = Pubkey::create_program_address(
            &[
                BET_SEED,
                &round_id.to_le_bytes(),
                bet.player.as_ref(),
                &[bet.symbol],
                &[bet.bump],
            ],
            &crate::ID,
        )
        .map_err(|_| error!(ErrorCode::BetAccountMismatch))?;
        require_keys_eq!(*info.key, expected, ErrorCode::BetAccountMismatch);

        // Price the bet on the round's faces (D9), then take the 2% win fee (D17).
        let k = matches(bet.symbol, &faces);
        let gross = gross_return(bet.amount, k).ok_or(ErrorCode::MathOverflow)?;
        let settled = settle_win(bet.amount, gross);
        let split = split_fee(settled.fee);
        treasury += split.treasury;
        devs += split.devs;
        stakers += split.stakers;

        bet.owed = settled.owed;
        bet.settled = true;
        let mut bytes = Vec::new();
        bet.try_serialize(&mut bytes)?;
        let mut data = info.try_borrow_mut_data()?;
        data[..bytes.len()].copy_from_slice(&bytes);
        processed += 1;
    }
    round.settled_count = round
        .settled_count
        .checked_add(processed)
        .ok_or(ErrorCode::MathOverflow)?;
    require!(round.settled_count <= round.bet_count, ErrorCode::IncompleteSettle);

    let table = &mut ctx.accounts.table;
    table.fee_treasury = table.fee_treasury.checked_add(treasury).ok_or(ErrorCode::MathOverflow)?;
    table.fee_devs = table.fee_devs.checked_add(devs).ok_or(ErrorCode::MathOverflow)?;
    table.fee_stakers = table.fee_stakers.checked_add(stakers).ok_or(ErrorCode::MathOverflow)?;

    msg!("Round {}: settled {} more bets ({} of {})", round_id, processed, round.settled_count, round.bet_count);
    Ok(())
}

#[derive(Accounts)]
pub struct FinishRound<'info> {
    /// The round to close. Every bet must be settled first.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,
}

pub fn handle_finish_round(ctx: Context<FinishRound>) -> Result<()> {
    let round = &mut ctx.accounts.round;
    require!(round.status == RoundStatus::Rolled, ErrorCode::RoundNotRolled);
    require!(round.settled_count == round.bet_count, ErrorCode::IncompleteSettle);

    // Idle again for the next round. Faces stay for the record until the next roll.
    round.status = RoundStatus::Idle;
    round.round_id += 1;
    round.opened_at = 0;
    round.players.clear();
    round.request_id = [0u8; 32];
    round.bet_count = 0;
    round.settled_count = 0;

    msg!("Round finished; round id is now {}", round.round_id);
    Ok(())
}
