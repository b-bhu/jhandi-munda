import type { Snapshot } from "@/lib/types";
import { formatRatio } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { ExampleTag } from "@/components/shell/ExampleTag";

/* Daily NAV per share as a plain SVG line, with the fee split under it. No chart library; the same data
   is in a hidden table for screen readers. NAV is a bigint ratio; Number is only used to place pixels. */
const W = 520, H = 150, PAD = { l: 44, r: 10, t: 12, b: 24 };

export function SnapshotChart({ snapshots }: { snapshots: Snapshot[] }) {
  const navs = snapshots.map((s) => Number(s.nav));
  const lo = Math.min(...navs), hi = Math.max(...navs), span = hi - lo || 1;
  const x = (i: number) => PAD.l + (i * (W - PAD.l - PAD.r)) / Math.max(1, snapshots.length - 1);
  const y = (v: number) => PAD.t + (1 - (v - lo) / span) * (H - PAD.t - PAD.b);
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>Daily snapshots · NAV per share</span><ExampleTag /></div>
      {snapshots.length < 2 ? <p className="small">Not enough daily snapshots yet.</p> : (
        <>
          <svg className="chart" viewBox={`0 0 ${W} ${H}`} width="100%" role="img" aria-label="Pool NAV per share, one point a day">
            {[lo, hi].map((t) => (
              <g key={t}><line x1={PAD.l} x2={W - PAD.r} y1={y(t)} y2={y(t)} stroke="var(--line2)" strokeDasharray="3 4" /><text x={PAD.l - 6} y={y(t) + 4} textAnchor="end" fontSize="10" fontFamily="var(--mono)" fill="var(--t4)">{formatRatio(BigInt(Math.round(t)), 1_000_000n, 3)}</text></g>
            ))}
            <polyline fill="none" stroke="var(--gold)" strokeWidth="2" points={navs.map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" ")} />
            {navs.map((v, i) => <circle key={i} cx={x(i)} cy={y(v)} r="3" fill="var(--gold)" />)}
            <text x={PAD.l} y={H - 6} fontSize="10" fontFamily="var(--mono)" fill="var(--t4)">{snapshots[0].day.slice(5)}</text>
            <text x={W - PAD.r} y={H - 6} fontSize="10" fontFamily="var(--mono)" fill="var(--t4)" textAnchor="end">{snapshots[snapshots.length - 1].day.slice(5)}</text>
          </svg>
          <table className="sr-only"><caption>NAV per share by day</caption><tbody>{snapshots.map((s) => <tr key={s.day}><th scope="row">{s.day}</th><td>{formatRatio(s.nav, 1_000_000n, 4)}</td></tr>)}</tbody></table>
        </>
      )}
      <div className="legend"><span>{snapshots.length} daily snapshots · example</span></div>
      <div className="kv" style={{ marginTop: 14 }}><span>Fee split</span><b>70% stakers · 20% insurance · 10% devs</b></div>
      <div className="kv"><span>Above $10,000 in the pool</span><b>gains split 50/50 stakers and LPs</b></div>
      <p className="note">Hackathon demo on devnet. Test money only. Nothing here is a promise of returns, and past NAV says nothing about the future.</p>
    </CornerBox>
  );
}
