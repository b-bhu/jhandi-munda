import { SectionRule } from "@/components/ui/SectionRule";
import { IMAGES } from "@/lib/images";
import { Photo } from "./Photo";

export function Dealer() {
  const d = IMAGES.dealer;
  return (
    <section className="blk">
      <SectionRule n="03">The dealer</SectionRule>
      <h2>Watch the bowl, not the wallet.</h2>
      <p className="lead">The dealer shakes the covered bowl, lifts the lid, and the six dice show. Late arrivals see the result too, never a blank board.</p>
      <div className="two two-dealer">
        <div className="photo"><div className="dealer" role="img" aria-label={d.alt} style={{ backgroundImage: `url(${d.src})` }} /></div>
        <Photo k="diceAndBowl" />
      </div>
    </section>
  );
}
