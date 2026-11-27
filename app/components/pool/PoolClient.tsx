"use client";
import { useEffect, useMemo, useState } from "react";
import { getClient } from "@/lib/game-client";
import type { PoolView, Position, QueueItem, Snapshot, StakeView } from "@/lib/types";
import { SectionRule } from "@/components/ui/SectionRule";
import { ExampleTag } from "@/components/shell/ExampleTag";
import "@/styles/app-mock.css";
import "@/styles/app-extra.css";
import "@/styles/pool.css";
import { NavCard } from "./NavCard";
import { MyPosition } from "./MyPosition";
import { DepositWithdraw } from "./DepositWithdraw";
import { QueueTable } from "./QueueTable";
import { StakeCard } from "./StakeCard";
import { SnapshotChart } from "./SnapshotChart";

export function PoolClient() {
  const client = useMemo(() => getClient(), []);
  const [pool, setPool] = useState<PoolView | null>(null);
  const [position, setPosition] = useState<Position | null>(null);
  const [queue, setQueue] = useState<QueueItem[]>([]);
  const [snaps, setSnaps] = useState<Snapshot[]>([]);
  const [stake, setStake] = useState<StakeView | null>(null);

  useEffect(() => {
    let live = true;
    client.getPool().then((p) => live && setPool(p));
    client.getMyPosition().then((p) => live && setPosition(p));
    client.getQueue().then((q) => live && setQueue(q));
    client.getSnapshots().then((x) => live && setSnaps(x));
    client.getStake().then((x) => live && setStake(x));
    return () => { live = false; };
  }, [client]);

  return (
    <div className="app">
      <div className="ph">
        <SectionRule n="POOL">The house <ExampleTag /></SectionRule>
        <h1>Anyone can be <em>the house.</em></h1>
        <p className="lead">Players bet against this shared pool. Deposit test MUSD, get shares, and watch what they are worth. Pool value can go down as well as up. Devnet only, no real money.</p>
      </div>
      {pool && position && (
        <>
          <section className="state" style={{ marginTop: 32 }}>
            <div className="grid2 even">
              <NavCard pool={pool} />
              <DepositWithdraw nav={pool.nav} queued={pool.queueLength} walletMusd={position.walletMusd} shares={position.shares}
                onDeposit={async (a) => setPosition(await client.deposit(a))} onWithdraw={async (s) => setPosition(await client.withdraw(s))} />
            </div>
          </section>
          <section className="state" style={{ marginTop: 24 }}>
            <div className="grid2 even"><MyPosition position={position} nav={pool.nav} /><QueueTable items={queue} /></div>
          </section>
          {stake && (
            <section className="state" style={{ marginTop: 24 }}>
              <div className="grid2 even"><StakeCard stake={stake} /><SnapshotChart snapshots={snaps} /></div>
            </section>
          )}
        </>
      )}
    </div>
  );
}
