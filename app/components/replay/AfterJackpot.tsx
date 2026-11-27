import type { SimJackpot } from "@/lib/types";
import { formatMusd } from "@/lib/format";

/** What the pool did when the six-match win arrived and could not be paid at once (D30). */
export function AfterJackpot({ jackpot }: { jackpot: SimJackpot }) {
  const rows = [
    ["Before the round settled", jackpot.beforeSettle],
    ["After settle: the win joins the queue", jackpot.afterSettle],
    ["After the queue pass", jackpot.afterRound],
  ] as const;
  return (
    <>
      <p>In round {jackpot.round} all six dice showed the same face (101x). Each 1 MUSD bet on it is owed <b>{formatMusd(jackpot.owed)}</b>, more than the pool could pay at once, so the wins waited in the queue instead of failing the round.</p>
      {rows.map(([label, s]) => (
        <div className="kv" key={label}><span>{label}</span><b>pool {formatMusd(s.vault)} · {s.queue} queued</b></div>
      ))}
    </>
  );
}
