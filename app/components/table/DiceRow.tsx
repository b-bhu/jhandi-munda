import { symbolName } from "@/lib/symbols";
import "@/styles/dice.css";
import { SymbolIcon } from "./SymbolIcon";

/** The six dice after the roll. Each shows its symbol; the order is the order the program stored. */
export function DiceRow({ faces }: { faces: number[] }) {
  return (
    <ol className="dice" aria-label={`Dice: ${faces.map(symbolName).join(", ")}`}>
      {faces.map((f, i) => (
        <li key={i} className="die" style={{ animationDelay: `${i * 90}ms` }}>
          <SymbolIcon name={symbolName(f)} size={30} />
        </li>
      ))}
    </ol>
  );
}
