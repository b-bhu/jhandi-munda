"use client";
import { useState } from "react";
import { SYMBOLS, type SymbolId } from "@/lib/symbols";
import { MIN_BET, PAYOUT_ROWS } from "@/lib/payouts";
import { formatMusd, parseAmount } from "@/lib/format";

const QUICK = ["1", "5", "10"];

export function BetPanel({ symbol, open, onPlace }: {
  symbol: SymbolId | null;
  open: boolean;
  onPlace: (symbol: SymbolId, amount: bigint) => Promise<void>;
}) {
  const [text, setText] = useState("1");
  const [busy, setBusy] = useState(false);
  const amount = parseAmount(text);
  const tooSmall = amount !== null && amount < MIN_BET;
  const ok = open && symbol !== null && amount !== null && !tooSmall && !busy;
  const name = SYMBOLS.find((s) => s.id === symbol)?.label;

  async function place() {
    if (!ok || symbol === null || amount === null) return;
    setBusy(true);
    try { await onPlace(symbol, amount); } finally { setBusy(false); }
  }

  return (
    <div className="bet-panel">
      <p className="dim">{name ? `Betting on ${name}` : "Pick a symbol on the board"}</p>
      <label className="dim" htmlFor="bet-amount">Amount (test MUSD, $1 minimum)</label>
      <input id="bet-amount" className="input" inputMode="decimal" value={text} onChange={(e) => setText(e.target.value)} aria-invalid={amount === null || tooSmall} />
      <div className="row">
        {QUICK.map((q) => <button key={q} type="button" className="btn ghost" onClick={() => setText(q)}>${q}</button>)}
      </div>
      {tooSmall && <p className="loss">The minimum bet is {formatMusd(MIN_BET, 0)}.</p>}
      {amount === null && <p className="loss">Type an amount like 5 or 2.5.</p>}
      <button type="button" className="btn" disabled={!ok} onClick={place}>{open ? (busy ? "Placing…" : "Place bet") : "Betting is closed"}</button>
      <p className="dim small">2 to 6 matches pay {PAYOUT_ROWS.map((r) => `${r.multiplier}x`).join(" / ")} back. 0 or 1 match pays nothing. You can lose the whole bet.</p>
    </div>
  );
}
