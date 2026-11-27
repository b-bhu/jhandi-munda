import Link from "next/link";

const LINKS = [
  { href: "/play", label: "Play" },
  { href: "/pool", label: "Pool" },
  { href: "/fairness/12", label: "Fairness" },
  { href: "/replay", label: "Replay" },
  { href: "/get-musd", label: "Get MUSD" },
];

export function TopBar() {
  return (
    <header className="topbar">
      <Link className="brand" href="/">Jhandi Munda · devnet</Link>
      <nav aria-label="Main">
        {LINKS.map((l) => <Link key={l.href} href={l.href}>{l.label}</Link>)}
      </nav>
    </header>
  );
}
