"use client";
import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";
import { getClient, isMock, type TableState } from "@/lib/game-client";
import type { Bet, Round, SessionView } from "@/lib/types";
import type { SymbolId } from "@/lib/symbols";
import { Panel } from "@/components/ui/Panel";
import "@/styles/table.css";
import { Board } from "./Board";
import { RoundTimer } from "./RoundTimer";
import { BetPanel } from "./BetPanel";
import { SessionBadge } from "./SessionBadge";
import { MyBets } from "./MyBets";

const STATES: TableState[] = ["open", "rolling", "win", "loss", "result"];

/** ?state= picks the look of the mock table for screenshots. It is ignored against the real program. */
function useTableState(): TableState | undefined {
  const raw = useSearchParams().get("state");
  return isMock() && STATES.includes(raw as TableState) ? (raw as TableState) : undefined;
}

export function PlayClient() {
  const state = useTableState();
  const client = useMemo(() => getClient(state), [state]);
  const [round, setRound] = useState<Round | null>(null);
  const [bets, setBets] = useState<Bet[]>([]);
  const [selected, setSelected] = useState<SymbolId | null>(null);
  const [session, setSession] = useState<SessionView | null>(null);

  useEffect(() => {
    let live = true;
    const off = client.subscribeRound((r) => live && setRound(r));
    client.getMyBets().then((b) => live && setBets(b));
    client.getSession().then((x) => live && setSession(x));
    return () => { live = false; off(); };
  }, [client]);

  const staked: Partial<Record<SymbolId, bigint>> = {};
  for (const b of bets) staked[b.symbol] = (staked[b.symbol] ?? 0n) + b.amount;

  const place = async (symbol: SymbolId, amount: bigint) => {
    const bet = await client.placeBet(symbol, amount);
    setBets((b) => [...b, bet]);
  };

  return (
    <div className="page">
      <h1>The table</h1>
      {round && (
        <Panel example>
          <RoundTimer round={round} />
          <Board selected={selected} onSelect={setSelected} disabled={round.status !== "open"} staked={staked} faces={round.faces} />
        </Panel>
      )}
      <div className="grid-2">
        <Panel title="Place a bet" example>
          <SessionBadge session={session} onStart={() => client.startSession().then(setSession)} onRevoke={() => client.revokeSession().then(setSession)} />
          <BetPanel symbol={selected} open={round?.status === "open"} onPlace={place} />
        </Panel>
        <Panel title="My bets" example>
          <MyBets bets={bets} />
        </Panel>
      </div>
    </div>
  );
}
