//! `settle` (D4, D9, D15, D17, D31): anyone can settle a rolled round, so it
//! never waits on us. Every bet of the round is passed as a remaining account.
//! Each bet gets `owed` (stake back plus 98% of the profit, or 0 on a loss),
//! and each win's 2% fee accrues to the three pots on the table. Losing stakes
//! stay in the pool vault, and no tokens move here: stakes already sit in the
//! vault from `place_bet`. Afterwards the round goes idle, with `round_id`
//! bumped for the next round.

use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    payout::{gross_return, matches, settle_win, split_fee},
    state::{Bet, Round, RoundStatus, TableConfig},
    BET_SEED, ROUND_SEED, TABLE_SEED,
};

#[derive(Accounts)]
pub struct Settle<'info> {
    /// Table settings, and where the fee pots accrue.
    #[account(mut, seeds = [TABLE_SEED], bump)]
    pub table: Account<'info, TableConfig>,

    /// The round to settle. Must be rolled, and is idle again afterwards.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,
    // Remaining accounts: every bet of this round, writable.
}

pub fn handler(ctx: Context<Settle>) -> Result<()> {
    let round = &mut ctx.accounts.round;
    require!(round.status == RoundStatus::Rolled, ErrorCode::RoundNotRolled);
    let round_id = round.round_id;
    let faces = round.faces;

    let mut processed: u16 = 0;
    let (mut treasury, mut devs, mut stakers) = (0u64, 0u64, 0u64);
    for info in ctx.remaining_accounts.iter() {
        // Each account must be this program's writable, unsettled bet of this
        // round, at the PDA its own fields derive.
        require_keys_eq!(*info.owner, crate::ID, ErrorCode::BetAccountMismatch);
        require!(info.is_writable, ErrorCode::BetAccountMismatch);
        let mut bet = {
            let data = info.try_borrow_data()?;
            Bet::try_deserialize(&mut &data[..])?
        };
        require!(bet.round_id == round_id && !bet.settled, ErrorCode::BetAccountMismatch);
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
    // Every bet of the round must be settled; a skipped bet would be unpaid.
    require!(processed == round.bet_count, ErrorCode::IncompleteSettle);

    let table = &mut ctx.accounts.table;
    table.fee_treasury = table.fee_treasury.checked_add(treasury).ok_or(ErrorCode::MathOverflow)?;
    table.fee_devs = table.fee_devs.checked_add(devs).ok_or(ErrorCode::MathOverflow)?;
    table.fee_stakers = table.fee_stakers.checked_add(stakers).ok_or(ErrorCode::MathOverflow)?;

    // The round is done: idle, next round id, no players or bets. Faces stay
    // for the record until the next roll.
    round.status = RoundStatus::Idle;
    round.round_id += 1;
    round.opened_at = 0;
    round.players.clear();
    round.request_id = [0u8; 32];
    round.bet_count = 0;

    msg!("Round {} settled: {} bets", round_id, processed);
    Ok(())
}
