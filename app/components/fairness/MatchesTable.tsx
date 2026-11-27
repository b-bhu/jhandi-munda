import type { FairnessBet } from "@/lib/types";
import { SYMBOLS } from "@/lib/symbols";
import { PAYOUT_MULTIPLIER } from "@/lib/payouts";
import { formatMusd, shortKey } from "@/lib/format";

export function MatchesTable({ bets }: { bets: FairnessBet[] }) {
  return (
    <div className="scroll-x" tabIndex={0} role="region" aria-label="Bets and matches table, scrolls sideways on small screens">
      <table className="t">
        <thead><tr><th>Player</th><th>Symbol</th><th className="num">Stake</th><th className="num">Matches</th><th className="num">Pays</th><th className="num">Owed</th></tr></thead>
        <tbody>
          {bets.map((b, i) => (
            <tr key={i}>
              <td className="mono">{shortKey(b.player)}</td>
              <td>{SYMBOLS.find((s) => s.id === b.symbol)?.name}{b.road === "chips" ? " (chips)" : ""}</td>
              <td className="num">{formatMusd(b.amount)}</td>
              <td className="num">{b.matches}</td>
              <td className="num">{PAYOUT_MULTIPLIER[b.matches] ? `${PAYOUT_MULTIPLIER[b.matches]}x` : "0"}</td>
              <td className="num">{formatMusd(b.owed)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p className="dim small">Owed = stake + 98% of the profit (a 2% fee on the profit part only).</p>
    </div>
  );
}
