import Link from "next/link";
import { WalletPill } from "./WalletPill";

export const REPO_URL = "https://github.com/b-bhu/jhandi-munda";
export const DOCS_URL = `${REPO_URL}#readme`; // the README until /docs exists (#150)

const LINKS = [
  { href: "/play", label: "Play" },
  { href: "/pool", label: "Pool" },
  { href: "/fairness/12", label: "Fairness" },
  { href: "/replay", label: "Replay" },
];

export function TopBar() {
  return (
    <header className="topbar">
      <Link className="logo" href="/">jhandi munda<i>.</i></Link>
      <nav aria-label="Main">
        {LINKS.map((l) => <Link key={l.href} href={l.href}>{l.label}</Link>)}
        <a href={DOCS_URL} target="_blank" rel="noreferrer">Docs</a>
        <a href={REPO_URL} target="_blank" rel="noreferrer">GitHub</a>
        <Link href="/get-musd">Get MUSD</Link>
        <WalletPill />
      </nav>
    </header>
  );
}
