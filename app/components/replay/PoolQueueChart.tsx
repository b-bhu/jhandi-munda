import type { SimRound } from "@/lib/types";

/* Pool vault (gold line, MUSD) and wins waiting in the queue (red bars, a count) per round. Plain SVG.
   `at` is the position the scrubber is on; `jackpotAt` marks the six-match round. */
const W = 600, H = 200, PAD = { l: 8, r: 8, t: 12, b: 20 };

export function PoolQueueChart({ rounds, at, jackpotAt }: { rounds: SimRound[]; at: number; jackpotAt: number | null }) {
  const vaults = rounds.map((r) => Number(r.vault));
  const topV = Math.max(...vaults, 1);
  const topQ = Math.max(...rounds.map((r) => r.queue), 1);
  const x = (i: number) => PAD.l + (i * (W - PAD.l - PAD.r)) / (rounds.length - 1);
  const yv = (v: number) => PAD.t + (1 - v / topV) * (H - PAD.t - PAD.b);
  const yq = (q: number) => PAD.t + (1 - q / topQ) * (H - PAD.t - PAD.b);
  const line = vaults.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)} ${yv(v).toFixed(1)}`).join(" ");
  const bw = (W - PAD.l - PAD.r) / rounds.length - 2;
  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Simulated pool vault and wins waiting in the queue, by round">
      {rounds.map((r, i) => r.queue > 0 && <rect key={r.id} x={x(i) - bw / 2} y={yq(r.queue)} width={bw} height={H - PAD.b - yq(r.queue)} fill="var(--tilak)" opacity="0.5" />)}
      <path d={line} fill="none" stroke="var(--gold)" strokeWidth="2.5" />
      {jackpotAt !== null && <g><line x1={x(jackpotAt)} x2={x(jackpotAt)} y1={PAD.t} y2={H - PAD.b} stroke="var(--gold)" strokeDasharray="2 3" /><text x={x(jackpotAt) + 5} y={PAD.t + 8} fontSize="11" fill="var(--gold)">101x</text></g>}
      <line x1={x(at)} x2={x(at)} y1={PAD.t} y2={H - PAD.b} stroke="var(--t1)" strokeDasharray="4 3" />
      <circle cx={x(at)} cy={yv(vaults[at])} r="4.5" fill="var(--t1)" />
      <text x={PAD.l} y={H - 4} fontSize="11" fill="var(--t4)">round {rounds[0].id}</text>
      <text x={W - PAD.r} y={H - 4} fontSize="11" fill="var(--t4)" textAnchor="end">round {rounds[rounds.length - 1].id}</text>
    </svg>
  );
}
