import type { SimRound } from "@/lib/types";
import { formatMusd, formatRatio } from "@/lib/format";

/** Totals up to and including the round the scrubber is on (`at` is a 0-based position). */
export function StatStrip({ rounds, at }: { rounds: SimRound[]; at: number }) {
  const seen = rounds.slice(0, at + 1);
  const bets = seen.reduce((a, r) => a + r.bets.length, 0);
  const peakQueue = seen.reduce((a, r) => Math.max(a, r.queue), 0);
  const lowNav = seen.reduce((a, r) => (r.nav < a ? r.nav : a), seen[0].nav);
  const now = seen[seen.length - 1];
  return (
    <div className="stats">
      <div className="stat"><b>{at + 1} / {rounds.length}</b><span>Rounds shown</span></div>
      <div className="stat"><b>{bets}</b><span>Bets of 1 MUSD</span></div>
      <div className="stat"><b>{formatMusd(now.vault)}</b><span>Pool now</span></div>
      <div className="stat"><b>${formatRatio(now.nav, 1_000_000n, 3)}</b><span>Share price now (low ${formatRatio(lowNav, 1_000_000n, 3)})</span></div>
      <div className="stat"><b>{now.queue}</b><span>Wins queued (peak {peakQueue})</span></div>
    </div>
  );
}
