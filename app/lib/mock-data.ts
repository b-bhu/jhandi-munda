/*
 * ALL fake numbers live in this file and nowhere else. Everything here is EXAMPLE data for the UI
 * shell (NEXT_PUBLIC_DATA=mock), except ROUND_12, which copies the real devnet round from
 * docs/deploys.md (27 Nov 2026, scripts/devnet-checks-112.mjs --queue-only).
 * Money is bigint base units (6 decimals): 1_000_000n = 1 MUSD.
 */
import type { Bet, FairnessProof, PoolView, Position, QueueItem, Replay, SessionView, SimRound, Snapshot, StakeView } from "./types";
import { owedFor } from "./payouts";

const M = 1_000_000n; // one MUSD

export const ROUND_SECS = 90;
export const LOSS_UNLOCK_SECS = 86_400;
export const WIN_UNLOCK_SECS = 90;
export const TAKE_NOW_BPS = 9_500n; // winners only (D50)
export const MOCK_WALLET = "7Qe2mock1111111111111111111111111111Wlt9";
export const MOCK_SIGNER = "9Kx4mock2222222222222222222222222222Ssn3";

/** Faces of the example round: spade shows twice (a 2-match win); club never shows (a loss). */
export const EXAMPLE_FACES = [2, 2, 5, 6, 1, 3];
export const OPEN_SECONDS_LEFT = 68;
export const EXAMPLE_PLAYERS = 4;
export const ROUND_ID_EXAMPLE = 21;

export const WALLET_MUSD = 100n * M;
export const SESSION: SessionView = { active: true, signer: MOCK_SIGNER, cap: 15n * M, spent: 4n * M, expiresAt: 0 };
export const SESSION_SECS_LEFT = 2_460;

/** Bets of the example result state: a $10 spade (2 matches, a win) and a $5 club (no match, a loss). */
export const MY_BETS: Bet[] = [
  { id: "b1", roundId: ROUND_ID_EXAMPLE, symbol: 2, amount: 10n * M, road: "wallet", result: "won", matches: 2, owed: owedFor(10n * M, 2) },
  { id: "b2", roundId: ROUND_ID_EXAMPLE, symbol: 4, amount: 5n * M, road: "wallet", result: "lost", matches: 0 },
];
/** A loss mints MUNDA at 5x inside the first $1,000 of all losses (D49), Wait only after 24 h (D50). */
export const LOSS_MUNDA_PER_MUSD = 5n;

export const POOL: PoolView = {
  vault: 1_042_500_000n,
  owed: 29_600_000n,
  shares: 1_000_000_000n,
  nav: 0n, // computed by the client from vault, owed and shares
  queueLength: 2,
  insurance: 48_250_000n,
};
export const POSITION: Position = { shares: 250_000_000n, value: 0n, walletMusd: 100n * M };
export const QUEUE: QueueItem[] = [
  { id: "q1", owner: "4Fd8mock3333333333333333333333333333Qa11", amount: 29_600_000n, position: 1, enqueuedAt: 0 },
  { id: "q2", owner: "BnT6mock4444444444444444444444444444Qb22", amount: 5_920_000n, position: 2, enqueuedAt: 0 },
];
export const STAKE: StakeView = {
  staked: 500n * M,
  eligible: 350n * M,
  pending: 1_680_000n,
  lots: [
    { amount: 350n * M, stakedAt: 0 },
    { amount: 150n * M, stakedAt: 0 },
  ],
  transfersEnabled: false,
  munda: 0n,
};
/** Seconds before "now" that each stake lot started (lot 2 is still inside the 24 h clock). */
export const STAKE_LOT_AGES = [200_000, 3_600];
export const QUEUE_AGES = [420, 95];

/** Daily NAV per share (6-decimal ratio, 1_000_000n = $1.0000), 1,000 shares, so vault = nav x 1,000. */
const NAV_BY_DAY: [string, bigint][] = [
  ["2026-11-21", 1_000_000n], ["2026-11-22", 1_004_200n], ["2026-11-23", 998_900n],
  ["2026-11-24", 1_011_500n], ["2026-11-25", 1_019_800n], ["2026-11-26", 1_016_300n],
  ["2026-11-27", 1_029_900n],
];
export const SNAPSHOTS: Snapshot[] = NAV_BY_DAY.map(([day, nav]) => ({ day, nav, vault: nav * 1_000n, shares: 1_000_000_000n }));

/** REAL: devnet round 12, from docs/deploys.md. A 2-match chip win of 2.96 MUSD, queued and paid. */
export const ROUND_12: FairnessProof = {
  roundId: 12,
  faces: [2, 2, 4, 6, 4, 5],
  steps: [
    { label: "Bet placed", detail: "1 MUSD of chips on spade (place_bet_chips)", tx: "45KEdeQTrCqz9yV6smK8U6dYwg5Xw1s19VWgTexnDSDPRwovZqdhrRNnD15y6pMdXGHiBZqBBWuHCnrDVqRYeqob" },
    { label: "Roll requested", detail: "The 90-second window ended; anyone can ask MagicBlock VRF for randomness (request_roll)", tx: "4hSyiicby5eaYtM8hBzGH5euGPcpyiSQWdqvHht6yZvQ7ZkrGDd27mgy8z8v5iKoo2ZQQzR9SwNwRnqbBFihfcRa" },
    { label: "VRF callback", detail: "The oracle's randomness is checked and written as six faces (face = byte % 6 + 1)", tx: "5aQ75QwPrQafMvV8sbHgZW69afP1fH4N3HaB2tm9jJAZBH83Turikxv1bmL3iq2s4CvmJ8V2Ma8iMap1aXhyAZtF" },
    { label: "Settled", detail: "Stakes move to the pool; the 2-match win is owed 2.96 MUSD (settle_chip_bets)", tx: "5XFUaLXUZQcRkQy1BFj4ksjhwYn7eBMm5umXBn9AdZyBfiASyP2ChHZhc6mTaPqDUw9nUhNSP4fde7Hijx4J1hSN" },
    { label: "Win paid", detail: "The win was queued first, then paid out of the pool (pay_queue)", tx: "5yX32eVCaGaYvXV2nWPxcjM4wAP7egB7d5GAzq8dgxafh9pm64DpkQasJHe1ouuVxrEnMHaNMuMUCsyiqVrUfJLS" },
    { label: "Round finished", detail: "The round goes idle for the next one (finish_round)", tx: "5bWST8rVjWo1LLv3WgmWn3x2uJEC7s4bmpbo8KUYTAJ7xr2Gort3vHDpe3rb2FR3w6B9EhpeJ5UxSvfLbpRicSfd" },
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

/* The 50-round replay (SIMULATION): a seeded fixture, not real play. See buildReplay below. */
export const REPLAY_SEED = 20261127;
export const REPLAY_ROUNDS = 50;
export const REPLAY_START_VAULT = 100n * M;
export const REPLAY_STAKE = 5n * M;
export const REPLAY_FORCED: Record<number, number[]> = { 37: [3, 3, 3, 3, 3, 3] }; // the 101x story

export function buildReplay(): Replay {
  let s = REPLAY_SEED;
  const rnd = () => ((s = (s * 1_103_515_245 + 12_345) & 0x7fffffff), s);
  let vault = REPLAY_START_VAULT;
  const rounds: SimRound[] = [];
  for (let i = 1; i <= REPLAY_ROUNDS; i++) {
    const faces = REPLAY_FORCED[i] ?? Array.from({ length: 6 }, () => (rnd() % 6) + 1);
    const symbol = REPLAY_FORCED[i] ? 3 : (rnd() % 6) + 1;
    const k = faces.filter((f) => f === symbol).length;
    const owed = owedFor(REPLAY_STAKE, k);
    vault += REPLAY_STAKE;
    const paid = owed <= vault ? owed : 0n;
    vault -= paid;
    rounds.push({ index: i, faces, stake: REPLAY_STAKE, matches: k, owed, poolVault: vault, queueOwed: owed - paid, note: k === 6 ? "6 matches: 101x" : undefined });
  }
  return { rounds, isSimulation: true };
}

export const FAUCET_AMOUNT = 100n * M;
