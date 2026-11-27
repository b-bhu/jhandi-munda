"use client";
import type { Round } from "@/lib/types";
import { formatClock } from "@/lib/format";
import { useNow } from "@/components/ui/useNow";
import { ExampleTag } from "@/components/shell/ExampleTag";
import { ROUND_SECS } from "@/lib/mock-data";

/** The 90-second window: big clock, a bar that fills as it runs out, and one line of text. */
export function Timer({ round }: { round: Round }) {
  const now = useNow();
  const open = round.status === "open";
  const left = open && now ? Math.max(0, round.closesAt - now) : 0;
  const used = open ? ((ROUND_SECS - left) / ROUND_SECS) * 100 : 100;
  return (
    <div className="timer" aria-live="off">
      <span className="big">{open ? (now ? formatClock(left) : "--:--") : "00:00"}</span>
      <div style={{ flex: 1 }}>
        <div className="bar"><i style={{ width: `${used}%` }} /></div>
        <div className="small" style={{ marginTop: 8 }}>
          {open ? `Window closes in ${now ? left : "…"} s of ${ROUND_SECS} s · ${round.players} players at the table` : "Window closed · bets are locked"} <ExampleTag />
        </div>
      </div>
    </div>
  );
}
