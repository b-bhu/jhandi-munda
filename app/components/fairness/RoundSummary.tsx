import type { FairnessProof } from "@/lib/types";
import { formatMusd } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { SymbolIcon } from "@/components/table/SymbolIcon";
import { DiceRow } from "@/components/table/DiceRow";
import { matches } from "@/lib/payouts";
import { SYMBOLS } from "@/lib/symbols";

const Tag = ({ proof }: { proof: FairnessProof }) => <span className="mock" style={proof.isExample ? undefined : { borderStyle: "solid" }}>{proof.isExample ? "Example" : "Real · devnet"}</span>;

/** "The six faces" (with the winning symbols outlined) next to "Round summary". */
export function RoundSummary({ proof }: { proof: FairnessProof }) {
  const won = proof.bets.filter((b) => b.owed > 0n);
  const paid = won.reduce((a, b) => a + b.owed, 0n);
  return (
    <div className="grid2 even">
      <CornerBox className="panel">
        <div className="ptitle"><span>The six faces</span><Tag proof={proof} /></div>
        <DiceRow faces={proof.faces} mark />
        <p className="note" style={{ marginTop: 16 }}>
          Faces [{proof.faces.join(", ")}]. Faces are numbered 1 to 6; the pictures follow the board order
          (heart, spade, diamond, club, crown, flag). The program keeps random bytes below 252 only, so no face is likelier than another, and turns each into a face with <span className="mono">byte % 6 + 1</span>.
        </p>
      </CornerBox>
      <CornerBox className="panel">
        <div className="ptitle"><span>Round summary</span><Tag proof={proof} /></div>
        <div className="kv"><span>Round</span><b>{proof.roundId}</b></div>
        <div className="kv"><span>Network</span><b>Solana devnet</b></div>
        <div className="kv"><span>{proof.bets.length === 1 ? "Bet" : "Bets"}</span><b>{proof.bets.length === 1 ? `one ${proof.bets[0].road === "chips" ? "chip " : ""}bet · ${proof.bets[0].matches} dice matched` : `${proof.bets.length} bets`}</b></div>
        <div className="kv"><span>Result</span><b className={won.length ? "g" : ""}>{won.length ? `won ${formatMusd(paid)}` : "no winner"}</b></div>
        <div className="kv"><span>Payout</span><b>{proof.steps.some((s) => s.label.startsWith("Queued win")) ? "queued, then paid (pay_queue)" : won.length ? "paid by Wait or Take now" : "none"}</b></div>
      </CornerBox>
    </div>
  );
}

/** "Matches per symbol": every symbol, the face number, how many dice show it, and what it pays. */
export function MatchesTable({ proof }: { proof: FairnessProof }) {
  const mult: Record<number, string> = { 2: "2x profit (3x back)", 3: "4x profit (5x back)", 4: "8x profit (9x back)", 5: "25x profit (26x back)", 6: "100x profit (101x back)" };
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>Matches per symbol</span><span className="mock" style={proof.isExample ? undefined : { borderStyle: "solid" }}>{proof.isExample ? "Example" : "Real · from faces"}</span></div>
      <div className="scroll-x" tabIndex={0} role="region" aria-label="Matches per symbol table, scrolls sideways on small screens">
        <table className="tbl">
          <thead><tr><th><span className="sr-only">Picture</span></th><th>Symbol</th><th>Face</th><th>Dice</th><th>Pays</th></tr></thead>
          <tbody>
            {SYMBOLS.map((s) => {
              const k = matches(s.id, proof.faces);
              return (
                <tr key={s.id}>
                  <td><span className="mini"><SymbolIcon name={s.name} /></span></td>
                  <td>{s.label}</td>
                  <td className="m">face {s.id}</td>
                  <td className="m">{k}</td>
                  <td className="m">{k >= 2 ? mult[k] : "lose"}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </CornerBox>
  );
}
