import type { SimRound } from "@/lib/types";

/* Pool vault (line) and the money still queued (bars) over the simulated rounds. Plain SVG.
   `at` is the round the scrubber is on. */
const W = 600, H = 200, PAD = { l: 8, r: 8, t: 10, b: 20 };

export function PoolQueueChart({ rounds, at }: { rounds: SimRound[]; at: number }) {
  const vaults = rounds.map((r) => Number(r.poolVault));
  const queued = rounds.map((r) => Number(r.queueOwed));
  const top = Math.max(...vaults, ...queued, 1);
  const x = (i: number) => PAD.l + (i * (W - PAD.l - PAD.r)) / (rounds.length - 1);
  const y = (v: number) => PAD.t + (1 - v / top) * (H - PAD.t - PAD.b);
  const line = vaults.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join(" ");
  const bw = (W - PAD.l - PAD.r) / rounds.length - 2;
  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Simulated pool vault and queued wins by round">
      {queued.map((q, i) => q > 0 && <rect key={i} x={x(i) - bw / 2} y={y(q)} width={bw} height={H - PAD.b - y(q)} fill="var(--loss)" opacity="0.55" />)}
      <path d={line} fill="none" stroke="var(--brass)" strokeWidth="2.5" />
      <line x1={x(at - 1)} x2={x(at - 1)} y1={PAD.t} y2={H - PAD.b} stroke="var(--ink)" strokeDasharray="4 3" />
      <circle cx={x(at - 1)} cy={y(vaults[at - 1])} r="4.5" fill="var(--ink)" />
      <text x={PAD.l} y={H - 4} fontSize="11" fill="var(--ink-dim)">round 1</text>
      <text x={W - PAD.r} y={H - 4} fontSize="11" fill="var(--ink-dim)" textAnchor="end">round {rounds.length}</text>
    </svg>
  );
}
