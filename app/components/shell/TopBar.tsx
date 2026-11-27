import Link from "next/link";
import { Suspense } from "react";
import { WalletPill } from "./WalletPill";
import { NavLinks } from "./NavLinks";

export const REPO_URL = "https://github.com/b-bhu/jhandi-munda";

export function TopBar() {
  return (
    <header className="topbar">
      <Link className="logo" href="/">jhandi munda<i>.</i></Link>
      <nav aria-label="Main">
        <Suspense fallback={<><Link href="/play">Play</Link><Link href="/pool">Pool</Link><Link href="/fairness/12">Fairness</Link><Link href="/replay">Replay</Link><Link href="/docs">Docs</Link></>}>
          <NavLinks />
        </Suspense>
        <a href={REPO_URL} target="_blank" rel="noreferrer">GitHub</a>
        <Link href="/get-musd">Get MUSD</Link>
      </nav>
      <div className="topbar-wallet"><WalletPill /></div>
    </header>
  );
}
