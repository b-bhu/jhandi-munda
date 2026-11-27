import { isMock } from "@/lib/game-client";

/** Shown on every page in mock mode, so a screenshot can never pass for live data. */
export function MockStrip() {
  if (!isMock()) return null;
  return <div className="mock-strip" role="status"><span>Demo · test chips</span><span style={{ flex: 1, border: 0, padding: 0 }} /></div>;
}
