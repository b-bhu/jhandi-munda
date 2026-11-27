import type { Replay } from "@/lib/types";
import { formatMusd } from "@/lib/format";
import { matches } from "@/lib/payouts";

/** Four numbers for the whole replay (not the scrubber position). */
export function StatStrip({ replay }: { replay: Replay }) {
  const { rounds, jackpot } = replay;
  const six = rounds.filter((r) => r.bets.some((b) => matches(b.face, r.faces) === 6)).length;
  const low = rounds.reduce((a, r) => (r.vault < a ? r.vault : a), rounds[0].vault);
  const last = rounds[rounds.length - 1];
  return (
    <div className="stats">
      <div><b>{rounds.length}</b><span>rounds replayed</span></div>
      <div><b>{six}</b><span>{six === 1 ? "101x jackpot" : "101x jackpots"}{jackpot ? ` (round ${jackpot.round})` : ""}</span></div>
      <div><b>{formatMusd(low, 2).replace(" MUSD", "")}</b><span>pool cash low point (MUSD)</span></div>
      <div><b>{last.queue}</b><span>wins still queued after round {last.id}</span></div>
    </div>
  );
}
