"use client";
import type { StakeView } from "@/lib/types";
import { formatAmount, formatMusd, formatWait } from "@/lib/format";
import { useNow } from "@/components/ui/useNow";

const LOCK_SECS = 86_400; // a stake lot earns after 24 hours (D29)

export function StakeCard({ stake }: { stake: StakeView }) {
  const now = useNow();
  return (
    <>
      <div className="stats">
        <div className="stat"><b>{formatAmount(stake.staked)}</b><span>MUNDA staked</span></div>
        <div className="stat"><b>{formatAmount(stake.eligible)}</b><span>Earning now (staked 24 h+)</span></div>
        <div className="stat"><b>{formatMusd(stake.pending)}</b><span>Rewards credited</span></div>
      </div>
      <div className="scroll-x pool-note">
        <table className="t">
          <thead><tr><th>Lot</th><th className="num">MUNDA</th><th>Status</th></tr></thead>
          <tbody>
            {stake.lots.map((l, i) => {
              const left = l.stakedAt + LOCK_SECS - now;
              return (
                <tr key={i}>
                  <td>{i + 1}</td>
                  <td className="num">{formatAmount(l.amount)}</td>
                  <td>{!now ? "…" : left > 0 ? `earns in ${formatWait(left)}` : "earning"}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
      <p className="dim small pool-note">Stakers share 70% of the fees and half of any pool profit above the overflow target. Rewards are not a promise.</p>
    </>
  );
}
