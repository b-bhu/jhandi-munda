"use client";
import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { getClient, type TableState as MockState } from "@/lib/game-client";
import type { Bet, Claim, Round, SessionView } from "@/lib/types";
import type { SymbolId } from "@/lib/symbols";
import { Timer } from "./Timer";
import { Board } from "./Board";
import { Bowl } from "./Bowl";
import { DiceRow } from "./DiceRow";
import { BetPanel } from "./BetPanel";
import { SessionBadge } from "./SessionBadge";
import { MyBets } from "./MyBets";
import { RoundMath } from "./RoundMath";
import { ClaimCard } from "./ClaimCard";

/**
 * The table, in whichever phase the round is in: A betting open, B bowl covered (rolling), C result and claims.
 * The phase comes from `round.status`, so the real client drives it the same way the mock does.
 * `mock` only picks which example the mock client serves.
 */
export function TableState({ mock, preselect }: { mock?: MockState; preselect?: SymbolId }) {
  const client = useMemo(() => getClient(mock), [mock]);
  const [round, setRound] = useState<Round | null>(null);
  const [bets, setBets] = useState<Bet[]>([]);
  const [claims, setClaims] = useState<Claim[]>([]);
  const [session, setSession] = useState<SessionView | null>(null);
  const [selected, setSelected] = useState<SymbolId | null>(preselect ?? null);

  useEffect(() => {
    let live = true;
    const off = client.subscribeRound((r) => live && setRound(r));
    client.getMyBets().then((b) => live && setBets(b));
    client.getMyClaims().then((c) => live && setClaims(c));
    client.getSession().then((x) => live && setSession(x));
    return () => { live = false; off(); };
  }, [client]);

  if (!round) return <p className="small" role="status">Loading the table…</p>;

  const mine: Partial<Record<SymbolId, bigint>> = {};
  for (const b of bets) mine[b.symbol] = (mine[b.symbol] ?? 0n) + b.amount;
  const open = round.status === "open";
  const place = async (symbol: SymbolId, amount: bigint) => { const bet = await client.placeBet(symbol, amount); setBets((b) => [...b, bet]); };
  const redeem = async (id: string, how: "wait" | "takeNow") => { const done = await client.redeemClaim(id, how); setClaims((cs) => cs.map((c) => (c.id === id ? done : c))); };

  const betPanel = (
    <BetPanel symbol={selected} onSymbol={setSelected} open={open} walletMusd={100_000_000n} onPlace={place}>
      <SessionBadge session={session} onStart={() => client.startSession().then(setSession)} onRevoke={() => client.revokeSession().then(setSession)} />
    </BetPanel>
  );

  if (round.status === "rolled") {
    return (
      <>
        <div className="grid2">
          <div>
            <DiceRow faces={round.faces} mark />
            <div style={{ height: 18 }} />
            <Board selected={null} onSelect={() => {}} disabled totals={round.totals} mine={mine} faces={round.faces} />
            <p className="note">Faces <span className="mock">Example</span>: two or more of a symbol wins for that symbol. The faces and their proof are on the <Link href="/fairness/12">fairness page</Link>.</p>
          </div>
          <div className="stack">
            <MyBets bets={bets} status={round.status} />
            {bets.length > 0 && <RoundMath bets={bets} />}
          </div>
        </div>
        {claims.length > 0 && <div className="claims">{claims.map((c) => <ClaimCard key={c.id} claim={c} onRedeem={redeem} />)}</div>}
      </>
    );
  }
  return (
    <div className="grid2">
      <div>
        <Timer round={round} />
        {open ? (
          <>
            <Board selected={selected} onSelect={setSelected} totals={round.totals} mine={mine} faces={null} />
            <p className="note">Cloth totals show all players’ bets on each symbol (example). The gold tile is the symbol you picked.</p>
          </>
        ) : (
          <>
            <Bowl />
            <div style={{ height: 18 }} />
            <DiceRow faces={null} />
            <p className="note">If you open the page late, you still see the result, never a blank board.</p>
          </>
        )}
      </div>
      <div className="stack">{betPanel}<MyBets bets={bets} status={round.status} /></div>
    </div>
  );
}
