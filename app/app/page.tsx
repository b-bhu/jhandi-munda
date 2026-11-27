import Link from "next/link";
import { PAYOUT_ROWS } from "@/lib/payouts";

export default function Home() {
  return (
    <main className="center">
      <div className="page" style={{ textAlign: "center" }}>
        <h1>Jhandi Munda Casino - devnet</h1>
        <p className="dim">Six dice, six symbols. Pick one; the more dice show it, the more it pays back.</p>
        <div className="scroll-x">
          <table className="t" style={{ maxWidth: 360, margin: "0 auto" }}>
            <thead><tr><th>Matches</th><th className="num">Total back</th></tr></thead>
            <tbody>
              {PAYOUT_ROWS.map((r) => (
                <tr key={r.matches}><td>{r.matches} of 6</td><td className="num">{r.multiplier}x</td></tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="dim">0 or 1 match pays nothing. A 2% fee is taken on the profit part of a win. Test MUSD, devnet only.</p>
        <p>
          <Link href="/play">Play</Link> · <Link href="/pool">Pool</Link> · <Link href="/get-musd">Get MUSD</Link> (devnet faucet, 100 MUSD per click)
        </p>
      </div>
    </main>
  );
}
