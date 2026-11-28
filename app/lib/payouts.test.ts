import { describe, expect, it } from "vitest";
import { FEE_BPS, MIN_BET, PAYOUT_MULTIPLIER, PAYOUT_ROWS, grossReturn, matches, owedFor } from "./payouts";

const ONE = 1_000_000n; // 1 MUSD

describe("payout table (mirrors programs/jhandi_munda/src/payout.rs)", () => {
  it("pays 3/5/9/26/101x back for 2 to 6 matches and nothing for 0 or 1", () => {
    expect(PAYOUT_ROWS.map((r) => r.multiplier)).toEqual([3, 5, 9, 26, 101]);
    for (const k of [0, 1]) expect(owedFor(ONE, k)).toBe(0n);
  });
  it("matches the program's known cases: owed + fee == gross, fee is 2% of the profit", () => {
    // $1 at 2 and 6 matches (payout.rs tests): owed 2.96 and 99.00
    expect(owedFor(ONE, 2)).toBe(2_960_000n);
    expect(owedFor(ONE, 6)).toBe(99_000_000n);
    for (let k = 2; k <= 6; k++) {
      const gross = grossReturn(ONE, k);
      const fee = ((gross - ONE) * FEE_BPS) / 10_000n;
      expect(owedFor(ONE, k) + fee).toBe(gross);
    }
  });
  it("a $10 bet at 2 matches is owed 29.60", () => {
    expect(owedFor(10n * ONE, 2)).toBe(29_600_000n);
  });
  it("counts the dice that show a symbol", () => {
    expect(matches(2, [2, 2, 4, 6, 4, 5])).toBe(2);
    expect(matches(4, [2, 2, 4, 6, 4, 5])).toBe(2);
    expect(matches(1, [2, 2, 4, 6, 4, 5])).toBe(0);
    expect(matches(5, [5, 5, 5, 5, 5, 5])).toBe(6);
  });
  it("the minimum bet is $1 and every multiplier is for 2 to 6 matches only", () => {
    expect(MIN_BET).toBe(ONE);
    expect(Object.keys(PAYOUT_MULTIPLIER).map(Number)).toEqual([2, 3, 4, 5, 6]);
  });
});
