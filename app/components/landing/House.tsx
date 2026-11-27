import { SectionRule } from "@/components/ui/SectionRule";
import { CornerBox } from "@/components/ui/CornerBox";

/* The pool example box. The three numbers are made up and tagged EXAMPLE; the real NAV is on /pool. */
export function House() {
  return (
    <section className="blk" id="house">
      <SectionRule n="05">The house</SectionRule>
      <h2>Anyone can be the house.</h2>
      <p className="lead">Players bet against one shared pool. Deposit test MUSD, get pool shares, and see what they were worth when you bought them and what they’re worth now.</p>
      <div className="two two-even">
        <CornerBox style={{ padding: "22px 24px" }}>
          <div className="eyebrow">Your pool shares <span className="ex">Sample</span></div>
          <div className="kv"><span>You bought at</span><b>$1.00 / share</b></div>
          <div className="kv"><span>Worth now</span><b>$1.25 / share</b></div>
          <div className="kv"><span>Your profit</span><b style={{ color: "var(--green)" }}>+25.0%</b></div>
          <div className="kv"><span>Payout queue</span><b>0 wins waiting</b></div>
          <p className="note">Example only. Live numbers are on the pool page. Pool value can go down as well as up, and you can lose what you put in.</p>
        </CornerBox>
        <div className="facts">
          <div className="fact"><b>Pool shares</b>Deposit MUSD, get shares at today’s price. Withdraw at today’s price, except while wins are waiting to be paid.</div>
          <div className="fact"><b>MUNDA stake</b>Losers get MUNDA. Stakers share 70% of the 2% fee, plus half of pool gains above $10,000.</div>
          <div className="fact"><b>Strict queue</b>When the pool can’t pay, wins queue first-in, first-out and are paid from future losses.</div>
        </div>
      </div>
    </section>
  );
}
