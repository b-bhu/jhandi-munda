import type { SimRound } from "@/lib/types";
import { formatMusd } from "@/lib/format";

export function StatStrip({ rounds, upTo }: { rounds: SimRound[]; upTo: number }) {
  const seen = rounds.slice(0, upTo);
  const wagered = seen.reduce((a, r) => a + r.stake, 0n);
  const paid = seen.reduce((a, r) => a + r.owed - r.queueOwed, 0n);
  const biggest = seen.reduce((a, r) => (r.owed > a ? r.owed : a), 0n);
  const queued = seen.reduce((a, r) => a + r.queueOwed, 0n);
  return (
    <div className="stats">
      <div className="stat"><b>{upTo} / {rounds.length}</b><span>Rounds</span></div>
      <div className="stat"><b>{formatMusd(wagered, 0)}</b><span>Wagered</span></div>
      <div className="stat"><b>{formatMusd(paid)}</b><span>Paid out</span></div>
      <div className="stat"><b>{formatMusd(biggest)}</b><span>Biggest win</span></div>
      <div className="stat"><b>{formatMusd(queued)}</b><span>Still queued</span></div>
    </div>
  );
}
