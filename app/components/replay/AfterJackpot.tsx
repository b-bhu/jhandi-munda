import type { Replay } from "@/lib/types";
import { formatMusd } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";

/** What the queue looked like in the rounds after the 101x hit: the table kept taking bets. */
export function AfterJackpot({ replay }: { replay: Replay }) {
  const { jackpot, rounds } = replay;
  if (!jackpot) return null;
  const j = rounds.findIndex((r) => r.id === jackpot.round);
  const picks = [j + 1, j + 4, rounds.length - 1].filter((i, n, a) => i < rounds.length && a.indexOf(i) === n);
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>After the jackpot</span><span className="mock">Simulation</span></div>
      <p className="small" style={{ marginBottom: 8 }}>Round {jackpot.round}: each 1 MUSD bet on the six matching face is owed {formatMusd(jackpot.owed)}. The pool could not pay at once, so {jackpot.afterSettle.queue - jackpot.beforeSettle.queue} wins joined the queue instead of failing the round.</p>
      {picks.map((i) => (
        <div className="kv" key={i}><span>Round {rounds[i].id}</span><b>table open · queue owes {formatMusd(rounds[i].queueOwed)} ({rounds[i].queue} wins)</b></div>
      ))}
    </CornerBox>
  );
}
