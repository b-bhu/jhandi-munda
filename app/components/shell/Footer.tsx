import Link from "next/link";
import { REPO_URL } from "./TopBar";

export function Footer() {
  return (
    <footer className="foot">
      <span className="logo">jhandi munda</span>
      <span className="fl">
        <Link href="/play">Play</Link><Link href="/pool">Pool</Link><Link href="/fairness/12">Fairness</Link><Link href="/replay">Replay</Link>
        <Link href="/docs">Docs</Link><a href={REPO_URL} target="_blank" rel="noreferrer">GitHub</a>
      </span>
      <span>Test network only. Chips have no real value. · MagicBlock Solana Blitz v10</span>
    </footer>
  );
}
