import type { SimRound } from "@/lib/types";
import { symbolName, SYMBOLS } from "@/lib/symbols";
import { PAYOUT_MULTIPLIER, matches, owedFor } from "@/lib/payouts";
import { formatMusd } from "@/lib/format";
import { SymbolIcon } from "@/components/table/SymbolIcon";
import "@/styles/dice.css";

const STAKE = 1_000_000n; // every simulated bet is 1 MUSD

export function RoundCard({ round }: { round: SimRound }) {
  return (
    <article className="claim">
      <ol className="dice replay-dice" aria-label={`Round ${round.id} dice`}>
        {round.faces.map((f, i) => <li key={i} className="die"><SymbolIcon name={symbolName(f)} size={26} /></li>)}
      </ol>
      {round.bets.length === 0 ? <p className="dim">No bets this round.</p> : (
        <ul className="sim-bets">
          {round.bets.map((b, i) => {
            const k = matches(b.face, round.faces);
            const owed = owedFor(STAKE, k);
            return (
              <li key={i}>
                Player {b.player + 1} · {SYMBOLS.find((s) => s.id === b.face)?.name} · {k} match{k === 1 ? "" : "es"}
                {PAYOUT_MULTIPLIER[k] ? ` (${PAYOUT_MULTIPLIER[k]}x)` : ""} ·{" "}
                {owed > 0n ? <span className="win">owed {formatMusd(owed)}</span> : <span className="loss">lost 1.00 MUSD</span>}
              </li>
            );
          })}
        </ul>
      )}
      {round.note && <p className="dim small">{round.note}</p>}
    </article>
  );
}
