import type { Position } from "@/lib/types";
import { formatAmount, formatMusd } from "@/lib/format";

export function MyPosition({ position }: { position: Position }) {
  return (
    <div className="stats pos">
      <div className="stat"><b>{formatAmount(position.shares)}</b><span>My shares</span></div>
      <div className="stat"><b>{formatMusd(position.value)}</b><span>Worth now (shares × NAV)</span></div>
      <div className="stat"><b>{formatMusd(position.walletMusd)}</b><span>In my wallet</span></div>
    </div>
  );
}
