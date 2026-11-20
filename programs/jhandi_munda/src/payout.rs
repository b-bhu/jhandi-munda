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

/// Basis points in one whole (10_000 BPS = 100%).
const BPS: u128 = 10_000;

/// The win fee: 200 BPS (2%) on the profit of a win only (D17). Losers pay
/// nothing, and the stake a winner gets back is never charged.
pub const WIN_FEE_BPS: u128 = 200;

/// What a settled bet pays and the fee taken from it, in base units.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Settled {
    /// What the bettor is owed: the stake back plus 98% of the profit.
    pub owed: u64,
    /// The win fee, 2% of the profit. Zero on a loss.
    pub fee: u64,
}

/// Splits a total return into what the winner keeps and the fee (D17):
/// `owed = stake + (gross - stake) x 9_800 / 10_000`, rounded down. The fee is
/// the remainder, so `owed + fee == gross` to the base unit and the rounding
/// dust goes to the house. A loss (`gross` 0) pays and charges nothing.
pub fn settle_win(stake: u64, gross: u64) -> Settled {
    if gross <= stake {
        // A loss (gross 0) or a return of the stake alone: no profit, no fee.
        return Settled { owed: gross, fee: 0 };
    }
    let profit = gross - stake;
    let kept = (profit as u128 * (BPS - WIN_FEE_BPS) / BPS) as u64;
    Settled { owed: stake + kept, fee: profit - kept }
}

/// One win fee split three ways (D15): 20% treasury (into the insurance fund
/// until its target, D18), 10% devs, 70% stakers.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct FeeSplit {
    pub treasury: u64,
    pub devs: u64,
    pub stakers: u64,
}

/// Splits a fee 20 / 10 / 70. Treasury and devs round down; stakers take the
/// remainder, so the three always add back up to `fee` exactly.
pub fn split_fee(fee: u64) -> FeeSplit {
    let treasury = (fee as u128 * 2_000 / BPS) as u64;
    let devs = (fee as u128 * 1_000 / BPS) as u64;
    FeeSplit { treasury, devs, stakers: fee - treasury - devs }
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

    #[test]
    fn test_fee_on_profit_examples() {
        // $1 at 2 to 6 matches (3x, 5x, 9x, 26x, 101x): owed and fee in base units.
        let cases = [
            (2u8, 2_960_000u64, 40_000u64),
            (3, 4_920_000, 80_000),
            (4, 8_840_000, 160_000),
            (5, 25_500_000, 500_000),
            (6, 99_000_000, 2_000_000),
        ];
        for (k, owed, fee) in cases {
            let gross = gross_return(ONE_MUSD, k).unwrap();
            assert_eq!(settle_win(ONE_MUSD, gross), Settled { owed, fee }, "k={k}");
            assert_eq!(owed + fee, gross, "owed plus fee must equal the gross, k={k}");
        }
        // 0 or 1 match: a loss pays $0 and takes no fee.
        for k in [0u8, 1] {
            let gross = gross_return(ONE_MUSD, k).unwrap();
            assert_eq!(settle_win(ONE_MUSD, gross), Settled { owed: 0, fee: 0 });
        }
        // $10 at 3x is exactly ten times the $1 case.
        let ten = settle_win(10 * ONE_MUSD, gross_return(10 * ONE_MUSD, 2).unwrap());
        assert_eq!(ten, Settled { owed: 29_600_000, fee: 400_000 });
    }

    #[test]
    fn test_fee_split_20_10_70() {
        assert_eq!(
            split_fee(40_000),
            FeeSplit { treasury: 8_000, devs: 4_000, stakers: 28_000 }
        );
        // Rounding: treasury and devs floor, stakers take the remainder.
        assert_eq!(split_fee(1), FeeSplit { treasury: 0, devs: 0, stakers: 1 });
        assert_eq!(split_fee(7), FeeSplit { treasury: 1, devs: 0, stakers: 6 });
        for fee in [0u64, 1, 7, 99, 40_000, 400_000, u64::MAX / 3] {
            let s = split_fee(fee);
            assert_eq!(s.treasury + s.devs + s.stakers, fee, "split must add back up, fee={fee}");
        }
    }
}
