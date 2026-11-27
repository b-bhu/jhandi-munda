"use client";
import { useEffect, useMemo, useState } from "react";
import { getClient } from "@/lib/game-client";
import type { Replay } from "@/lib/types";
import { Panel } from "@/components/ui/Panel";
import "@/styles/replay.css";
import { StatStrip } from "./StatStrip";
import { PoolQueueChart } from "./PoolQueueChart";
import { Scrubber } from "./Scrubber";
import { RoundCard } from "./RoundCard";

const SIM = <span className="tag sim">Simulation</span>;

export function ReplayClient() {
  const client = useMemo(() => getClient(), []);
  const [replay, setReplay] = useState<Replay | null>(null);
  const [at, setAt] = useState(1);

  useEffect(() => {
    let live = true;
    client.getReplay().then((r) => { if (!live) return; setReplay(r); setAt(r.rounds.find((x) => x.note)?.index ?? 1); });
    return () => { live = false; };
  }, [client]);

  if (!replay) return <div className="page">Loading the replay…</div>;
  const round = replay.rounds[at - 1];
  return (
    <div className="page">
      <h1>Replay: 50 rounds</h1>
      <p className="dim">A <b>simulation</b> with made-up dice and one simulated player, not real rounds. It shows how a small pool copes when one very big win (six matching dice, 101x) arrives.</p>
      <Panel title="Totals so far" tag={SIM}><StatStrip rounds={replay.rounds} upTo={at} /></Panel>
      <Panel title="Pool and queue" tag={SIM}>
        <PoolQueueChart rounds={replay.rounds} at={at} />
        <Scrubber value={at} max={replay.rounds.length} onChange={setAt} />
        <p className="dim small">Line: the pool&apos;s vault. Red bars: wins the pool could not pay yet.</p>
      </Panel>
      <Panel title={`Round ${at}`} tag={SIM}><RoundCard round={round} /></Panel>
    </div>
  );
}
