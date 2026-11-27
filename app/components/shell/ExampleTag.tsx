import { isMock } from "@/lib/game-client";

/** Marks a panel that is showing made-up numbers. Renders nothing against the real program. */
export function ExampleTag({ label = "Example" }: { label?: string }) {
  if (!isMock()) return null;
  return <span className="tag">{label}</span>;
}
