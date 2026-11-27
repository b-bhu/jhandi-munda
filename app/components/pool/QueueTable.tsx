import type { QueueItem } from "@/lib/types";
import { formatAmount } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { ExampleTag } from "@/components/shell/ExampleTag";

/** Wins the pool could not pay at once, paid first in, first out (D30). Nobody jumps the line. */
export function QueueTable({ items }: { items: QueueItem[] }) {
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>Payout queue · first in, first out</span><ExampleTag /></div>
      {items.length === 0 ? <p className="small">The queue is empty: every win was paid at once.</p> : (
        <div className="scroll-x" tabIndex={0} role="region" aria-label="Payout queue table, scrolls sideways on small screens">
          <table className="tbl">
            <thead><tr><th>#</th><th>What</th><th>Owed</th><th>Status</th></tr></thead>
            <tbody>
              {items.map((q) => (
                <tr key={q.id}>
                  <td className="m">#{q.position}</td>
                  <td>{q.label}</td>
                  <td className="m">{formatAmount(q.amount)} MUSD</td>
                  <td><span className={`pill2${q.position === 1 ? " o" : ""}`}>{q.position === 1 ? "Next in line" : `Behind #${q.position - 1}`}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="note">When the pool can’t pay a win, it waits here and is paid from future losses. No skipping (D30). The queue holds up to 100 wins.</p>
    </CornerBox>
  );
}
