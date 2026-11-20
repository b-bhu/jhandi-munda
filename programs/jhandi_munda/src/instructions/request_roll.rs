//! `request_roll` (D11, D31): once the join window has closed, anyone can ask
//! MagicBlock VRF for the six dice of the open round. The request is sent on
//! the base-layer `DEFAULT_QUEUE`, and the round stores the request id and
//! moves to `Rolling`. Rolling rejects a second request. The callback that
//! consumes the randomness is Step 5.2.

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::vrf;
use ephemeral_rollups_sdk::vrf::{
    consts::{program_identity_pubkey, DEFAULT_QUEUE},
    instructions::{create_request_randomness_ix_const, RequestRandomnessParams},
    types::SerializableAccountMeta,
};

use crate::{error::ErrorCode, state::{Round, RoundStatus, TableConfig}, ROUND_SEED, TABLE_SEED};

/// Anchor sighash of `global:callback_roll_dice`, the callback Step 5.2 adds:
/// sha256("global:callback_roll_dice")[..8].
const CALLBACK_DISCRIMINATOR: [u8; 8] = [129, 76, 217, 160, 252, 234, 19, 238];

/// The identity PDA this program signs VRF requests with, derived at compile
/// time from `crate::ID` (never the deprecated global identity).
const PROGRAM_IDENTITY: Pubkey = program_identity_pubkey(&crate::ID);

#[vrf]
#[derive(Accounts)]
pub struct RequestRoll<'info> {
    /// Table settings: the round length that sets when the window closes.
    #[account(seeds = [TABLE_SEED], bump)]
    pub table: Account<'info, TableConfig>,

    /// The table's round account. Must be open and past its join window.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,

    /// Anyone can request a roll and pays the VRF request fee.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: the base-layer VRF queue (D11). Pinned to `DEFAULT_QUEUE`.
    #[account(mut, address = DEFAULT_QUEUE)]
    pub oracle_queue: UncheckedAccount<'info>,
}

pub fn handler(ctx: Context<RequestRoll>) -> Result<()> {
    let round = &ctx.accounts.round;
    require!(round.status != RoundStatus::Rolling, ErrorCode::RollAlreadyPending);
    require!(round.status == RoundStatus::Open, ErrorCode::NoRoundToRoll);
    let now = Clock::get()?.unix_timestamp;
    require!(
        now >= round.opened_at + ctx.accounts.table.round_length_secs,
        ErrorCode::WindowStillOpen
    );

    // The caller seed is built from on-chain state, never a value the
    // requester picks: round id, opened_at and slot make it unique per request.
    // The VRF output is what makes the roll fair.
    let slot = Clock::get()?.slot;
    let mut caller_seed = [0u8; 32];
    caller_seed[0..8].copy_from_slice(&round.round_id.to_le_bytes());
    caller_seed[8..16].copy_from_slice(&round.opened_at.to_le_bytes());
    caller_seed[16..24].copy_from_slice(&slot.to_le_bytes());

    let params = RequestRandomnessParams {
        payer: ctx.accounts.payer.key(),
        oracle_queue: ctx.accounts.oracle_queue.key(),
        callback_program_id: crate::ID,
        // The callback instruction lands in Step 5.2; its name is fixed here.
        callback_discriminator: CALLBACK_DISCRIMINATOR.to_vec(),
        caller_seed,
        accounts_metas: Some(vec![SerializableAccountMeta {
            pubkey: ctx.accounts.round.key(),
            is_signer: false,
            is_writable: true,
        }]),
        callback_args: None,
    };
    let ix = create_request_randomness_ix_const(params, PROGRAM_IDENTITY);
    ctx.accounts
        .invoke_signed_vrf(&ctx.accounts.payer.to_account_info(), &ix)?;

    let round = &mut ctx.accounts.round;
    round.request_id = caller_seed;
    round.status = RoundStatus::Rolling;

    msg!("VRF roll requested for round {}", round.round_id);
    Ok(())
}
