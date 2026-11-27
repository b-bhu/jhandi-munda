import type { ProofStep } from "@/lib/types";
import { explorerTx, shortSig } from "@/lib/format";

/** Every step with its number, a plain sentence, and its Explorer link (devnet cluster). */
export function ProofSteps({ steps, roundId, example }: { steps: ProofStep[]; roundId: number; example: boolean }) {
  return (
    <>
      <div className="stlabel"><b>Proof</b>Transactions for round {roundId}<span className="x" /></div>
      <ul className="proof">
        {steps.map((st, i) => (
          <li key={i}>
            <span className="n">{String(i + 1).padStart(2, "0")}</span>
            <span className="w">{st.label}<small>{st.detail}</small></span>
            <span className="c">Solana devnet</span>
            {st.tx ? <a href={explorerTx(st.tx)} target="_blank" rel="noreferrer">{shortSig(st.tx)} ↗</a> : <span className="small">no transaction to link</span>}
          </li>
        ))}
      </ul>
      <p className="note">{example ? "Example round: there are no real transactions behind these steps." : "Every link opens Solana Explorer on devnet."} A proof blob from the VRF, if available, will show here (#54).</p>
    </>
  );
}
