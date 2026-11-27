"use client";
import type { SessionView } from "@/lib/types";
import { formatMusd, formatWait } from "@/lib/format";
import { useNow } from "@/components/ui/useNow";

/** Session keys (D12): approve once, then bets go through without a wallet popup, up to a cap. */
export function SessionBadge({ session, onStart, onRevoke }: { session: SessionView | null; onStart: () => void; onRevoke: () => void }) {
  const now = useNow();
  if (!session) return null;
  if (!session.active) {
    return (
      <div className="session off">
        <span>Session off: every bet asks your wallet.</span>
        <button type="button" className="btn ghost" onClick={onStart}>Start a 1-hour session</button>
      </div>
    );
  }
  const left = now ? session.expiresAt - now : 0;
  return (
    <div className="session on">
      <span>Session on · spent {formatMusd(session.spent, 0)} of {formatMusd(session.cap, 0)} cap · {now ? formatWait(left) : "…"} left</span>
      <button type="button" className="btn ghost" onClick={onRevoke}>Revoke</button>
    </div>
  );
}
