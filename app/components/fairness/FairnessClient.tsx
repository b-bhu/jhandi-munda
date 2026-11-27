"use client";
import { useEffect, useMemo, useState } from "react";
import { getClient } from "@/lib/game-client";
import type { FairnessProof } from "@/lib/types";
import { SectionRule } from "@/components/ui/SectionRule";
import "@/styles/app-mock.css";
import "@/styles/app-extra.css";
import "@/styles/pool.css";
import { RoundSummary, MatchesTable } from "./RoundSummary";
import { ProofSteps } from "./ProofSteps";

export function FairnessClient({ roundId }: { roundId: number }) {
  const client = useMemo(() => getClient(), []);
  const [proof, setProof] = useState<FairnessProof | null>(null);

  useEffect(() => {
    let live = true;
    client.getFairness(roundId).then((p) => live && setProof(p));
    return () => { live = false; };
  }, [client, roundId]);

  if (!proof) return <div className="app"><p className="small" role="status">Loading the proof…</p></div>;
  return (
    <div className="app">
      <div className="ph">
        <SectionRule n="FAIRNESS">/fairness/{proof.roundId}</SectionRule>
        <h1>Round {proof.roundId}. <em>Check it yourself.</em></h1>
        <p className="lead">Every settled round has a public page: the six faces, the bets, and the transactions. The dice come from MagicBlock’s random number service, not from us. {proof.isExample ? "This round is an example: made-up data, no links." : "Round and signatures below are real devnet data."}</p>
      </div>
      <section className="state" style={{ marginTop: 28 }}><RoundSummary proof={proof} /></section>
      <section className="state" style={{ marginTop: 24 }}><ProofSteps steps={proof.steps} roundId={proof.roundId} example={proof.isExample} /></section>
      <section className="state" style={{ marginTop: 24 }}><MatchesTable proof={proof} /></section>
    </div>
  );
}
