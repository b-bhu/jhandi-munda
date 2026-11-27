import type { Snapshot } from "@/lib/types";
import { formatRatio } from "@/lib/format";

/* Daily NAV per share as a plain SVG line. No chart library; the data is also in a table for
   screen readers. NAV is a bigint ratio (1_000_000n = $1.0000); the plot uses Number only to draw. */
const W = 600, H = 220, PAD = { l: 52, r: 12, t: 12, b: 28 };

export function SnapshotChart({ snapshots }: { snapshots: Snapshot[] }) {
  if (snapshots.length < 2) return <p className="dim">Not enough daily snapshots yet.</p>;
  const navs = snapshots.map((s) => Number(s.nav));
  const lo = Math.min(...navs), hi = Math.max(...navs);
  const span = hi - lo || 1;
  const x = (i: number) => PAD.l + (i * (W - PAD.l - PAD.r)) / (snapshots.length - 1);
  const y = (v: number) => PAD.t + (1 - (v - lo) / span) * (H - PAD.t - PAD.b);
  const line = navs.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join(" ");
  const ticks = [lo, lo + span / 2, hi];
  return (
    <>
      <svg className="chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Pool share price, one point a day">
        {ticks.map((t) => (
          <g key={t}>
            <line x1={PAD.l} x2={W - PAD.r} y1={y(t)} y2={y(t)} stroke="var(--line)" strokeDasharray="3 4" />
            <text x={PAD.l - 6} y={y(t) + 4} textAnchor="end" fontSize="11" fill="var(--ink-dim)">{formatRatio(BigInt(Math.round(t)), 1_000_000n, 4)}</text>
          </g>
        ))}
        <path d={line} fill="none" stroke="var(--brass)" strokeWidth="2.5" />
        {navs.map((v, i) => <circle key={i} cx={x(i)} cy={y(v)} r="3.5" fill="var(--brass)" />)}
        {snapshots.map((s, i) => (i === 0 || i === snapshots.length - 1 ? (
          <text key={s.day} x={x(i)} y={H - 8} textAnchor={i === 0 ? "start" : "end"} fontSize="11" fill="var(--ink-dim)">{s.day.slice(5)}</text>
        ) : null))}
      </svg>
      <table className="sr-only">
        <caption>Share price by day</caption>
        <tbody>{snapshots.map((s) => <tr key={s.day}><th scope="row">{s.day}</th><td>{formatRatio(s.nav, 1_000_000n, 4)}</td></tr>)}</tbody>
      </table>
    </>
  );
}
