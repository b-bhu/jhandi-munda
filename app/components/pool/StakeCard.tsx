"use client";
import type { StakeView } from "@/lib/types";
import { formatAmount, formatAgo } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { ExampleTag } from "@/components/shell/ExampleTag";
import { useNow } from "@/components/ui/useNow";
import { UnstakeButton } from "./UnstakeButton";

const LOCK_SECS = 86_400; // a stake lot earns after 24 hours (D29)

export function StakeCard({ stake }: { stake: StakeView }) {
  const now = useNow();
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>My MUNDA stake</span><ExampleTag /></div>
      <div className="bigval" style={{ fontSize: 40 }}>{formatAmount(stake.staked, 0)} <small>MUNDA staked</small></div>
      <div style={{ marginTop: 14 }}>
        {stake.lots.map((l, i) => {
          const age = now ? now - l.stakedAt : null;
          const left = age === null ? null : LOCK_SECS - age;
          return (
            <div className="lot" key={i}>
              <span className="t">Lot {i + 1}</span>
              <span>{age === null ? "…" : formatAgo(age)} · {formatAmount(l.amount, 0)} MUNDA</span>
              {left === null ? <span className="pill2">…</span> : left > 0 ? <span className="pill2 o">{Math.ceil(left / 3600)} h lock left</span> : <span className="pill2 g">earning fees</span>}
            </div>
          );
        })}
      </div>
      <div className="kv" style={{ marginTop: 6 }}><span>Worth when you got it</span><b>${formatAmount(stake.worthThen)}</b></div>
      <div className="kv"><span>Worth now</span><b className="g">${formatAmount(stake.worthNow)}</b></div>
      <div className="lock" id="unstake-why">
        <b>{stake.transfersEnabled ? "Open" : "Locked"}</b>
        <span>
          {stake.transfersEnabled ? (
            <>Unstaking returns MUNDA to your wallet. New MUNDA earns nothing for its first 24 hours (D29).</>
          ) : (
            <><b style={{ color: "var(--t1)" }}>Unstake is off.</b> The admin “transfers” switch is not on yet (D46, D47), so MUNDA stays in your stake. New MUNDA earns nothing for its first 24 hours (D29). This is not a refund of lost money.</>
          )}
        </span>
      </div>
      <UnstakeButton transfersEnabled={stake.transfersEnabled} staked={stake.staked} />
    </CornerBox>
  );
}
