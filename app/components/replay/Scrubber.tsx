"use client";
/** Drag or step through the simulated rounds. A native range input keeps it keyboard and touch friendly. */
export function Scrubber({ value, max, onChange }: { value: number; max: number; onChange: (n: number) => void }) {
  return (
    <div className="scrub">
      <button type="button" className="btn ghost" onClick={() => onChange(Math.max(1, value - 1))} disabled={value <= 1} aria-label="Previous round">‹</button>
      <input type="range" min={1} max={max} value={value} onChange={(e) => onChange(Number(e.target.value))} aria-label="Round" />
      <button type="button" className="btn ghost" onClick={() => onChange(Math.min(max, value + 1))} disabled={value >= max} aria-label="Next round">›</button>
    </div>
  );
}
