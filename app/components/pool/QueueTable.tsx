"use client";
import type { QueueItem } from "@/lib/types";
import { formatMusd, formatWait, shortKey } from "@/lib/format";
import { useNow } from "@/components/ui/useNow";

/** Wins the pool could not pay at once, paid first in, first out (D30). Nobody jumps the line. */
export function QueueTable({ items }: { items: QueueItem[] }) {
  const now = useNow();
  if (items.length === 0) return <p className="dim">The queue is empty: every win was paid at once.</p>;
  return (
    <div className="scroll-x">
      <table className="t">
        <thead><tr><th>#</th><th>Player</th><th className="num">Owed</th><th className="num">Waiting</th></tr></thead>
        <tbody>
          {items.map((q) => (
            <tr key={q.id}>
              <td>{q.position}</td>
              <td className="mono">{shortKey(q.owner)}</td>
              <td className="num">{formatMusd(q.amount)}</td>
              <td className="num">{now ? formatWait(now - q.enqueuedAt) : "…"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
