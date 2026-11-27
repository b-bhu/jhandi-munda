"use client";
import { useState } from "react";
import { formatAmount, parseAmount } from "@/lib/format";

/** Two small forms. Withdraw is closed while wins are queued: they are paid first (D30). */
export function DepositWithdraw({ queued, maxShares, onDeposit, onWithdraw }: {
  queued: number;
  maxShares: bigint;
  onDeposit: (amount: bigint) => Promise<void>;
  onWithdraw: (shares: bigint) => Promise<void>;
}) {
  const [dep, setDep] = useState("10");
  const [wd, setWd] = useState("1");
  const [busy, setBusy] = useState(false);
  const depAmt = parseAmount(dep);
  const wdAmt = parseAmount(wd);
  const run = async (fn: () => Promise<void>) => { setBusy(true); try { await fn(); } finally { setBusy(false); } };
  const canWd = queued === 0 && wdAmt !== null && wdAmt > 0n && wdAmt <= maxShares && !busy;

  return (
    <div className="grid-2 pool-forms">
      <div className="bet-panel">
        <label className="dim" htmlFor="dep">Deposit test MUSD</label>
        <input id="dep" className="input" inputMode="decimal" value={dep} onChange={(e) => setDep(e.target.value)} />
        <button type="button" className="btn" disabled={busy || depAmt === null || depAmt <= 0n} onClick={() => depAmt && run(() => onDeposit(depAmt))}>Deposit</button>
        <p className="dim small">You get shares at the current NAV.</p>
      </div>
      <div className="bet-panel">
        <label className="dim" htmlFor="wd">Withdraw shares (you hold {formatAmount(maxShares)})</label>
        <input id="wd" className="input" inputMode="decimal" value={wd} onChange={(e) => setWd(e.target.value)} />
        <button type="button" className="btn ghost" disabled={!canWd} onClick={() => wdAmt && run(() => onWithdraw(wdAmt))}>Withdraw</button>
        <p className="dim small">{queued > 0 ? `Closed: ${queued} queued win${queued > 1 ? "s are" : " is"} paid first (D30).` : "Pays shares × NAV."}</p>
      </div>
    </div>
  );
}
