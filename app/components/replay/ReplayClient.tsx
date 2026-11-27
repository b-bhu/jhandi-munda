"use client";
import { useCallback, useEffect, useMemo, useState } from "react";
import { getClient } from "@/lib/game-client";
import type { Replay } from "@/lib/types";
import { SectionRule } from "@/components/ui/SectionRule";
import { CornerBox } from "@/components/ui/CornerBox";
import "@/styles/app-mock.css";
import "@/styles/app-extra.css";
import "@/styles/pool.css";
import "@/styles/replay.css";
import { StatStrip } from "./StatStrip";
import { PoolQueueChart } from "./PoolQueueChart";
import { Scrubber } from "./Scrubber";
import { RoundPanel } from "./RoundPanel";
import { AfterJackpot } from "./AfterJackpot";

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
        const j = r.jackpot ? r.rounds.findIndex((x) => x.id === r.jackpot!.round) : -1; // open on the story
        setAt(Math.max(0, j));
      })
      .catch((e) => live && setError(e instanceof Error ? e.message : String(e)));
    return () => { live = false; };
  }, [client]);

  if (error) return <div className="app"><p className="loss" role="alert">{error}</p></div>;
  if (!replay) return <div className="app"><p className="small" role="status">Loading the replay…</p></div>;
  const round = replay.rounds[at];
  return (
    <div className="app">
      <div className="ph">
        <SectionRule n="REPLAY">{replay.rounds.length}-round simulation <span className="mock">Simulation</span></SectionRule>
        <h1>The pool takes a <em>101x hit.</em></h1>
        <p className="lead">A replay of {replay.rounds.length} simulated rounds: the program’s own code, run offline with three made-up players betting 1 MUSD each (seed {replay.seed}). One round shows all six dice the same. The pool pays what it can, the rest waits in the queue, and the table keeps taking bets. This is a simulation, not live results.</p>
      </div>
      <section className="state" style={{ marginTop: 28 }}><StatStrip replay={replay} /></section>
      <section className="state" style={{ marginTop: 24 }}>
        <div className="grid2" data-wide>
          <CornerBox className="panel">
            <div className="ptitle"><span>Pool cash and queue by round</span><span className="mock">Simulation</span></div>
            <PoolQueueChart replay={replay} at={at} />
            <div className="legend"><span><i style={{ background: "var(--gold)" }} />Pool cash (MUSD)</span><span><i style={{ background: "var(--tilak)" }} />Owed in queue (MUSD)</span><span><i style={{ background: "var(--green)" }} />Jackpot</span></div>
            <Scrubber value={at} ids={replay.rounds.map((r) => r.id)} onChange={setPos} />
          </CornerBox>
          <div className="stack">
            <RoundPanel round={round} replay={replay} />
            <AfterJackpot replay={replay} />
          </div>
        </div>
      </section>
      <section className="state" style={{ marginTop: 24 }}><p className="note">Simulation data, not live results. Regenerate it with <span className="mono">node scripts/gen-sim.mjs</span>; the numbers come from the program itself.</p></section>
    </div>
  );
}
