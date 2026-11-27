import Link from "next/link";
import { WalletPill } from "./WalletPill";

export const REPO_URL = "https://github.com/b-bhu/jhandi-munda";

const LINKS = [
  { href: "/play", label: "Play" },
  { href: "/pool", label: "Pool" },
  { href: "/fairness/12", label: "Fairness" },
  { href: "/replay", label: "Replay" },
  { href: "/docs", label: "Docs" },
];

export function TopBar() {
  return (
    <header className="topbar">
      <Link className="logo" href="/">jhandi munda<i>.</i></Link>
      <nav aria-label="Main">
        {LINKS.map((l) => <Link key={l.href} href={l.href}>{l.label}</Link>)}
        <a href={REPO_URL} target="_blank" rel="noreferrer">GitHub</a>
        <Link href="/get-musd">Get MUSD</Link>
        <WalletPill />
      </nav>
    </header>
  );
}
