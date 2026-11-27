"use client";
import { useEffect, useRef, useState } from "react";

/** Play, pause, drag, step, or jump to a round number. `value` is a 0-based position, `ids` the round numbers. */
export function Scrubber({ value, ids, onChange }: { value: number; ids: number[]; onChange: (i: number) => void }) {
  const max = ids.length - 1;
  const [playing, setPlaying] = useState(false);
  const [jump, setJump] = useState("");
  const at = useRef(value);
  useEffect(() => { at.current = value; }, [value]);

  useEffect(() => {
    if (!playing) return;
    const id = setInterval(() => {
      if (at.current >= max) { setPlaying(false); return; }
      onChange(at.current + 1);
    }, 700);
    return () => clearInterval(id);
  }, [playing, max, onChange]);

  function go(e: React.FormEvent) {
    e.preventDefault();
    const i = ids.indexOf(Number(jump));
    if (i >= 0) { setPlaying(false); onChange(i); }
  }

  return (
    <div className="scrub-wrap">
      <div className="scrub">
        <button type="button" className="btn ghost" onClick={() => { setPlaying(false); onChange(Math.max(0, value - 1)); }} disabled={value <= 0} aria-label="Previous round">‹</button>
        <input type="range" min={0} max={max} value={value} onChange={(e) => { setPlaying(false); onChange(Number(e.target.value)); }} aria-label="Round" aria-valuetext={`Round ${ids[value]}`} />
        <button type="button" className="btn ghost" onClick={() => { setPlaying(false); onChange(Math.min(max, value + 1)); }} disabled={value >= max} aria-label="Next round">›</button>
      </div>
      <div className="row scrub-row">
        <button type="button" className="btn" onClick={() => { if (!playing && value >= max) onChange(0); setPlaying((p) => !p); }}>{playing ? "Pause" : "Play"}</button>
        <form onSubmit={go} className="row">
          <label htmlFor="jump" className="dim small">Go to round</label>
          <input id="jump" className="input jump" inputMode="numeric" value={jump} onChange={(e) => setJump(e.target.value)} placeholder={`${ids[0]}–${ids[max]}`} />
          <button type="submit" className="btn ghost" disabled={ids.indexOf(Number(jump)) < 0 || jump === ""}>Go</button>
        </form>
      </div>
    </div>
  );
}
