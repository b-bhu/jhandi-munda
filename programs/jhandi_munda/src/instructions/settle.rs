//! Settling a rolled round (D4, D9, D15, D17, D31, D39), in steps so a full table
//! fits Solana's transaction size (#102).
//!
//! - `settle_bets`: base-layer bets (MUSD already in the pool vault). Each bet gets
//!   `owed` (stake back plus 98% of the profit, or 0 on a loss) and is marked
//!   settled, so it cannot be counted twice. Each win's 2% fee accrues to the three
//!   pots on the table. No tokens move.
//! - `settle_chip_bets` (D39): chip bets. Each stake moves from the cage vault to
//!   the pool vault. A win is then paid from the pool vault back into the cage vault
//!   and credited to the player's chips. Runs on Solana (the vaults are not delegated).
//! - `finish_round`: only once every bet of the round is settled. The round goes
//!   idle, and its `round_id` is bumped.

use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    instructions::claim_redeem::{spl_transfer, token_balance},
    instructions::payout_queue::enqueue,
    state::PayoutQueue,
    payout::{gross_return, loss_fee, matches, settle_win, split_fee, FeeSplit, Settled},
    state::{Bet, ChipStack, PoolState, Round, RoundStatus, TableConfig},
    BET_SEED, CAGE_VAULT_SEED, CHIPS_SEED, POOL_SEED, POOL_VAULT_SEED, QUEUE_SEED, ROUND_SEED,
    SPL_TOKEN_PROGRAM_ID, TABLE_SEED,
};

/// Loads a bet from a remaining account and checks it is this program's unsettled
/// writable bet of `round_id`, at the PDA its own fields derive.
fn load_bet(info: &AccountInfo, round_id: u64) -> Result<Bet> {
    require_keys_eq!(*info.owner, crate::ID, ErrorCode::BetAccountMismatch);
    require!(info.is_writable, ErrorCode::BetAccountMismatch);
    let bet = {
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
    Ok(bet)
}

/// Writes a bet or chip stack back to its account.
pub(crate) fn save_account<T: AccountSerialize>(info: &AccountInfo, value: &T) -> Result<()> {
    let mut bytes = Vec::new();
    value.try_serialize(&mut bytes)?;
    let mut data = info.try_borrow_mut_data()?;
    data[..bytes.len()].copy_from_slice(&bytes);
    Ok(())
}

/// Prices one bet on the round's faces (D9) and takes the 2% fee (D17 on a win,
/// D45 on a loss: 2% of the lost stake, accrued from pool money).
fn price(bet: &mut Bet, faces: &[u8; 6], now: i64) -> Result<(Settled, FeeSplit)> {
    let k = matches(bet.symbol, faces);
    let gross = gross_return(bet.amount, k).ok_or(ErrorCode::MathOverflow)?;
    let mut settled = settle_win(bet.amount, gross);
    if settled.owed == 0 {
        settled.fee = loss_fee(bet.amount);
    }
    bet.owed = settled.owed;
    bet.settled = true;
    bet.settled_at = now;
    Ok((settled, split_fee(settled.fee)))
}

#[derive(Accounts)]
pub struct SettleBets<'info> {
    /// Table settings, and where the fee pots accrue.
    #[account(mut, seeds = [TABLE_SEED], bump)]
    pub table: Account<'info, TableConfig>,

    /// The rolled round whose bets are settled.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,
    // Remaining accounts: any slice of this round's base-layer bets, writable.
}

pub fn handle_settle_bets(ctx: Context<SettleBets>) -> Result<()> {
    let round = &mut ctx.accounts.round;
    require!(round.status == RoundStatus::Rolled, ErrorCode::RoundNotRolled);
    let round_id = round.round_id;
    let faces = round.faces;
    let now = Clock::get()?.unix_timestamp;

    let mut processed: u16 = 0;
    let (mut treasury, mut devs, mut stakers) = (0u64, 0u64, 0u64);
    for info in ctx.remaining_accounts.iter() {
        let mut bet = load_bet(info, round_id)?;
        require!(!bet.paid_with_chips, ErrorCode::ChipBetNeedsChipSettle);
        let (_, split) = price(&mut bet, &faces, now)?;
        treasury += split.treasury;
        devs += split.devs;
        stakers += split.stakers;
        save_account(info, &bet)?;
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
pub struct SettleChipBets<'info> {
    #[account(mut, seeds = [TABLE_SEED], bump)]
    pub table: Account<'info, TableConfig>,

    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,

    /// The house pool: its vault receives the chip stakes and pays the wins.
    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool MUSD vault, seeds pin it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump)]
    pub pool_vault: UncheckedAccount<'info>,

    /// CHECK: the cage vault (chip money), seeds pin it. Owned by the pool PDA.
    #[account(mut, seeds = [CAGE_VAULT_SEED], bump)]
    pub cage_vault: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    /// The payout queue (D30). A chip win the pool cannot pay now joins its back (#110).
    #[account(mut, seeds = [QUEUE_SEED], bump)]
    pub queue: AccountLoader<'info, PayoutQueue>,
    // Remaining accounts, in pairs: each chip bet (writable), then its player's
    // ChipStack (writable).
}

pub fn handle_settle_chip_bets<'info>(ctx: Context<'info, SettleChipBets<'info>>) -> Result<()> {
    require_keys_eq!(ctx.accounts.token_program.key(), SPL_TOKEN_PROGRAM_ID, ErrorCode::InvalidTokenProgram);
    let round = &mut ctx.accounts.round;
    require!(round.status == RoundStatus::Rolled, ErrorCode::RoundNotRolled);
    let round_id = round.round_id;
    let faces = round.faces;
    let now = Clock::get()?.unix_timestamp;
    let seeds: &[&[u8]] = &[POOL_SEED, &[ctx.accounts.pool.bump]];
    let pool_info = ctx.accounts.pool.to_account_info();
    let pool_vault = ctx.accounts.pool_vault.to_account_info();
    let cage_vault = ctx.accounts.cage_vault.to_account_info();
    let mut queue = ctx.accounts.queue.load_mut()?;

    let mut processed: u16 = 0;
    let (mut treasury, mut devs, mut stakers) = (0u64, 0u64, 0u64);
    for pair in ctx.remaining_accounts.chunks_exact(2) {
        let (bet_info, chips_info) = (&pair[0], &pair[1]);
        let mut bet = load_bet(bet_info, round_id)?;
        require!(bet.paid_with_chips, ErrorCode::BetAccountMismatch);

        // The chip stack must be this program's stack for the bettor.
        require_keys_eq!(*chips_info.owner, crate::ID, ErrorCode::BetAccountMismatch);
        require!(chips_info.is_writable, ErrorCode::BetAccountMismatch);
        let expected_chips = Pubkey::find_program_address(&[CHIPS_SEED, bet.player.as_ref()], &crate::ID).0;
        require_keys_eq!(*chips_info.key, expected_chips, ErrorCode::BetAccountMismatch);
        let mut chips = {
            let data = chips_info.try_borrow_data()?;
            ChipStack::try_deserialize(&mut &data[..])?
        };

        // 1. The stake moves from the cage to the pool: it is pool money from now on.
        spl_transfer(&cage_vault, &pool_vault, &pool_info, bet.amount, &[seeds])?;
        // 2. The win (if any). D30: when the pool can pay it now and nothing is queued ahead, it is
        // paid from the pool back to the cage and credited to chips. Otherwise it joins the back of
        // the FIFO queue, keyed by the chip stack, and `pay_queue` credits it later (#110). The round
        // still settles, so one big win cannot freeze everyone else's bets.
        let (settled, split) = price(&mut bet, &faces, now)?;
        if settled.owed > 0 {
            let queue_empty = queue.head == queue.tail;
            if queue_empty && token_balance(&pool_vault)? >= settled.owed {
                spl_transfer(&pool_vault, &cage_vault, &pool_info, settled.owed, &[seeds])?;
                chips.balance = chips.balance.checked_add(settled.owed).ok_or(ErrorCode::MathOverflow)?;
            } else {
                enqueue(&mut queue, chips_info.key(), settled.owed)?;
                msg!("Chip win of {} queued: the pool cannot pay it now (D30)", settled.owed);
            }
        }
        treasury += split.treasury;
        devs += split.devs;
        stakers += split.stakers;

        save_account(bet_info, &bet)?;
        save_account(chips_info, &chips)?;
        processed += 1;
    }
    round.settled_count = round.settled_count.checked_add(processed).ok_or(ErrorCode::MathOverflow)?;
    require!(round.settled_count <= round.bet_count, ErrorCode::IncompleteSettle);

    let table = &mut ctx.accounts.table;
    table.fee_treasury = table.fee_treasury.checked_add(treasury).ok_or(ErrorCode::MathOverflow)?;
    table.fee_devs = table.fee_devs.checked_add(devs).ok_or(ErrorCode::MathOverflow)?;
    table.fee_stakers = table.fee_stakers.checked_add(stakers).ok_or(ErrorCode::MathOverflow)?;
    msg!("Round {}: settled {} chip bets", round_id, processed);
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
