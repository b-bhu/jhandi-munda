import { SectionRule } from "@/components/ui/SectionRule";
import { ExampleTag } from "@/components/shell/ExampleTag";

export function Steps() {
  return (
    <section className="blk" id="how">
      <SectionRule n="01">How a round works</SectionRule>
      <h2>Three steps to a bowl reveal.</h2>
      <p className="lead">You need Phantom, set to the test network (devnet). Rounds run on MagicBlock&apos;s fast lane, so bets go through quickly. Approve a session once and you can bet without a popup on every click.</p>
      <ol className="pipe">
        <li><span className="pn">[1]</span><h3>Pick a symbol, place a bet</h3><p>Heart, spade, diamond, club, crown or flag. $1 minimum. Join the open table or start one with your bet.</p><code>bet: crown · 5 MUSD <ExampleTag /></code></li>
        <li><span className="pn">[2]</span><h3>The dealer covers the bowl</h3><p>The table stays open for 90 seconds. Then the random number service rolls six dice at once.</p><code>round 12 · 90 s · dice requested</code></li>
        <li><span className="pn">[3]</span><h3>The bowl opens</h3><p>Count the dice that show your symbol. Two or more is a win. Zero or one is a loss.</p><code>faces: 2 · 2 · 4 · 6 · 4 · 5</code></li>
      </ol>
    </section>
  );
}
