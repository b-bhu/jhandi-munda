//! `callback_roll_dice` (D11): the VRF oracle calls this with the randomness
//! for a pending roll. It turns the 32 bytes into six faces, stores them on the
//! round, and clears the pending request. `#[vrf_callback]` makes the oracle's
//! scoped identity a required signer, so only the VRF program can call it.

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::{anchor::vrf_callback, vrf::consts::scoped_vrf_identity_const};

use crate::{error::ErrorCode, state::{Round, RoundStatus}, ROUND_SEED};

/// This program's scoped VRF identity, computed at compile time. The macro's
/// own check derives it at runtime, which costs over 1.4M compute units; a
/// `const` costs nothing on chain.
const VRF_IDENTITY: Pubkey = Pubkey::new_from_array(scoped_vrf_identity_const(&crate::ID).0);

/// Bytes at or above this are skipped: 252 = 42 x 6, so every kept byte maps to
/// a face with no modulo bias (the same rule as the SDK's `random_u8_with_range`).
const BIAS_LIMIT: u8 = 252;

/// Six faces from the 32 VRF bytes (D11). Walk the bytes in order, skip any
/// byte >= 252, and give each kept byte its own die: face = (byte % 6) + 1.
/// No byte is reused. If fewer than six bytes are kept, return `RollExhausted`.
pub fn faces_from_randomness(randomness: &[u8; 32]) -> Result<[u8; 6]> {
    let mut faces = [0u8; 6];
    let mut count = 0;
    for &byte in randomness.iter() {
        if count == 6 {
            break;
        }
        if byte < BIAS_LIMIT {
            faces[count] = (byte % 6) + 1;
            count += 1;
        }
    }
    require!(count == 6, ErrorCode::RollExhausted);
    Ok(faces)
}

#[vrf_callback]
#[derive(Accounts)]
pub struct CallbackRollDice<'info> {
    /// Signed only by the VRF program when it calls back for this program.
    #[account(address = VRF_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,

    /// The table's round. Only a round with a pending roll accepts the faces.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,
}

pub fn handler(
    ctx: Context<CallbackRollDice>,
    randomness: [u8; 32],
    request_id: [u8; 32],
) -> Result<()> {
    let round = &mut ctx.accounts.round;
    // Reject a callback when no roll is pending (none requested, or already
    // fulfilled), or when it answers a different request (another round).
    require!(round.status == RoundStatus::Rolling, ErrorCode::UnexpectedCallback);
    require!(round.request_id == request_id, ErrorCode::UnexpectedCallback);

    round.faces = faces_from_randomness(&randomness)?;
    round.request_id = [0u8; 32];
    round.status = RoundStatus::Rolled;

    msg!("Round {} rolled: {:?}", round.round_id, round.faces);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // bytes[i] = i: the first six kept bytes are 0..=5, faces 1..=6.
    #[test]
    fn counting_bytes_give_six_distinct_faces() {
        let mut r = [0u8; 32];
        for (i, b) in r.iter_mut().enumerate() {
            *b = i as u8;
        }
        assert_eq!(faces_from_randomness(&r).unwrap(), [1, 2, 3, 4, 5, 6]);
    }

    // The first four bytes (252..=255) are skipped; the rest map 7..=34.
    #[test]
    fn bytes_at_or_above_252_are_skipped() {
        let mut r = [0u8; 32];
        for (i, b) in r.iter_mut().enumerate() {
            *b = if i < 4 { 252 + i as u8 } else { i as u8 + 3 };
        }
        // 7%6+1=2, 8->3, 9->4, 10->5, 11->6, 12->1
        assert_eq!(faces_from_randomness(&r).unwrap(), [2, 3, 4, 5, 6, 1]);
    }

    #[test]
    fn too_few_usable_bytes_is_an_error() {
        assert!(faces_from_randomness(&[255u8; 32]).is_err());
    }

    #[test]
    fn every_die_uses_its_own_byte() {
        // Same value everywhere would give one face six times; distinct
        // bytes give distinct dice here.
        let mut r = [0u8; 32];
        r[..6].copy_from_slice(&[10, 11, 12, 13, 14, 15]);
        let faces = faces_from_randomness(&r).unwrap();
        assert_eq!(faces, [5, 6, 1, 2, 3, 4]);
    }
}
