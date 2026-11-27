/*
 * The payout table, shared by the landing page and the table screen (decision 9, build-plan).
 * Payouts are total return: a $1 bet at 3x gives $3 back. 0 or 1 match pays nothing.
 * The 2% fee is taken on the profit part of a win only (D17); stake comes back untouched.
 */
import type { SymbolId } from "./symbols";

export const PAYOUT_MULTIPLIER: Readonly<Record<number, number>> = { 2: 3, 3: 5, 4: 9, 5: 26, 6: 101 };
export const FEE_BPS = 200n;
export const MIN_BET = 1_000_000n; // $1 in base units

/** How many of the six dice show `symbol`. */
export function matches(symbol: SymbolId | number, faces: readonly number[]): number {
  return faces.filter((f) => f === symbol).length;
}

/** Gross total return for a stake at `k` matches (before the fee). */
export function grossReturn(stake: bigint, k: number): bigint {
  return stake * BigInt(PAYOUT_MULTIPLIER[k] ?? 0);
}

/** What the bettor is owed: stake plus 98% of the profit, rounded down (mirrors payout.rs). */
export function owedFor(stake: bigint, k: number): bigint {
  const gross = grossReturn(stake, k);
  if (gross <= stake) return gross;
  const profit = gross - stake;
  return stake + (profit * (10_000n - FEE_BPS)) / 10_000n;
}

export const PAYOUT_ROWS = [2, 3, 4, 5, 6].map((k) => ({ matches: k, multiplier: PAYOUT_MULTIPLIER[k] }));
