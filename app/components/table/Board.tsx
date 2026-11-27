import type { SymbolId } from "@/lib/symbols";
import { SYMBOLS } from "@/lib/symbols";
import { matches } from "@/lib/payouts";
import { Tile } from "./Tile";

/** The 2 x 3 cloth: heart, spade, diamond on top; club, crown, flag below (docs/characters). */
export function Board({ selected, onSelect, disabled, staked, faces }: {
  selected: SymbolId | null;
  onSelect: (id: SymbolId) => void;
  disabled?: boolean;
  staked: Partial<Record<SymbolId, bigint>>;
  faces: number[] | null;
}) {
  return (
    <div className="board" role="group" aria-label="Pick a symbol">
      {SYMBOLS.map((s) => (
        <Tile
          key={s.id}
          id={s.id}
          selected={selected === s.id}
          disabled={disabled}
          staked={staked[s.id]}
          shown={faces ? matches(s.id, faces) : undefined}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}
