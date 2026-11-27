/*
 * ALL fake numbers live in this file and nowhere else. Everything here is EXAMPLE data for the UI
 * shell (NEXT_PUBLIC_DATA=mock), except ROUND_12, which copies the real devnet round from
 * docs/deploys.md (27 Nov 2026, scripts/devnet-checks-112.mjs --queue-only).
 * Money is bigint base units (6 decimals): 1_000_000n = 1 MUSD.
 */
import type { Bet, FairnessProof, PoolView, Position, QueueItem, SessionView, Snapshot, StakeView } from "./types";
import { owedFor } from "./payouts";

const M = 1_000_000n; // one MUSD

export const ROUND_SECS = 90;
export const LOSS_UNLOCK_SECS = 86_400;
export const WIN_UNLOCK_SECS = 90;
export const TAKE_NOW_BPS = 9_500n; // winners only (D50)
export const MOCK_WALLET = "7Qe2mock1111111111111111111111111111Wlt9";
export const MOCK_SIGNER = "9Kx4mock2222222222222222222222222222Ssn3";

/** Faces of the example round: spade and club show twice, crown and flag once; heart and diamond never. */
export const EXAMPLE_FACES = [2, 2, 4, 4, 5, 6];
export const OPEN_SECONDS_LEFT = 72;
export const EXAMPLE_PLAYERS = 4;
export const ROUND_ID_EXAMPLE = 13;
/** What all players have on each symbol this round (the cloth totals). */
export const SYMBOL_TOTALS: Record<number, bigint> = { 1: 14n * M, 2: 31n * M, 3: 6n * M, 4: 9n * M, 5: 22n * M, 6: 3n * M };

export const WALLET_MUSD = 100n * M;
export const SESSION: SessionView = { active: true, signer: MOCK_SIGNER, cap: 50n * M, spent: 7n * M, expiresAt: 0 };
export const SESSION_SECS_LEFT = 58 * 60;

/** My two bets: a $5 spade (2 dice match, a win) and a $2 heart (no match, a loss). */
export const MY_BETS: Bet[] = [
  { id: "b1", roundId: ROUND_ID_EXAMPLE, symbol: 2, amount: 5n * M, road: "wallet", result: "won", matches: 2, owed: owedFor(5n * M, 2) },
  { id: "b2", roundId: ROUND_ID_EXAMPLE, symbol: 1, amount: 2n * M, road: "wallet", result: "lost", matches: 0 },
];
/** A loss mints MUNDA at 5x inside the first $1,000 of all losses (D49), Wait only after 24 h (D50). */
export const LOSS_MUNDA_PER_MUSD = 5n;

/** vault 1,262.00, owed 12.00, 1,000 shares: NAV = (1,262 - 12) / 1,000 = 1.25 (the pool mock). */
export const POOL: PoolView = {
  vault: 1_262_000_000n,
  owed: 12_000_000n,
  shares: 1_000_000_000n,
  nav: 0n, // computed by the client from vault, owed and shares
  queueLength: 3,
  insurance: 48_250_000n,
};
export const POSITION: Position = { shares: 80n * M, value: 0n, walletMusd: 340n * M, boughtAt: 1_000_000n };
/** The queue sums to the 12.00 the pool owes: 2.96 + 6.40 + 2.64. */
export const QUEUE: QueueItem[] = [
  { id: "q1", label: "Win · round 9", owner: "4Fd8mock3333333333333333333333333333Qa11", amount: 2_960_000n, position: 1, enqueuedAt: 0 },
  { id: "q2", label: "Win · round 10", owner: "BnT6mock4444444444444444444444444444Qb22", amount: 6_400_000n, position: 2, enqueuedAt: 0 },
  { id: "q3", label: "Win · round 11", owner: "9Kx4mock2222222222222222222222222222Ssn3", amount: 2_640_000n, position: 3, enqueuedAt: 0 },
];
export const STAKE: StakeView = {
  staked: 165n * M,
  eligible: 140n * M,
  pending: 1_680_000n,
  lots: [
    { amount: 100n * M, stakedAt: 0 },
    { amount: 40n * M, stakedAt: 0 },
    { amount: 25n * M, stakedAt: 0 },
  ],
  transfersEnabled: false,
  munda: 0n,
  worthThen: 3_300_000n,
  worthNow: 3_800_000n,
};
/** Seconds before "now" that each stake lot started: 3.2 days, 1.1 days, 6 hours (the last is inside the 24 h clock). */
export const STAKE_LOT_AGES = [276_480, 95_040, 21_600];
export const QUEUE_AGES = [420, 300, 95];

/** Daily NAV per share (6-decimal ratio, 1_000_000n = $1.0000), 1,000 shares, so vault = nav x 1,000. */
const NAV_BY_DAY: [string, bigint][] = [
  ["2026-11-21", 1_000_000n], ["2026-11-22", 1_025_000n], ["2026-11-23", 1_012_000n],
  ["2026-11-24", 1_085_000n], ["2026-11-25", 1_140_000n], ["2026-11-26", 1_200_000n],
  ["2026-11-27", 1_250_000n],
];
export const SNAPSHOTS: Snapshot[] = NAV_BY_DAY.map(([day, nav]) => ({ day, nav, vault: nav * 1_000n, shares: 1_000_000_000n }));

/** REAL: devnet round 12, from docs/deploys.md. A 2-match chip win of 2.96 MUSD, queued and paid. */
export const ROUND_12: FairnessProof = {
  roundId: 12,
  faces: [2, 2, 4, 6, 4, 5],
  steps: [
    { label: "Bet placed", detail: "A chip bet is placed (on Solana devnet for this proof round).", tx: "45KEdeQTrCqz9yV6smK8U6dYwg5Xw1s19VWgTexnDSDPRwovZqdhrRNnD15y6pMdXGHiBZqBBWuHCnrDVqRYeqob" },
    { label: "Roll requested", detail: "The program asks MagicBlock’s random number service (VRF) for six dice.", tx: "4hSyiicby5eaYtM8hBzGH5euGPcpyiSQWdqvHht6yZvQ7ZkrGDd27mgy8z8v5iKoo2ZQQzR9SwNwRnqbBFihfcRa" },
    { label: "Dice rolled · VRF callback", detail: "The service answers. The six faces are written onchain.", tx: "5aQ75QwPrQafMvV8sbHgZW69afP1fH4N3HaB2tm9jJAZBH83Turikxv1bmL3iq2s4CvmJ8V2Ma8iMap1aXhyAZtF" },
    { label: "Bets settled", detail: "Each bet is checked against the faces.", tx: "5XFUaLXUZQcRkQy1BFj4ksjhwYn7eBMm5umXBn9AdZyBfiASyP2ChHZhc6mTaPqDUw9nUhNSP4fde7Hijx4J1hSN" },
    { label: "Pool top-up", detail: "A deposit gave the pool cash to pay the queue.", tx: "3AtDWmm9UzmdePSengkQyadbku3gEovGUJtMkiBohcMVVCT87PUuAFafhqKJ7e83Z3NNszwZc3qn1G21aXNNAaM9" },
    { label: "Queued win paid", detail: "The 2.96 MUSD win was paid, first in first out.", tx: "5yX32eVCaGaYvXV2nWPxcjM4wAP7egB7d5GAzq8dgxafh9pm64DpkQasJHe1ouuVxrEnMHaNMuMUCsyiqVrUfJLS" },
    { label: "Round finished", detail: "The round closes. A new one can start.", tx: "5bWST8rVjWo1LLv3WgmWn3x2uJEC7s4bmpbo8KUYTAJ7xr2Gort3vHDpe3rb2FR3w6B9EhpeJ5UxSvfLbpRicSfd" },
  ],
  bets: [{ player: "5qPRTWcUZw1XYGnXqUCCF65TGBgMjk9BfMyz2VBHacm7", symbol: 2, amount: 1n * M, matches: 2, owed: 2_960_000n, road: "chips" }],
  isExample: false,
};

/** Example for any other round id: made-up faces and bets, tagged EXAMPLE on the page. */
export const EXAMPLE_PROOF_BETS: FairnessProof["bets"] = [
  { player: MOCK_WALLET, symbol: 2, amount: 10n * M, matches: 2, owed: owedFor(10n * M, 2), road: "wallet" },
  { player: "4Fd8mock3333333333333333333333333333Qa11", symbol: 4, amount: 5n * M, matches: 0, owed: 0n, road: "wallet" },
  { player: "BnT6mock4444444444444444444444444444Qb22", symbol: 5, amount: 2n * M, matches: 1, owed: 0n, road: "chips" },
];

export const EXAMPLE_PROOF_STEPS: FairnessProof["steps"] = [
  { label: "Bets placed", detail: "Three players bet before the window ended", tx: null },
  { label: "Roll requested", detail: "Example round: no real transaction behind it", tx: null },
  { label: "VRF callback", detail: "Example round: no real transaction behind it", tx: null },
  { label: "Settled", detail: "Example round: no real transaction behind it", tx: null },
];

export const FAUCET_AMOUNT = 100n * M;
