/* Shapes the pages use. Money is bigint base units (6 decimals). No chain types here. */
import type { SymbolId } from "./symbols";

export type RoundStatus = "idle" | "open" | "rolling" | "rolled";

export interface Round {
  id: number;
  status: RoundStatus;
  openedAt: number; // unix seconds, 0 when idle
  closesAt: number; // openedAt + 90
  players: number;
  /** Everyone's stake on each symbol (the cloth totals), base units. */
  totals: Partial<Record<SymbolId, bigint>>;
  faces: number[] | null; // six faces 1..6 once rolled
}

export type BetRoad = "wallet" | "chips";
export type BetResult = "open" | "won" | "lost";

export interface Bet {
  id: string;
  roundId: number;
  symbol: SymbolId;
  amount: bigint;
  road: BetRoad;
  result: BetResult;
  matches?: number;
  owed?: bigint; // what a win pays (stake + 98% of profit)
}

export type ClaimKind = "win" | "loss";
export type ClaimStatus = "locked" | "ready" | "queued" | "redeemed";

export interface Claim {
  id: string;
  betId: string;
  kind: ClaimKind;
  asset: "MUSD" | "MUNDA";
  amount: bigint; // MUSD owed (win) or MUNDA minted (loss)
  unlockAt: number;
  status: ClaimStatus;
  /** Win only (D50): 95% now from insurance. A loss claim never has it. */
  takeNowPayout: bigint | null;
  road: BetRoad;
}

export interface SessionView {
  active: boolean;
  signer: string | null;
  cap: bigint;
  spent: bigint;
  expiresAt: number;
}

export interface PoolView {
  vault: bigint;
  owed: bigint; // queued wins the pool still owes
  shares: bigint;
  /** NAV per share = (vault - owed) / shares, as a base-unit ratio (1_000_000 = $1.00). */
  nav: bigint;
  queueLength: number;
  insurance: bigint;
}

export interface Position {
  shares: bigint;
  value: bigint; // shares x NAV
  walletMusd: bigint;
}

export interface QueueItem {
  id: string;
  owner: string;
  amount: bigint;
  position: number; // 1 = next
  enqueuedAt: number;
}

export interface StakeLot {
  amount: bigint;
  stakedAt: number;
}

export interface StakeView {
  staked: bigint;
  eligible: bigint;
  pending: bigint; // MUSD rewards credited, not yet claimed
  lots: StakeLot[];
  /** Admin switch (D46). While false, unstake is refused and loss MUNDA stays staked (D47). */
  transfersEnabled: boolean;
  munda: bigint; // loose MUNDA in the wallet (none while transfers are off)
}

export interface Snapshot {
  day: string; // YYYY-MM-DD
  nav: bigint;
  vault: bigint;
  shares: bigint;
}

export interface FairnessBet {
  player: string;
  symbol: SymbolId;
  amount: bigint;
  matches: number;
  owed: bigint;
  road: BetRoad;
}

export interface ProofStep {
  label: string;
  detail: string;
  tx: string | null; // a devnet transaction signature, null when there is none to show
}

export interface FairnessProof {
  roundId: number;
  faces: number[];
  steps: ProofStep[];
  bets: FairnessBet[];
  /** True for made-up data. False only for a real devnet round copied from docs/deploys.md. */
  isExample: boolean;
}

export interface SimBet { player: number; face: number } // face 1..6, a 1 MUSD chip bet

/** One round of the program simulation (scripts/sim-rounds.sh run in LiteSVM, saved as public/sim.json). */
export interface SimRound {
  id: number;
  bets: SimBet[];
  faces: number[];
  vault: bigint; // pool vault after the round
  nav: bigint; // pool NAV per share, 6-decimal ratio
  queue: number; // wins waiting in the payout queue
  insurance: bigint;
  note: string | null;
}

export interface PoolSnap { vault: bigint; queue: number; insurance: bigint }

/** The injected six-match round: a 99 MUSD win the pool could not pay, so it queued. */
export interface SimJackpot {
  round: number;
  owed: bigint;
  beforeSettle: PoolSnap;
  afterSettle: PoolSnap;
  afterRound: PoolSnap;
}

export interface Replay {
  rounds: SimRound[];
  jackpot: SimJackpot | null;
  seed: number;
  source: string;
  /** Always true: a replay is a SIMULATION, never real rounds. */
  isSimulation: true;
}
