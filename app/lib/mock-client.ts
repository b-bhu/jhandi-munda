/* The mock GameClient: logic only. Every number it returns comes from mock-data.ts. */
import type { GameClient, TableState } from "./game-client";
import type { Bet, Claim, PoolView, Position, Round, SessionView, StakeView, QueueItem } from "./types";
import * as D from "./mock-data";
import { owedFor } from "./payouts";
import type { Replay } from "./types";

const nowSecs = () => Math.floor(Date.now() / 1000);
const wait = <T,>(v: T) => new Promise<T>((r) => setTimeout(() => r(v), 60));

function navOf(vault: bigint, owed: bigint, shares: bigint): bigint {
  return shares === 0n ? 1_000_000n : ((vault - owed) * 1_000_000n) / shares;
}

export function createMockClient(state: TableState): GameClient {
  const now = nowSecs();
  const rolled = state === "win" || state === "loss" || state === "result";
  let bets: Bet[] = state === "open" || state === "rolling" ? [] : D.MY_BETS.filter((b) => (state === "win" ? b.result === "won" : state === "loss" ? b.result === "lost" : true));
  let session: SessionView = { ...D.SESSION, expiresAt: now + D.SESSION_SECS_LEFT };
  let position: Position = { ...D.POSITION, value: 0n };
  let claims: Claim[] = bets.filter((b) => b.result !== "open").map((b) => claimFor(b, now));
  const pool = (): PoolView => ({ ...D.POOL, nav: navOf(D.POOL.vault, D.POOL.owed, D.POOL.shares) });
  const val = (shares: bigint) => (shares * navOf(D.POOL.vault, D.POOL.owed, D.POOL.shares)) / 1_000_000n;

  const round = (): Round => ({
    id: D.ROUND_ID_EXAMPLE,
    status: rolled ? "rolled" : state === "rolling" ? "rolling" : "open",
    openedAt: now - (D.ROUND_SECS - D.OPEN_SECONDS_LEFT),
    closesAt: now + D.OPEN_SECONDS_LEFT,
    players: D.EXAMPLE_PLAYERS,
    faces: rolled ? D.EXAMPLE_FACES : null,
  });

  return {
    getRound: () => wait(round()),
    subscribeRound: (cb) => { cb(round()); return () => {}; },
    placeBet: async (symbol, amount) => {
      const bet: Bet = { id: `m${bets.length + 1}`, roundId: D.ROUND_ID_EXAMPLE, symbol, amount, road: "wallet", result: "open" };
      bets = [...bets, bet];
      return wait(bet);
    },
    getMyBets: () => wait(bets),
    getMyClaims: () => wait(claims),
    redeemClaim: async (id, how) => {
      const c = claims.find((x) => x.id === id);
      if (!c) throw new Error("no such claim");
      if (how === "takeNow" && c.kind === "loss") throw new Error("A loss claim has Wait only (D50)");
      const done: Claim = { ...c, status: "redeemed" };
      claims = claims.map((x) => (x.id === id ? done : x));
      return wait(done);
    },
    getSession: () => wait(session),
    startSession: async () => (session = { ...D.SESSION, expiresAt: nowSecs() + D.SESSION_SECS_LEFT, spent: 0n }),
    revokeSession: async () => (session = { ...session, active: false, signer: null }),
    getPool: () => wait(pool()),
    getMyPosition: () => wait({ ...position, value: val(position.shares) }),
    deposit: async (amount) => {
      const shares = (amount * 1_000_000n) / navOf(D.POOL.vault, D.POOL.owed, D.POOL.shares);
      position = { ...position, shares: position.shares + shares, walletMusd: position.walletMusd - amount };
      return wait({ ...position, value: val(position.shares) });
    },
    withdraw: async (shares) => {
      position = { ...position, shares: position.shares - shares, walletMusd: position.walletMusd + val(shares) };
      return wait({ ...position, value: val(position.shares) });
    },
    getQueue: () => wait(D.QUEUE.map((q, i): QueueItem => ({ ...q, enqueuedAt: now - D.QUEUE_AGES[i] }))),
    getStake: () => wait<StakeView>({ ...D.STAKE, lots: D.STAKE.lots.map((l, i) => ({ ...l, stakedAt: now - D.STAKE_LOT_AGES[i] })) }),
    getSnapshots: () => wait(D.SNAPSHOTS),
    getFairness: async (roundId) => {
      if (roundId === D.ROUND_12.roundId) return wait(D.ROUND_12);
      const faces = D.EXAMPLE_FACES;
      return wait({ roundId, faces, steps: D.EXAMPLE_PROOF_STEPS, bets: D.EXAMPLE_PROOF_BETS, isExample: true });
    },
    getReplay: loadReplay,
    faucet: () => wait(D.FAUCET_AMOUNT),
  };
}

function claimFor(b: Bet, now: number): Claim {
  if (b.result === "won") {
    const owed = b.owed ?? owedFor(b.amount, b.matches ?? 0);
    return { id: `c-${b.id}`, betId: b.id, kind: "win", asset: "MUSD", amount: owed, unlockAt: now - 10, status: "ready", takeNowPayout: (owed * D.TAKE_NOW_BPS) / 10_000n, road: b.road };
  }
  return { id: `c-${b.id}`, betId: b.id, kind: "loss", asset: "MUNDA", amount: b.amount * D.LOSS_MUNDA_PER_MUSD, unlockAt: now + D.LOSS_UNLOCK_SECS - 120, status: "locked", takeNowPayout: null, road: b.road };
}

/** The 50-round simulation, written by scripts/gen-sim.mjs from the real program. Amounts are strings in the file. */
interface SimFile {
  seed: number; source: string;
  rounds: { id: number; bets: { player: number; face: number }[]; faces: number[]; vault: string; navMicro: number; queue: number; insurance: string; note: string | null }[];
  jackpot: { round: number; owedMusd: string; beforeSettle: SnapFile; afterSettle: SnapFile; afterRound: SnapFile } | null;
}
type SnapFile = { vault: string; queue: number; insurance: string };

async function loadReplay(): Promise<Replay> {
  const res = await fetch("/sim.json");
  if (!res.ok) throw new Error("The simulation file /sim.json is missing: run node scripts/gen-sim.mjs");
  const f = (await res.json()) as SimFile;
  const snap = (x: SnapFile) => ({ vault: BigInt(x.vault), queue: x.queue, insurance: BigInt(x.insurance) });
  return {
    isSimulation: true, seed: f.seed, source: f.source,
    rounds: f.rounds.map((r) => ({ id: r.id, bets: r.bets, faces: r.faces, vault: BigInt(r.vault), nav: BigInt(r.navMicro), queue: r.queue, insurance: BigInt(r.insurance), note: r.note })),
    jackpot: f.jackpot && {
      round: f.jackpot.round, owed: BigInt(Math.round(Number(f.jackpot.owedMusd) * 1e6)),
      beforeSettle: snap(f.jackpot.beforeSettle), afterSettle: snap(f.jackpot.afterSettle), afterRound: snap(f.jackpot.afterRound),
    },
  };
}
