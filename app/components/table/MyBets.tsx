import type { Bet } from "@/lib/types";
import { SYMBOLS } from "@/lib/symbols";
import { formatMusd } from "@/lib/format";

const RESULT = { open: "waiting", won: "won", lost: "lost" } as const;

export function MyBets({ bets }: { bets: Bet[] }) {
  if (bets.length === 0) return <p className="dim">No bets in this round yet.</p>;
  return (
    <div className="scroll-x" tabIndex={0} role="region" aria-label="My bets table, scrolls sideways on small screens">
      <table className="t">
        <thead><tr><th>Symbol</th><th className="num">Stake</th><th>Road</th><th>Result</th></tr></thead>
        <tbody>
          {bets.map((b) => (
            <tr key={b.id}>
              <td>{SYMBOLS.find((s) => s.id === b.symbol)?.label}</td>
              <td className="num">{formatMusd(b.amount, 2)}</td>
              <td>{b.road === "chips" ? "chips" : "wallet"}</td>
              <td className={b.result === "won" ? "win" : b.result === "lost" ? "loss" : "dim"}>
                {RESULT[b.result]}{b.matches !== undefined ? ` · ${b.matches} matches` : ""}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
