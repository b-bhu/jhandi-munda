import Link from "next/link";
import Image from "next/image";
import { IMAGES } from "@/lib/images";
import { CornerBox } from "@/components/ui/CornerBox";

export function Hero() {
  const im = IMAGES.street;
  return (
    <>
      <div className="film">
        <span className="badge"><i />Solana devnet · test MUSD only</span>
        <Image src={im.src} width={im.width} height={im.height} alt={im.alt} priority unoptimized />
        <Link href="/play" className="cta">Play a round →</Link>
      </div>
      <header className="hero">
        <h1>The street dice game, <em>onchain.</em></h1>
        <p className="sub">Six symbol dice, one covered bowl, a 90-second table. The roll comes from MagicBlock’s random number service, so anyone can check it wasn’t rigged.</p>
        <CornerBox className="ctabox">
          <div className="h">Get free test MUSD, then take a seat</div>
          <div className="b"><code>100 MUSD per click · devnet faucet</code><Link href="/get-musd" className="btn gbtn">Get MUSD</Link></div>
        </CornerBox>
        <p className="hint"><i />Hackathon demo · Solana devnet only · no real money</p>
        <a className="alt" href="#how">Or read how a round works ↓</a>
      </header>
    </>
  );
}
