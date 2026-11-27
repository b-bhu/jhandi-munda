import type { Replay } from "@/lib/types";

/* Pool cash (gold) and what the queue owes (red, dashed) per round, both in MUSD on one scale. Plain SVG.
   `at` is the position the scrubber is on; the green marker is the 101x round. */
const W = 760, H = 260, L = 40, R = 20, T = 14, B = 24;

export function PoolQueueChart({ replay, at }: { replay: Replay; at: number }) {
  const { rounds, jackpot } = replay;
  const cash = rounds.map((r) => Number(r.vault));
  const owed = rounds.map((r) => Number(r.queueOwed));
  const top = Math.max(...cash, ...owed, 1);
  const x = (i: number) => L + (i * (W - L - R)) / (rounds.length - 1);
  const y = (v: number) => T + (1 - v / top) * (H - T - B);
  const pts = (a: number[]) => a.map((v, i) => `${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
  const jIdx = jackpot ? rounds.findIndex((r) => r.id === jackpot.round) : -1;
  const ticks = [0, Math.round(rounds.length / 5), Math.round((2 * rounds.length) / 5), Math.round((3 * rounds.length) / 5), Math.round((4 * rounds.length) / 5), rounds.length - 1];
  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} width="100%" role="img" aria-label="Simulated pool cash and what the payout queue owes, by round">
      <line x1={L} y1={H - B} x2={W - R} y2={H - B} stroke="var(--line2)" />
      {ticks.map((t) => <text key={t} x={x(t)} y={H - 6} fill="var(--t4)" fontSize="10" fontFamily="var(--mono)" textAnchor="middle">{rounds[t].id}</text>)}
      <polyline fill="none" stroke="var(--gold)" strokeWidth="2.2" points={pts(cash)} />
      <polyline fill="none" stroke="var(--tilak)" strokeWidth="1.8" strokeDasharray="4 3" points={pts(owed)} />
      {jIdx >= 0 && (
        <g>
          <line x1={x(jIdx)} y1={T} x2={x(jIdx)} y2={H - B} stroke="var(--green)" strokeDasharray="3 3" />
          <circle cx={x(jIdx)} cy={y(cash[jIdx])} r="6" fill="var(--green)" />
          <text x={x(jIdx) - 10} y={T + 12} fill="var(--green)" fontSize="12" fontFamily="var(--mono)" fontWeight="700" textAnchor={jIdx > rounds.length / 2 ? "end" : "start"}>Round {jackpot!.round}: 101x jackpot</text>
        </g>
      )}
      <line x1={x(at)} y1={T} x2={x(at)} y2={H - B} stroke="var(--t1)" strokeDasharray="4 3" />
      <circle cx={x(at)} cy={y(cash[at])} r="4.5" fill="var(--t1)" />
    </svg>
  );
}
