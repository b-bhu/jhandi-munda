import type { RoundStatus } from "@/lib/types";

/* The steel bowl with its plate lid. Shut while the window is open, shaking while the dice roll,
   lifted away once they have rolled. Motion is switched off for prefers-reduced-motion. */
export function Bowl({ status }: { status: RoundStatus }) {
  const label = status === "rolled" ? "The bowl is lifted" : status === "rolling" ? "The dealer shakes the bowl" : "The bowl is covered";
  return (
    <div className={`bowl ${status}`} role="img" aria-label={label}>
      <svg viewBox="0 0 160 110" width="160" height="110">
        <g className="bowl-lid"><ellipse cx="80" cy="44" rx="62" ry="12" fill="#b9b3a6" stroke="#6d675c" strokeWidth="2" /></g>
        <path d="M18 48c0 34 28 54 62 54s62-20 62-54z" fill="#8f8a7e" stroke="#6d675c" strokeWidth="2" />
        <ellipse cx="80" cy="48" rx="62" ry="10" fill="#a7a296" stroke="#6d675c" strokeWidth="2" />
      </svg>
      <span className="dim small">{label}</span>
    </div>
  );
}
