"use client";
import { useState } from "react";
import type { Claim } from "@/lib/types";
import { formatMunda, formatMusd, formatWait } from "@/lib/format";
import { useNow } from "@/components/ui/useNow";

/*
 * One card per result. A win has two buttons: Wait (100% after the unlock) and Take now (95% at
 * once from the insurance fund). A loss has ONE button, Wait (D50): after 24 hours it mints MUNDA
 * straight into your stake. It is not a refund (#122), and chip bets get the same card (D51).
 */
export function ClaimCard({ claim, onRedeem }: { claim: Claim; onRedeem: (id: string, how: "wait" | "takeNow") => Promise<void> }) {
  const now = useNow();
  const [busy, setBusy] = useState(false);
  const left = now ? claim.unlockAt - now : 1;
  const unlocked = claim.status !== "locked" && left <= 0;
  const redeemed = claim.status === "redeemed";
  const go = async (how: "wait" | "takeNow") => { setBusy(true); try { await onRedeem(claim.id, how); } finally { setBusy(false); } };

  if (claim.kind === "win") {
    return (
      <article className="claim win-card">
        <h3 className="win">You won {formatMusd(claim.amount)}</h3>
        <p className="dim">Wait pays the full amount {unlocked ? "now" : `in ${formatWait(left)}`}. Take now pays {claim.takeNowPayout !== null ? formatMusd(claim.takeNowPayout) : "95%"} at once (95%) from a safety pot.</p>
        <div className="row">
          <button type="button" className="btn" disabled={busy || redeemed || !unlocked} onClick={() => go("wait")}>{redeemed ? "Paid" : "Wait"}</button>
          <button type="button" className="btn ghost" disabled={busy || redeemed} onClick={() => go("takeNow")}>Take now (95%)</button>
        </div>
      </article>
    );
  }
  return (
    <article className="claim loss-card">
      <h3 className="loss">This bet lost</h3>
      <p>
        Your stake is gone. In return you can claim <b>{formatMunda(claim.amount)}</b> after 24 hours.
        It goes straight into your stake, and you cannot unstake it while transfers are off. <b>This is not a refund.</b>
      </p>
      {claim.road === "chips" && <p className="dim small">Chip bets earn MUNDA the same way as wallet bets.</p>}
      <div className="row">
        <button type="button" className="btn" disabled={busy || redeemed || !unlocked} onClick={() => go("wait")}>
          {redeemed ? "Staked" : unlocked ? "Wait is over: claim" : `Wait · ${now ? formatWait(left) : "24h"}`}
        </button>
      </div>
      <p className="dim small">Losses have Wait only: there is no Take now for a loss.</p>
    </article>
  );
}
