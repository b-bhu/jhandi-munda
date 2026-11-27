import type { SimRound } from "@/lib/types";
import "@/styles/dice.css";
import { SYMBOLS, symbolName } from "@/lib/symbols";
import { PAYOUT_MULTIPLIER } from "@/lib/payouts";
import { formatMusd } from "@/lib/format";
import { SymbolIcon } from "@/components/table/SymbolIcon";

export function RoundCard({ round }: { round: SimRound }) {
  const win = round.owed > 0n;
  return (
    <article className={`claim ${win ? "win-card" : "loss-card"}`}>
      {round.note && <h3>{round.note}</h3>}
      <ol className="dice replay-dice" aria-label="Dice">
        {round.faces.map((f, i) => <li key={i} className="die"><SymbolIcon name={symbolName(f)} size={26} /></li>)}
      </ol>
      <p>
        Bet {formatMusd(round.stake, 0)} on <b>{SYMBOLS.find((s) => s.id === round.symbol)?.name}</b>: {round.matches} match{round.matches === 1 ? "" : "es"}
        {PAYOUT_MULTIPLIER[round.matches] ? ` (${PAYOUT_MULTIPLIER[round.matches]}x)` : ""}.{" "}
        {win ? <span className="win">Owed {formatMusd(round.owed)}.</span> : <span className="loss">Lost the stake.</span>}
      </p>
      <p className="dim small">Pool after: {formatMusd(round.poolVault)}{round.queueOwed > 0n ? ` · ${formatMusd(round.queueOwed)} of this win waits in the queue` : ""}</p>
    </article>
  );
}
