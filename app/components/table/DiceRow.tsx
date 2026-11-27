import { symbolName } from "@/lib/symbols";
import { matches } from "@/lib/payouts";
import { SymbolIcon } from "./SymbolIcon";

/** Six dice. With no faces yet they show "?" (the bowl is covered); `mark` outlines the dice of a winning symbol. */
export function DiceRow({ faces, mark }: { faces: number[] | null; mark?: boolean }) {
  if (!faces) {
    return <div className="dice" role="img" aria-label="Six dice under the bowl, not rolled yet">{Array.from({ length: 6 }, (_, i) => <div key={i} className="die q" aria-hidden>?</div>)}</div>;
  }
  return (
    <div className="dice" role="img" aria-label={`The dice: ${faces.map(symbolName).join(", ")}`}>
      {faces.map((f, i) => (
        <div key={i} className={`die${mark && matches(f, faces) >= 2 ? " hit" : ""}`}><SymbolIcon name={symbolName(f)} /></div>
      ))}
    </div>
  );
}
