"use client";
import { useState } from "react";
import Link from "next/link";
import { formatAmount, formatRatio, parseAmount } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { ExampleTag } from "@/components/shell/ExampleTag";

type Tab = "deposit" | "withdraw";
const DEPOSIT_CHIPS = ["10", "50", "100"];

/** Two tabs. Withdraw is closed while wins are queued: they are paid first (D30). */
export function DepositWithdraw({ nav, queued, walletMusd, shares, onDeposit, onWithdraw }: {
  nav: bigint;
  queued: number;
  walletMusd: bigint;
  shares: bigint;
  onDeposit: (amount: bigint) => Promise<void>;
  onWithdraw: (shares: bigint) => Promise<void>;
}) {
  const [tab, setTab] = useState<Tab>("deposit");
  const [dep, setDep] = useState("100");
  const [wd, setWd] = useState("10");
  const [busy, setBusy] = useState(false);
  const depAmt = parseAmount(dep);
  const wdAmt = parseAmount(wd);
  const run = async (fn: () => Promise<void>) => { setBusy(true); try { await fn(); } finally { setBusy(false); } };
  const canDep = !busy && depAmt !== null && depAmt > 0n && depAmt <= walletMusd;
  const canWd = !busy && queued === 0 && wdAmt !== null && wdAmt > 0n && wdAmt <= shares;
  const receive = depAmt !== null && nav > 0n ? (depAmt * 1_000_000n) / nav : 0n;
  const get = wdAmt !== null ? (wdAmt * nav) / 1_000_000n : 0n;

  return (
    <CornerBox className="panel">
      <div className="tabs" role="tablist" aria-label="Deposit or withdraw">
        {(["deposit", "withdraw"] as const).map((t) => (
          <button key={t} role="tab" type="button" id={`tab-${t}`} aria-selected={tab === t} aria-controls="pool-form" className={tab === t ? "on" : ""} onClick={() => setTab(t)}>{t === "deposit" ? "Deposit" : "Withdraw"}</button>
        ))}
      </div>
      <div id="pool-form" role="tabpanel" aria-labelledby={`tab-${tab}`}>
        {tab === "deposit" ? (
          <>
            <div className="ptitle"><span>Deposit MUSD, get shares</span><ExampleTag /></div>
            <label className="field"><span className="small">Amount</span><input className="amt" inputMode="decimal" value={dep} onChange={(e) => setDep(e.target.value)} aria-label="Amount of MUSD to deposit" /></label>
            <div className="chips">
              {DEPOSIT_CHIPS.map((c) => <button key={c} type="button" className={dep === c ? "on" : ""} onClick={() => setDep(c)}>{c}</button>)}
              <button type="button" onClick={() => setDep(formatAmount(walletMusd, 2).replace(/,/g, ""))}>MAX</button>
            </div>
            <div className="kv" style={{ marginTop: 14 }}><span>Your wallet</span><b>{formatAmount(walletMusd)} MUSD</b></div>
            <div className="kv"><span>You receive (at NAV {formatRatio(nav, 1_000_000n, 2)})</span><b>≈ {formatAmount(receive)} shares</b></div>
            <button type="button" className={`go${canDep ? "" : " off"}`} disabled={!canDep} onClick={() => depAmt && run(() => onDeposit(depAmt))}>Deposit {depAmt !== null ? formatAmount(depAmt, 2).replace(/\.00$/, "") : ""} MUSD</button>
            <p className="note">Withdraw burns shares at today’s NAV, rounded in the pool’s favour. Need test MUSD? Use the <Link href="/get-musd">faucet</Link>.</p>
          </>
        ) : (
          <>
            <div className="ptitle"><span>Withdraw shares, get MUSD</span><ExampleTag /></div>
            <label className="field"><span className="small">Shares</span><input className="amt" inputMode="decimal" value={wd} onChange={(e) => setWd(e.target.value)} aria-label="Pool shares to withdraw" /></label>
            <div className="chips">
              {[["25%", 4n], ["50%", 2n], ["All", 1n]].map(([label, d]) => <button key={label as string} type="button" onClick={() => setWd(formatAmount(shares / (d as bigint), 2).replace(/,/g, ""))}>{label as string}</button>)}
            </div>
            <div className="kv" style={{ marginTop: 14 }}><span>Your shares</span><b>{formatAmount(shares)}</b></div>
            <div className="kv"><span>You receive (at NAV {formatRatio(nav, 1_000_000n, 2)})</span><b>≈ {formatAmount(get)} MUSD</b></div>
            <button type="button" className={`go${canWd ? "" : " off"}`} disabled={!canWd} onClick={() => wdAmt && run(() => onWithdraw(wdAmt))}>
              {queued > 0 ? "Withdraw closed while wins are queued" : `Withdraw ${wdAmt !== null ? formatAmount(wdAmt, 2).replace(/\.00$/, "") : ""} shares`}
            </button>
            <p className="note">{queued > 0 ? `${queued} queued win${queued > 1 ? "s are" : " is"} paid first, in order (D30). Withdraw opens when the queue is empty.` : "Pays shares × NAV, rounded in the pool’s favour."}</p>
          </>
        )}
      </div>
    </CornerBox>
  );
}
