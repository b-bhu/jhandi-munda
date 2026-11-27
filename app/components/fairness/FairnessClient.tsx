"use client";
import { useEffect, useMemo, useState } from "react";
import { getClient } from "@/lib/game-client";
import type { FairnessProof } from "@/lib/types";
import { Panel } from "@/components/ui/Panel";
import "@/styles/fairness.css";
import { FacesRow } from "./FacesRow";
import { RoundSummary } from "./RoundSummary";
import { ProofSteps } from "./ProofSteps";
import { MatchesTable } from "./MatchesTable";

export function FairnessClient({ roundId }: { roundId: number }) {
  const client = useMemo(() => getClient(), []);
  const [proof, setProof] = useState<FairnessProof | null>(null);

  useEffect(() => {
    let live = true;
    client.getFairness(roundId).then((p) => live && setProof(p));
    return () => { live = false; };
  }, [client, roundId]);

  if (!proof) return <div className="page">Loading the proof…</div>;
  const tag = proof.isExample ? undefined : <span className="tag real">Real test round</span>;
  return (
    <div className="page">
      <h1>Fairness: round {proof.roundId}</h1>
      <p className="dim">Check the dice yourself: every step below is a test-network transaction you can open on Solana Explorer.</p>
      <Panel title="The dice" example={proof.isExample} tag={tag}>
        <FacesRow faces={proof.faces} />
        <RoundSummary proof={proof} />
      </Panel>
      <Panel title="Proof trail" example={proof.isExample} tag={tag}><ProofSteps steps={proof.steps} /></Panel>
      <Panel title="Bets and matches" example={proof.isExample} tag={tag}><MatchesTable bets={proof.bets} /></Panel>
    </div>
  );
}
