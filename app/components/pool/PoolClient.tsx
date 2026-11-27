"use client";
import { useEffect, useMemo, useState } from "react";
import { getClient } from "@/lib/game-client";
import type { PoolView, Position, QueueItem, Snapshot, StakeView } from "@/lib/types";
import { Panel } from "@/components/ui/Panel";
import "@/styles/pool.css";
import { NavCard } from "./NavCard";
import { MyPosition } from "./MyPosition";
import { DepositWithdraw } from "./DepositWithdraw";
import { QueueTable } from "./QueueTable";
import { SnapshotChart } from "./SnapshotChart";
import { StakeCard } from "./StakeCard";
import { UnstakeButton } from "./UnstakeButton";

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
    client.getQueue().then((q) => live && setQueue(q));
    client.getSnapshots().then((x) => live && setSnaps(x));
    client.getStake().then((x) => live && setStake(x));
    client.getMyPosition().then((p) => live && setPosition(p));
    return () => { live = false; };
  }, [client]);

  return (
    <div className="page">
      <h1>The house pool</h1>
      <p className="dim">Anyone can put test MUSD into the pool and become part of the house. Devnet only, no real money.</p>
      {pool && <Panel title="Pool NAV" example><NavCard pool={pool} /></Panel>}
      {pool && position && (
        <Panel title="My position" example>
          <MyPosition position={position} />
          <DepositWithdraw
            queued={pool.queueLength}
            maxShares={position.shares}
            onDeposit={async (a) => setPosition(await client.deposit(a))}
            onWithdraw={async (s) => setPosition(await client.withdraw(s))}
          />
        </Panel>
      )}
      <Panel title="NAV per day" example><SnapshotChart snapshots={snaps} /><p className="dim small">One snapshot a day. Past NAV says nothing about the future.</p></Panel>
      <Panel title="Payout queue" example><QueueTable items={queue} /></Panel>
      {stake && (
        <Panel title="MUNDA stake" example>
          <StakeCard stake={stake} />
          <UnstakeButton transfersEnabled={stake.transfersEnabled} staked={stake.staked} />
        </Panel>
      )}
    </div>
  );
}
