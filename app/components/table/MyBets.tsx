import type { Bet } from "@/lib/types";
import { SYMBOLS } from "@/lib/symbols";
import { formatAmount } from "@/lib/format";
import { profitFor } from "@/lib/payouts";
import { CornerBox } from "@/components/ui/CornerBox";
import { ExampleTag } from "@/components/shell/ExampleTag";
import { SymbolIcon } from "./SymbolIcon";
import type { RoundStatus } from "@/lib/types";

/** "My bets this round": one row each. What the row says depends on whether the dice have rolled. */
export function MyBets({ bets, status }: { bets: Bet[]; status: RoundStatus }) {
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>My bets this round</span><ExampleTag /></div>
      {bets.length === 0 && <p className="small">No bets yet. Pick a symbol and place one.</p>}
      {bets.map((b) => {
        const sym = SYMBOLS.find((s) => s.id === b.symbol)!;
        const sub = b.result === "open" ? (status === "open" ? `Placed · ${b.road === "chips" ? "chips" : "no popup"}` : "Locked") : `${b.matches ?? 0} dice matched`;
        const right = b.result === "won" ? `Won +${formatAmount(profitFor(b.amount, b.matches ?? 0))}` : b.result === "lost" ? `Lost −${formatAmount(b.amount)}` : status === "open" ? "Open" : "Waiting";
        return (
          <div className="row" key={b.id}>
            <span className="mini"><SymbolIcon name={sym.name} /></span>
            <div><div style={{ fontWeight: 600 }}>{sym.label.split(" ")[0]} · {formatAmount(b.amount)} MUSD</div><div className="s">{sub}</div></div>
            <span className="a kv"><b className={b.result === "won" ? "g" : b.result === "lost" ? "r" : ""} style={{ border: 0 }}>{right}</b></span>
          </div>
        );
      })}
    </CornerBox>
  );
}
