import { SYMBOLS, symbolName } from "@/lib/symbols";
import "@/styles/dice.css";
import { SymbolIcon } from "@/components/table/SymbolIcon";

/** The six dice, each with its number and symbol name, so a face can be checked against the proof. */
export function FacesRow({ faces }: { faces: number[] }) {
  return (
    <ol className="faces" aria-label="The six faces">
      {faces.map((f, i) => (
        <li key={i} className="face">
          <div className="die"><SymbolIcon name={symbolName(f)} size={28} /></div>
          <span className="small">{f} · {SYMBOLS.find((s) => s.id === f)?.name}</span>
        </li>
      ))}
    </ol>
  );
}
