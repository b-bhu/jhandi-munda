"use client";
import type { SessionView } from "@/lib/types";
import { formatAmount } from "@/lib/format";
import { useNow } from "@/components/ui/useNow";

/** Session keys (D12): approve once, then bets need no wallet popup, up to a cap. Start and revoke live here. */
export function SessionBadge({ session, onStart, onRevoke }: { session: SessionView | null; onStart: () => void; onRevoke: () => void }) {
  const now = useNow();
  if (!session) return null;
  if (!session.active) {
    return (
      <p className="note"><span className="pill2">Session off</span> &nbsp;every bet asks your wallet · <button type="button" className="linkbtn" onClick={onStart}>start a 1-hour session</button></p>
    );
  }
  const mins = now ? Math.max(0, Math.round((session.expiresAt - now) / 60)) : null;
  return (
    <p className="note">
      <span className="pill2 g">Session on</span> &nbsp;{mins === null ? "…" : mins} min left · bets need no wallet popup (cap {formatAmount(session.cap, 0)} MUSD) · <button type="button" className="linkbtn" onClick={onRevoke}>revoke</button>
    </p>
  );
}
