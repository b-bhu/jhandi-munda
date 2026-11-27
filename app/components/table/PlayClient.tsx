"use client";
import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";
import { getClient, isMock, type TableState } from "@/lib/game-client";
import type { Bet, Round } from "@/lib/types";
import type { SymbolId } from "@/lib/symbols";
import { Panel } from "@/components/ui/Panel";
import "@/styles/table.css";
import { Board } from "./Board";
import { RoundTimer } from "./RoundTimer";

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

  useEffect(() => {
    let live = true;
    const off = client.subscribeRound((r) => live && setRound(r));
    client.getMyBets().then((b) => live && setBets(b));
    return () => { live = false; off(); };
  }, [client]);

  const staked: Partial<Record<SymbolId, bigint>> = {};
  for (const b of bets) staked[b.symbol] = (staked[b.symbol] ?? 0n) + b.amount;

  return (
    <div className="page">
      <h1>The table</h1>
      {round && (
        <Panel example>
          <RoundTimer round={round} />
          <Board selected={selected} onSelect={setSelected} disabled={round.status !== "open"} staked={staked} faces={round.faces} />
        </Panel>
      )}
    </div>
  );
}
