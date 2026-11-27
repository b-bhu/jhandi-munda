"use client";
import { useSearchParams } from "next/navigation";
import { isMock, type TableState as MockState } from "@/lib/game-client";
import { SectionRule } from "@/components/ui/SectionRule";
import { ExampleTag } from "@/components/shell/ExampleTag";
import "@/styles/app-mock.css";
import "@/styles/app-extra.css";
import { TableState } from "./TableState";

const SINGLE: MockState[] = ["open", "rolling", "win", "loss", "result"];

/**
 * /play. In mock mode `?state=open|rolling|win|loss|result` shows one example phase, and `?state=all`
 * stacks A, B and C like the design mock (for screenshots). It is ignored against the real program.
 */
export function PlayClient() {
  const raw = useSearchParams().get("state");
  const mock = isMock();
  const all = mock && raw === "all";
  const one = mock && SINGLE.includes(raw as MockState) ? (raw as MockState) : undefined;
  return (
    <div className="app">
      <div className="ph">
        <SectionRule n="TABLE 1">Round 13 <ExampleTag /></SectionRule>
        <h1>Pick a symbol. <em>Watch the bowl.</em></h1>
        <p className="lead">Public 90-second table on a MagicBlock Ephemeral Rollup. Bets are test MUSD on Solana devnet.</p>
      </div>
      {all ? (
        <>
          {([["A", "Betting open", "open"], ["B", "Bowl covered · rolling", "rolling"], ["C", "Result and claims", "result"]] as const).map(([k, label, st]) => (
            <section className="state" key={k} aria-label={`State ${k}: ${label}`}>
              <div className="stlabel"><b>State {k}</b>{label}<span className="x" /></div>
              <TableState mock={st} preselect={2} />
            </section>
          ))}
        </>
      ) : (
        <section className="state" style={{ marginTop: 32 }}><TableState mock={one} preselect={mock ? 2 : undefined} /></section>
      )}
    </div>
  );
}
