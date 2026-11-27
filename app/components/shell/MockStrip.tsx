import { isMock } from "@/lib/game-client";

/** Shown on every page in mock mode, so a screenshot can never pass for live data. */
export function MockStrip() {
  if (!isMock()) return null;
  return <div className="mock-strip" role="status">UI SHELL · MOCK DATA</div>;
}
