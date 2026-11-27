import { SectionRule } from "@/components/ui/SectionRule";
import { LANDING_PROOFS } from "@/lib/proof-links";
import { explorerTx, shortSig } from "@/lib/format";

export function Proof() {
  return (
    <section className="blk" id="proof">
      <SectionRule n="06">Proof</SectionRule>
      <h2>Every claim has a transaction.</h2>
      <p className="lead">Each round has a public fairness page: the six faces, the bets, and the Explorer link for the random-number callback.</p>
      <ul className="proof">
        {LANDING_PROOFS.map((p) => (
          <li key={p.sig}>
            <span className="w">{p.label}</span>
            <span className="c">Test network</span>
            <a href={explorerTx(p.sig)} target="_blank" rel="noreferrer" className="mono">{shortSig(p.sig)} ↗</a>
          </li>
        ))}
      </ul>
      <div className="nav3">
        <div className="fact"><b>Fast lane</b>The table runs on MagicBlock’s fast lane. Settled results land back on Solana.</div>
        <div className="fact"><b>Session keys</b>Approve once, then place bets with no wallet popup. Cap and revoke any time.</div>
        <div className="fact"><b>Test chips only</b>Test MUSD from a faucet. No real money, no real USDC, no promised returns.</div>
      </div>
      <p className="warn">Hackathon demo. Gambling games carry risk and may be restricted where you live. Nothing here is financial advice or a promise of returns.</p>
    </section>
  );
}
