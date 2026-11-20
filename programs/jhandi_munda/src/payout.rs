//! Payout table (D9): total-return multipliers for a bet on one die face. Pure
//! math, no accounts. Amounts are MUSD base units (6 decimals, so $1 is
//! 1_000_000). The 2% win fee is not here; it comes in Step 6.2.

/// Six dice, so a roll has six faces.
pub const DICE: usize = 6;

/// Total-return multiplier by match count `k` (0 to 6): 0 and 0 for 0 or 1
/// match, then 3x / 5x / 9x / 26x / 101x for 2 to 6 matches (D9). A bet of $1
/// at 3x returns $3 in total, stake included.
pub const MULTIPLIERS: [u64; DICE + 1] = [0, 0, 3, 5, 9, 26, 101];

/// How many of the six faces equal `symbol` (0 to 6).
pub fn matches(symbol: u8, faces: &[u8; DICE]) -> u8 {
    faces.iter().filter(|&&face| face == symbol).count() as u8
}

/// Total return before fee for a stake that matched `k` faces: `stake x
/// multiplier(k)`, in base units. A loss returns 0. `None` on overflow.
pub fn gross_return(stake: u64, k: u8) -> Option<u64> {
    stake.checked_mul(*MULTIPLIERS.get(k as usize)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_MUSD: u64 = 1_000_000;

    #[test]
    fn test_payout_table_k0_to_k6() {
        assert_eq!(MULTIPLIERS, [0, 0, 3, 5, 9, 26, 101]);
        let expected = [0, 0, 3, 5, 9, 26, 101];
        for k in 0..=6u8 {
            assert_eq!(gross_return(ONE_MUSD, k), Some(expected[k as usize] * ONE_MUSD));
        }
        // $1 at 3x returns $3; at 101x returns $101; 0 or 1 match returns $0.
        assert_eq!(gross_return(ONE_MUSD, 2), Some(3_000_000));
        assert_eq!(gross_return(ONE_MUSD, 6), Some(101_000_000));
        assert_eq!(gross_return(ONE_MUSD, 1), Some(0));
        assert_eq!(gross_return(ONE_MUSD, 0), Some(0));
        // $10 at 3x returns $30: the table scales with the stake.
        assert_eq!(gross_return(10 * ONE_MUSD, 2), Some(30_000_000));
    }

    #[test]
    fn test_matches_counts_the_chosen_face() {
        assert_eq!(matches(3, &[1, 2, 3, 3, 5, 3]), 3);
        assert_eq!(matches(6, &[1, 2, 3, 3, 5, 3]), 0);
        assert_eq!(matches(4, &[4; 6]), 6);
    }

    #[test]
    fn test_gross_return_overflow_is_none() {
        assert_eq!(gross_return(u64::MAX, 6), None);
        assert_eq!(gross_return(1, 7), None, "k above 6 is not a bet outcome");
    }
}
