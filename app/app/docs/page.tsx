import { SectionRule } from "@/components/ui/SectionRule";
import { Panel } from "@/components/ui/Panel";
import { REPO_URL } from "@/components/shell/TopBar";
import { PAYOUT_ROWS } from "@/lib/payouts";

export const metadata = { title: "Docs · Jhandi Munda" };

const PROGRAM_ID = "4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH";
const blob = (path: string) => `${REPO_URL}/blob/main/${path}`;

const READ = [
  { path: "README.md", name: "README", what: "What it is, the pinned toolchain and how to build and run it." },
  { path: "docs/decisions.md", name: "Decisions", what: "Every design decision (D1 to D51) with the reason, including the payout table, fees, claims and MUNDA." },
  { path: "docs/build-plan.md", name: "Build plan", what: "The step-by-step plan, with a done-when for each step." },
  { path: "docs/er-design.md", name: "Ephemeral Rollup design", what: "Which accounts run on the rollup, which stay on Solana, and the open points." },
  { path: "docs/deploys.md", name: "Devnet deploys", what: "Each devnet deploy with its transaction links." },
];

export default function DocsPage() {
  return (
    <div className="page">
      <SectionRule n="DOCS">Where to read more</SectionRule>
      <h1>How it works, <em>in the repo.</em></h1>
      <p className="dim">The full documents live on GitHub, next to the code. Devnet only: MUSD is a test token with no value, and nothing here promises returns.</p>
      <Panel title="In one minute">
        <ul className="plain">
          <li>Players pick one of six symbols. Six dice are rolled from MagicBlock&apos;s random number service (VRF). A symbol with 2 or more matching dice wins: {PAYOUT_ROWS.map((r) => `${r.multiplier}x`).join(", ")} back for 2 to 6 matches. 0 or 1 match pays nothing.</li>
          <li>A 2% fee is taken from a winner&apos;s profit, and from the pool side on a losing bet. Wins are paid in MUSD: Wait for the full amount, or Take now for 95%.</li>
          <li>A loss is not a refund. It becomes MUNDA, which goes straight into your stake after 24 hours. Losses have Wait only. Chip bets earn MUNDA too.</li>
          <li>Anyone can put test MUSD in the house pool and get shares at the current NAV. If the pool cannot pay a win, the win joins a first-in, first-out queue.</li>
        </ul>
      </Panel>
      <Panel title="Read the documents">
        <ul className="docs-list">
          {READ.map((d) => (
            <li key={d.path}>
              <a href={blob(d.path)} target="_blank" rel="noreferrer"><b>{d.name}</b> ↗</a>
              <span className="dim">{d.what}</span>
            </li>
          ))}
        </ul>
      </Panel>
      <Panel title="On-chain addresses (devnet)">
        <div className="kv"><span>Program</span><b className="mono"><a href={`https://explorer.solana.com/address/${PROGRAM_ID}?cluster=devnet`} target="_blank" rel="noreferrer">{PROGRAM_ID} ↗</a></b></div>
        <p className="note">The source for everything on this site is at <a href={REPO_URL} target="_blank" rel="noreferrer">{REPO_URL.replace("https://", "")}</a>.</p>
      </Panel>
    </div>
  );
}
