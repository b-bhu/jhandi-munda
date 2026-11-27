"use client";
import { useEffect, useRef, useState } from "react";

/** Play / pause, drag, step, or jump to a round number. `value` is a 0-based position, `ids` the round numbers. */
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

  const stop = () => setPlaying(false);
  const go = (e: React.FormEvent) => { e.preventDefault(); const i = ids.indexOf(Number(jump)); if (i >= 0) { stop(); onChange(i); } };

  return (
    <>
      <div className="scrub">
        <button type="button" className="play" aria-label={playing ? "Pause" : "Play"} onClick={() => { if (!playing && value >= max) onChange(0); setPlaying((p) => !p); }}>{playing ? "❚❚" : "▶"}</button>
        <input type="range" className="seek" min={0} max={max} value={value} onChange={(e) => { stop(); onChange(Number(e.target.value)); }} aria-label="Round" aria-valuetext={`Round ${ids[value]} of ${ids[max]}`} />
        <span className="small" style={{ font: "600 13px var(--mono)", whiteSpace: "nowrap" }}>Round {ids[value]} / {ids[max]}</span>
      </div>
      <div className="scrub-extra">
        <button type="button" className="step" onClick={() => { stop(); onChange(Math.max(0, value - 1)); }} disabled={value <= 0} aria-label="Previous round">‹</button>
        <button type="button" className="step" onClick={() => { stop(); onChange(Math.min(max, value + 1)); }} disabled={value >= max} aria-label="Next round">›</button>
        <form onSubmit={go} className="jump-form">
          <label htmlFor="jump" className="small">Go to round</label>
          <input id="jump" className="jump" inputMode="numeric" value={jump} onChange={(e) => setJump(e.target.value)} placeholder={`${ids[0]}–${ids[max]}`} />
          <button type="submit" className="step wide" disabled={jump === "" || ids.indexOf(Number(jump)) < 0}>Go</button>
        </form>
      </div>
    </>
  );
}
