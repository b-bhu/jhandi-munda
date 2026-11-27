//! `open_claim` (D13, #25): turns one settled bet into a claim PDA. Stores what
//! it pays, when it unlocks (settle time + one round length, D7/D8) and the
//! early-exit cut. Opening a second claim for the same bet is rejected, since the
//! PDA already exists. Loss claims come in Step 7.3; redeeming comes in 7.2.

use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    payout::loss_munda,
    state::{Bet, Claim, ClaimAsset, LossTally},
    BET_SEED, CLAIM_SEED, LOSS_TALLY_SEED, LOSS_UNLOCK_SECS, ROUND_LENGTH_SECS,
    WIN_EARLY_EXIT_BPS,
};

#[derive(Accounts)]
pub struct OpenClaim<'info> {
    /// The settled bet this claim settles. Its seeds bind it to the round.
    #[account(
        seeds = [BET_SEED, bet.round_id.to_le_bytes().as_ref(), bet.player.as_ref(), &[bet.symbol]],
        bump = bet.bump
    )]
    pub bet: Account<'info, Bet>,

    /// The claim for this bet, created here. One per bet.
    #[account(
        init,
        payer = payer,
        space = 8 + Claim::INIT_SPACE,
        seeds = [CLAIM_SEED, bet.key().as_ref()],
        bump
    )]
    pub claim: Account<'info, Claim>,

    /// The global loss tally (D49): read to price the early-loser bonus, then increased by
    /// this loss. Created on the first claim opened.
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + LossTally::INIT_SPACE,
        seeds = [LOSS_TALLY_SEED],
        bump
    )]
    pub loss_tally: Account<'info, LossTally>,

    /// Anyone can open a claim; the opener pays its rent.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_open_claim(ctx: Context<OpenClaim>) -> Result<()> {
    let bet = &ctx.accounts.bet;
    require!(bet.settled, ErrorCode::BetNotSettled);
    // A winning chip bet is paid into chips at settle (D39), or queued by chip stack, so it has no
    // claim. A losing chip bet earns MUNDA like any loss (D51): same tally, same 5x bonus.
    require!(!(bet.paid_with_chips && bet.owed > 0), ErrorCode::ChipBetSettledToChips);

    // A win pays its owed MUSD after one round (D7). A loss pays MUNDA after 24 hours (D23), Wait
    // only (D50): its early-exit rate is 0 and `redeem_take_now` refuses a MUNDA claim.
    // The loss's MUNDA is priced now, from the global tally, so redeem order cannot change it (D49).
    let (asset, amount, unlock_at, early_exit_bps) = if bet.owed > 0 {
        (ClaimAsset::Musd, bet.owed, bet.settled_at + ROUND_LENGTH_SECS, WIN_EARLY_EXIT_BPS)
    } else {
        let tally = &mut ctx.accounts.loss_tally;
        let munda = loss_munda(tally.total_lost, bet.amount);
        tally.total_lost = tally.total_lost.checked_add(bet.amount).ok_or(ErrorCode::MathOverflow)?;
        tally.bump = ctx.bumps.loss_tally;
        (ClaimAsset::Munda, munda, bet.settled_at + LOSS_UNLOCK_SECS, 0)
    };

    let claim = &mut ctx.accounts.claim;
    claim.bet = bet.key();
    claim.player = bet.player;
    claim.asset = asset;
    claim.amount = amount;
    claim.unlock_at = unlock_at;
    claim.early_exit_bps = early_exit_bps;
    claim.redeemed = false;
    claim.queued = false;
    claim.insurance_owed = 0;
    claim.bump = ctx.bumps.claim;

    msg!("Claim opened: {} base MUSD, unlocks at {}", claim.amount, claim.unlock_at);
    Ok(())
}
