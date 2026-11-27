import type { ProofStep } from "@/lib/types";
import { explorerTx, shortKey } from "@/lib/format";

/** Every step with its Explorer link (devnet cluster). A step without a link says so. */
export function ProofSteps({ steps }: { steps: ProofStep[] }) {
  return (
    <ol className="steps">
      {steps.map((st, i) => (
        <li key={i}>
          <b>{st.label}</b>
          <span className="dim small">{st.detail}</span>
          {st.tx ? (
            <a href={explorerTx(st.tx)} target="_blank" rel="noreferrer" className="mono">{shortKey(st.tx)} ↗ Explorer</a>
          ) : (
            <span className="dim small">no transaction to link</span>
          )}
        </li>
      ))}
    </ol>
  );
}
