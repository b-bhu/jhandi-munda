"use client";
import { useEffect, useState } from "react";

/** Current unix seconds, ticking once a second. Starts at 0 on the server so markup matches. */
export function useNow(): number {
  const [now, setNow] = useState(0);
  useEffect(() => {
    const tick = () => setNow(Math.floor(Date.now() / 1000));
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, []);
  return now;
}
