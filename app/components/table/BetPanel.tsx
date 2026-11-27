"use client";
import { useState } from "react";
import { SYMBOLS, type SymbolId } from "@/lib/symbols";
import { MIN_BET, PAYOUT_MULTIPLIER } from "@/lib/payouts";
import { formatAmount, parseAmount } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { ExampleTag } from "@/components/shell/ExampleTag";
import { SymbolIcon } from "./SymbolIcon";

const CHIPS = ["1", "5", "10", "25"];

/** "Place a bet": 1 pick a symbol, 2 an amount ($1 minimum), the payout lines, and the button. */
export function BetPanel({ symbol, onSymbol, open, walletMusd, onPlace, children }: {
  symbol: SymbolId | null;
  onSymbol: (s: SymbolId) => void;
  open: boolean;
  walletMusd: bigint;
  onPlace: (symbol: SymbolId, amount: bigint) => Promise<void>;
  children?: React.ReactNode; // the session note
}) {
  const [text, setText] = useState("5");
  const [busy, setBusy] = useState(false);
  const amount = parseAmount(text);
  const bad = amount === null || amount < MIN_BET;
  const ok = open && symbol !== null && !bad && !busy;

  async function place() {
    if (!ok || symbol === null || amount === null) return;
    setBusy(true);
    try { await onPlace(symbol, amount); } finally { setBusy(false); }
  }

  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>Place a bet</span><ExampleTag /></div>
      <div className="small" style={{ marginBottom: 10 }}>1 · Pick a symbol</div>
      <div className="syms" role="group" aria-label="Pick a symbol">
        {SYMBOLS.map((s) => (
          <button key={s.id} type="button" className={`sb${symbol === s.id ? " on" : ""}`} disabled={!open} aria-pressed={symbol === s.id} aria-label={s.label} onClick={() => onSymbol(s.id)}>
            <SymbolIcon name={s.name} />
          </button>
        ))}
      </div>
      <div className="small" style={{ margin: "18px 0 0" }}>2 · Amount (min $1)</div>
      <label className="field">
        <span className="small">MUSD</span>
        <input className="amt" inputMode="decimal" value={open ? text : ""} placeholder="—" disabled={!open} aria-invalid={open && bad} onChange={(e) => setText(e.target.value)} />
      </label>
      <div className="chips">
        {CHIPS.map((c) => <button key={c} type="button" className={text === c ? "on" : ""} disabled={!open} onClick={() => setText(c)}>${c}</button>)}
        <button type="button" disabled={!open} onClick={() => setText(formatAmount(walletMusd, 2).replace(/,/g, ""))}>MAX</button>
      </div>
      {open && bad && <p className="note" role="alert" style={{ color: "var(--tilak-text)" }}>{amount === null ? "Type an amount like 5 or 2.5." : "The minimum bet is $1."}</p>}
      <div className="kv" style={{ marginTop: 14 }}><span>If 2 dice match</span><b>+{PAYOUT_MULTIPLIER[2] - 1}x profit ({PAYOUT_MULTIPLIER[2]}x back)</b></div>
      <div className="kv"><span>If 6 dice match</span><b>+{PAYOUT_MULTIPLIER[6] - 1}x profit ({PAYOUT_MULTIPLIER[6]}x back)</b></div>
      <div className="kv"><span>Fee</span><b>2% of a winner’s profit</b></div>
      <button type="button" className={`go${ok ? "" : " off"}`} disabled={!ok} onClick={place}>
        {!open ? "Betting closed" : busy ? "Placing…" : `Place bet${amount !== null && !bad ? ` · ${formatAmount(amount, 2).replace(/\.00$/, "")} MUSD` : ""}`}
      </button>
      {children}
    </CornerBox>
  );
}
