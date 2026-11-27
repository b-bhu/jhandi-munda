import { SYMBOLS, type SymbolId } from "@/lib/symbols";
import { matches } from "@/lib/payouts";
import { formatAmount } from "@/lib/format";
import { SymbolIcon } from "./SymbolIcon";

/** The cloth: six tiles, 3 x 2. Open: everyone's totals and your own stake. Rolled: dice per symbol. */
export function Board({ selected, onSelect, disabled, totals, mine, faces }: {
  selected: SymbolId | null;
  onSelect: (id: SymbolId) => void;
  disabled?: boolean;
  totals: Partial<Record<SymbolId, bigint>>;
  mine: Partial<Record<SymbolId, bigint>>;
  faces: number[] | null;
}) {
  return (
    <div className="cloth">
      <div className="bgrid" role="group" aria-label="The board: pick a symbol">
        {SYMBOLS.map((s) => {
          const k = faces ? matches(s.id, faces) : null;
          const cls = ["tile", selected === s.id ? "sel" : "", k !== null ? (k >= 2 ? "hit" : "dim") : ""].join(" ").trim();
          return (
            <button key={s.id} type="button" className={cls} disabled={disabled} aria-pressed={selected === s.id} onClick={() => onSelect(s.id)}>
              <span className="nm">{s.name}</span>
              <SymbolIcon name={s.name} />
              {k === null && (totals[s.id] ?? 0n) > 0n && <span className="stk">${formatAmount(totals[s.id]!, 0)}</span>}
              {k !== null && <span className="cnt">×{k}</span>}
              {(mine[s.id] ?? 0n) > 0n && <span className="you">You ${formatAmount(mine[s.id]!, 0)}</span>}
            </button>
          );
        })}
      </div>
    </div>
  );
}
