import type { Position } from "@/lib/types";
import { formatAmount, formatRatio } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { ExampleTag } from "@/components/shell/ExampleTag";

/** Shares, what they cost per share, what they are worth now, and the change (which can be negative). */
export function MyPosition({ position, nav }: { position: Position; nav: bigint }) {
  const tenths = position.boughtAt > 0n ? ((nav - position.boughtAt) * 1000n) / position.boughtAt : 0n; // tenths of a percent
  const neg = tenths < 0n;
  const abs = neg ? -tenths : tenths;
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>My LP position</span><ExampleTag /></div>
      <div className="kv"><span>Shares</span><b>{formatAmount(position.shares)}</b></div>
      <div className="kv"><span>You bought at</span><b>${formatRatio(position.boughtAt, 1_000_000n, 2)} / share</b></div>
      <div className="kv"><span>Worth now</span><b>${formatRatio(nav, 1_000_000n, 2)} / share → ${formatAmount(position.value)}</b></div>
      <div className="kv"><span>Your profit</span><b className={neg ? "r" : "g"}>{neg ? "−" : "+"}{(abs / 10n).toString()}.{(abs % 10n).toString()}%</b></div>
      <p className="note">Past results are not a promise. Pool value can go down as well as up.</p>
    </CornerBox>
  );
}
