"use client";
import { useCallback, useEffect, useMemo, useState } from "react";
import { getClient } from "@/lib/game-client";
import type { Replay } from "@/lib/types";
import { Panel } from "@/components/ui/Panel";
import "@/styles/replay.css";
import { StatStrip } from "./StatStrip";
import { PoolQueueChart } from "./PoolQueueChart";
import { Scrubber } from "./Scrubber";
import { RoundCard } from "./RoundCard";
import { AfterJackpot } from "./AfterJackpot";

const SIM = <span className="tag sim">Simulation</span>;

export function ReplayClient() {
  const client = useMemo(() => getClient(), []);
  const [replay, setReplay] = useState<Replay | null>(null);
  const [at, setAt] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const setPos = useCallback((i: number) => setAt(i), []);

  useEffect(() => {
    let live = true;
    client.getReplay()
      .then((r) => {
        if (!live) return;
        setReplay(r);
        // Open on the jackpot round: that is the story.
        const j = r.jackpot ? r.rounds.findIndex((x) => x.id === r.jackpot!.round) : -1;
        setAt(Math.max(0, j));
      })
      .catch((e) => live && setError(e instanceof Error ? e.message : String(e)));
    return () => { live = false; };
  }, [client]);

  if (error) return <div className="page"><h1>Replay</h1><p className="loss" role="alert">{error}</p></div>;
  if (!replay) return <div className="page">Loading the replay…</div>;
  const round = replay.rounds[at];
  const jackpotAt = replay.jackpot ? replay.rounds.findIndex((x) => x.id === replay.jackpot!.round) : -1;
  return (
    <div className="page">
      <h1>Replay: {replay.rounds.length} rounds</h1>
      <p className="dim">A <b>simulation</b>: the program&apos;s own code, run offline with three made-up players betting 1 MUSD each round (seed {replay.seed}). Not real rounds. It shows how a small pool copes when one very big win (six matching dice, 101x) arrives.</p>
      <Panel title="Totals so far" tag={SIM}><StatStrip rounds={replay.rounds} at={at} /></Panel>
      <Panel title="Pool and queue" tag={SIM}>
        <PoolQueueChart rounds={replay.rounds} at={at} jackpotAt={jackpotAt >= 0 ? jackpotAt : null} />
        <Scrubber value={at} ids={replay.rounds.map((r) => r.id)} onChange={setPos} />
        <p className="dim small">Gold line: the pool&apos;s vault. Red bars: how many wins are waiting in the queue. Dotted gold line: the 101x round.</p>
      </Panel>
      <Panel title={`Round ${round.id}`} tag={SIM}><RoundCard round={round} /></Panel>
      {replay.jackpot && <Panel title="After the jackpot" tag={SIM}><AfterJackpot jackpot={replay.jackpot} /></Panel>}
    </div>
  );
}
