"use client";
import type { Round } from "@/lib/types";
import { formatClock } from "@/lib/format";
import { useNow } from "@/components/ui/useNow";

const LABEL = { idle: "No round running", open: "Join window", rolling: "Dice are rolling", rolled: "Result" } as const;

export function RoundTimer({ round }: { round: Round }) {
  const now = useNow();
  const left = round.status === "open" && now ? round.closesAt - now : 0;
  return (
    <div className="timer" aria-live="polite">
      <span className="dim">Round {round.id} · {round.players} players</span>
      <b>{LABEL[round.status]}</b>
      {round.status === "open" && <span className="clock">{now ? formatClock(left) : "--:--"}</span>}
    </div>
  );
}
