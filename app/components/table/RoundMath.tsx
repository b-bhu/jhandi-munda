import type { Bet } from "@/lib/types";
import { SYMBOLS } from "@/lib/symbols";
import { PAYOUT_MULTIPLIER, FEE_BPS, profitFor } from "@/lib/payouts";
import { formatAmount } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { ExampleTag } from "@/components/shell/ExampleTag";

/** "Math for this round": profit, the 2% fee on the profit, and what comes back (stake included). */
export function RoundMath({ bets }: { bets: Bet[] }) {
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>Math for this round</span><ExampleTag /></div>
      {bets.map((b) => {
        const name = SYMBOLS.find((s) => s.id === b.symbol)!.label.split(" ")[0];
        const k = b.matches ?? 0;
        if (b.result !== "won") {
          return <div className="kv" key={b.id}><span>{name} ${formatAmount(b.amount, 0)} · {k} matched</span><b className="r">−{formatAmount(b.amount)}</b></div>;
        }
        const profit = profitFor(b.amount, k);
        const fee = (profit * FEE_BPS) / 10_000n;
        return (
          <div key={b.id}>
            <div className="kv"><span>{name} ${formatAmount(b.amount, 0)} · {k} matched · {PAYOUT_MULTIPLIER[k] - 1}x profit</span><b>+{formatAmount(profit)}</b></div>
            <div className="kv"><span>2% fee on profit</span><b>−{formatAmount(fee)}</b></div>
            <div className="kv"><span>You get back (stake included)</span><b className="g">{formatAmount(b.amount + profit - fee)}</b></div>
          </div>
        );
      })}
    </CornerBox>
  );
}
