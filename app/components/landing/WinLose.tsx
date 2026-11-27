import { SectionRule } from "@/components/ui/SectionRule";
import { CornerBox } from "@/components/ui/CornerBox";

/* The two result cards, as pictures of the rules. A loss has Wait only: no Take now (D50, #122),
   and it is not a refund. Chip bets earn MUNDA the same way (D51). */
export function WinLose() {
  return (
    <section className="blk" id="win-lose">
      <SectionRule n="04">Win or lose</SectionRule>
      <h2>Every result becomes one claim card.</h2>
      <p className="lead">Wins pay in MUSD. Losses pay in MUNDA, a house token that goes straight into your stake. A loss is not a refund.</p>
      <div className="claims">
        <CornerBox className="claim">
          <span className="tag-l" style={{ color: "var(--gold)" }}>You won</span>
          <h3>Claim your win</h3>
          <p>Wait 90 seconds for the full payout, or take 95% now. If the pool is short, your win joins a first-in, first-out queue.</p>
          <div className="acts"><span className="b1">Wait · 100%</span><span className="b2">Take now · 95%</span></div>
        </CornerBox>
        <CornerBox className="claim">
          <span className="tag-l" style={{ color: "var(--tilak)" }}>You lost</span>
          <h3>You earned MUNDA</h3>
          <p>MUNDA is a share of house fees. This is not a refund. After 24 hours it is yours; it goes straight into your stake. The first $1,000 of losses earn 5x MUNDA. Chip bets earn it too.</p>
          <div className="acts"><span className="b1">Wait · 24h</span><span className="b2 off">Take now · not for losses</span></div>
        </CornerBox>
      </div>
    </section>
  );
}
