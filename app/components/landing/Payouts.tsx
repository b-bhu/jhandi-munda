import { SectionRule } from "@/components/ui/SectionRule";
import { PAYOUT_ROWS } from "@/lib/payouts";
import { Photo } from "./Photo";

/** Both columns come from payouts.ts, so the table cannot drift from the program's multipliers. */
export function Payouts() {
  return (
    <section className="blk">
      <SectionRule n="02">The board and the payouts</SectionRule>
      <h2>The more dice match, the more you win.</h2>
      <p className="lead">Six dice, six symbols each. Pick one symbol and count how many dice land on it.</p>
      <div className="two">
        <Photo k="board" />
        <div>
          <table>
            <thead><tr><th>Dice matching</th><th>Profit</th><th>You get back</th></tr></thead>
            <tbody>
              <tr><td className="m">0 or 1</td><td className="m">—</td><td className="m">0</td></tr>
              {PAYOUT_ROWS.map((r) => (
                <tr key={r.matches}><td className="m">{r.matches}</td><td className="g">{r.multiplier - 1}x</td><td className="m">{r.multiplier}x</td></tr>
              ))}
            </tbody>
          </table>
          <p className="note">“You get back” includes your stake. A 2% fee comes out of a winner’s profit. The table leans slightly to the house (about 3.8% before fees); we publish it instead of hiding it.</p>
        </div>
      </div>
    </section>
  );
}
