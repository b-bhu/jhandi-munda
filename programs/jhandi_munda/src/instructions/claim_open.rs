//! `open_claim` (D13, #25): turns one settled bet into a claim PDA. Stores what
//! it pays, when it unlocks (settle time + one round length, D7/D8) and the
//! early-exit cut. Opening a second claim for the same bet is rejected, since the
//! PDA already exists. Loss claims come in Step 7.3; redeeming comes in 7.2.

use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    state::{Bet, Claim, ClaimAsset},
    BET_SEED, CLAIM_SEED, LOSS_EARLY_EXIT_BPS, LOSS_UNLOCK_SECS, ROUND_LENGTH_SECS,
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

    /// Anyone can open a claim; the opener pays its rent.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_open_claim(ctx: Context<OpenClaim>) -> Result<()> {
    let bet = &ctx.accounts.bet;
    require!(bet.settled, ErrorCode::BetNotSettled);
    // A chip bet is paid into chips at settle (D39); it has no claim.
    require!(!bet.paid_with_chips, ErrorCode::ChipBetSettledToChips);

    // A win pays its owed MUSD after one round (D7). A loss pays MUNDA at one
    // base unit per base unit lost, after 24 hours (D23, D26).
    let (asset, amount, unlock_at, early_exit_bps) = if bet.owed > 0 {
        (ClaimAsset::Musd, bet.owed, bet.settled_at + ROUND_LENGTH_SECS, WIN_EARLY_EXIT_BPS)
    } else {
        (ClaimAsset::Munda, bet.amount, bet.settled_at + LOSS_UNLOCK_SECS, LOSS_EARLY_EXIT_BPS)
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
