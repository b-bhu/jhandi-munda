import type { SymbolId } from "@/lib/symbols";
import { SYMBOLS } from "@/lib/symbols";
import { formatAmount } from "@/lib/format";
import { SymbolIcon } from "./SymbolIcon";

export interface TileProps {
  id: SymbolId;
  selected?: boolean;
  disabled?: boolean;
  /** My total stake on this symbol this round (base units). */
  staked?: bigint;
  /** Dice showing this symbol, once the round has rolled. */
  shown?: number;
  onSelect?: (id: SymbolId) => void;
}

export function Tile({ id, selected, disabled, staked = 0n, shown, onSelect }: TileProps) {
  const sym = SYMBOLS.find((s) => s.id === id)!;
  const hit = shown !== undefined && shown >= 2;
  return (
    <button
      type="button"
      className={`tile${selected ? " sel" : ""}${hit ? " hit" : ""}`}
      disabled={disabled}
      aria-pressed={selected}
      onClick={() => onSelect?.(id)}
    >
      <SymbolIcon name={sym.name} />
      <span className="tile-name">{sym.label}</span>
      {staked > 0n && <span className="tile-stake">${formatAmount(staked, 0)} on it</span>}
      {shown !== undefined && <span className="tile-shown">{shown} of 6 dice</span>}
    </button>
  );
}
