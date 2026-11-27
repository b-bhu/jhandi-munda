import type { FairnessProof } from "@/lib/types";
import { SYMBOLS } from "@/lib/symbols";
import { matches } from "@/lib/payouts";

export function RoundSummary({ proof }: { proof: FairnessProof }) {
  const counts = SYMBOLS.map((s) => ({ s, n: matches(s.id, proof.faces) })).filter((c) => c.n > 0);
  return (
    <>
      <p>
        Round <b>{proof.roundId}</b>: {counts.map((c) => `${c.s.name} ×${c.n}`).join(", ")}.
        {" "}A symbol with 2 or more dice wins; 0 or 1 pays nothing.
      </p>
      <p className="dim small">
        The faces come from MagicBlock&apos;s random numbers. To keep every symbol equally likely, the program only uses random bytes below 252, then turns each into a face with <span className="mono">byte % 6 + 1</span>.
      </p>
    </>
  );
}
