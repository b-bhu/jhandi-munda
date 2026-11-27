/*
 * The only door between the pages and the game. Pages import this file and never touch
 * @solana/web3.js or the IDL. NEXT_PUBLIC_DATA=mock (default) serves fake data from mock-data.ts;
 * NEXT_PUBLIC_DATA=program will serve the chain through program-client.ts, the ONLY file that
 * changes when we go live (it does not exist yet).
 */
import type {
  Bet, Claim, FairnessProof, PoolView, Position, QueueItem, Replay, Round, SessionView, Snapshot, StakeView,
} from "./types";
import type { SymbolId } from "./symbols";
import { createMockClient } from "./mock-client";

export type DataMode = "mock" | "program";
export const dataMode = (): DataMode => (process.env.NEXT_PUBLIC_DATA === "program" ? "program" : "mock");
export const isMock = () => dataMode() === "mock";

/** Which look the mock table shows, for screenshots (?state= in mock mode only). */
export type TableState = "open" | "rolling" | "win" | "loss" | "result";

export interface GameClient {
  getRound(): Promise<Round>;
  subscribeRound(onChange: (round: Round) => void): () => void;
  placeBet(symbol: SymbolId, amount: bigint): Promise<Bet>;
  getMyBets(): Promise<Bet[]>;
  getMyClaims(): Promise<Claim[]>;
  redeemClaim(id: string, how: "wait" | "takeNow"): Promise<Claim>;
  getSession(): Promise<SessionView>;
  startSession(): Promise<SessionView>;
  revokeSession(): Promise<SessionView>;
  getPool(): Promise<PoolView>; // nav = (vault - owed) / shares
  getMyPosition(): Promise<Position>;
  deposit(amount: bigint): Promise<Position>;
  withdraw(shares: bigint): Promise<Position>;
  getQueue(): Promise<QueueItem[]>;
  getStake(): Promise<StakeView>;
  getSnapshots(): Promise<Snapshot[]>;
  getFairness(roundId: number): Promise<FairnessProof>;
  getReplay(): Promise<Replay>;
  faucet(): Promise<bigint>;
}

let cached: GameClient | null = null;

/** Picks the client by the flag. `state` only matters in mock mode (screenshots). */
export function getClient(state?: TableState): GameClient {
  if (dataMode() === "program") {
    throw new Error("NEXT_PUBLIC_DATA=program: lib/program-client.ts is not built yet. Use mock.");
  }
  if (state) return createMockClient(state);
  return (cached ??= createMockClient("open"));
}
