"use client";
import { useState } from "react";
import type { Claim } from "@/lib/types";
import { formatAmount, formatWait } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { useNow } from "@/components/ui/useNow";

/*
 * One card per result. A win has two buttons: Wait (100% after 90 s) and Take now (95% at once from
 * the insurance fund). A loss has ONE working button, Wait (D50): after 24 hours it mints MUNDA straight
 * into your stake. It is not a refund (#122), and chip bets get the same card (D51).
 */
export function ClaimCard({ claim, onRedeem }: { claim: Claim; onRedeem: (id: string, how: "wait" | "takeNow") => Promise<void> }) {
  const now = useNow();
  const [busy, setBusy] = useState(false);
  const left = now ? claim.unlockAt - now : 1;
  const unlocked = left <= 0;
  const done = claim.status === "redeemed";
  const go = async (how: "wait" | "takeNow") => { setBusy(true); try { await onRedeem(claim.id, how); } finally { setBusy(false); } };

  if (claim.kind === "win") {
    const payout = claim.takeNowPayout ?? 0n;
    return (
      <CornerBox className="claim">
        <span className="tag" style={{ color: "var(--gold)" }}>You won</span>
        <h2>{formatAmount(claim.amount)} MUSD {done ? "paid" : "ready"}</h2>
        <p>Wait 90 seconds for the full payout, or take 95% now. If the pool is short, your win joins the first-in, first-out queue.</p>
        <div className="acts">
          <button type="button" className="b1" disabled={busy || done || !unlocked} onClick={() => go("wait")}>Wait · {formatAmount(claim.amount)} {unlocked ? "now" : `in ${formatWait(left)}`}</button>
          <button type="button" className="b2" disabled={busy || done} onClick={() => go("takeNow")}>Take now · {formatAmount(payout)} (95%)</button>
        </div>
        <p className="note">Example numbers.</p>
      </CornerBox>
    );
  }
  return (
    <CornerBox className="claim">
      <span className="tag" style={{ color: "var(--tilak-text)" }}>You lost</span>
      <h2>You earned {formatAmount(claim.amount, 0)} MUNDA</h2>
      <p>
        MUNDA is a share of house fees. <b>This is not a refund.</b> It goes straight into your stake after 24 hours.
        Your first $1,000 of losses earn 5x MUNDA. Chip bets earn it too.
      </p>
      <div className="acts">
        <button type="button" className="b1" disabled={busy || done || !unlocked} onClick={() => go("wait")}>{done ? "In your stake" : unlocked ? "Wait is over · claim" : `Wait · ${now ? formatWait(left) : "24 h"} → stake`}</button>
        <button type="button" className="b2 off" disabled aria-disabled="true">Take now · not for losses</button>
      </div>
      <p className="note">Example numbers. D50, D51.</p>
    </CornerBox>
  );
}
