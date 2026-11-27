"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";

const LINKS = [
  { href: "/play", label: "Play" },
  { href: "/pool", label: "Pool" },
  { href: "/fairness/12", label: "Fairness", match: "/fairness" },
  { href: "/replay", label: "Replay" },
  { href: "/docs", label: "Docs" },
];

/** The in-app links, with aria-current on the page you are on. */
export function NavLinks() {
  const path = usePathname();
  return (
    <>
      {LINKS.map((l) => (
        <Link key={l.href} href={l.href} aria-current={path.startsWith(l.match ?? l.href) ? "page" : undefined}>{l.label}</Link>
      ))}
    </>
  );
}
