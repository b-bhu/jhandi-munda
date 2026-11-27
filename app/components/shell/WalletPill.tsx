"use client";
import { useEffect } from "react";
import { connectWallet, disconnectWallet, useWallet, verifyCluster } from "@/lib/wallet";
import { shortKey } from "@/lib/format";

/** The wallet pill in the top bar. It stays disabled unless the devnet guard has passed. Connects only when the user clicks, never on load. */
export function WalletPill() {
  const w = useWallet();

  useEffect(() => {
    void verifyCluster();
  }, []);

  if (w.address) {
    return <button type="button" className="pill on" onClick={disconnectWallet} title="Disconnect">● {shortKey(w.address)}</button>;
  }
  if (w.guard === "checking") return <button type="button" className="pill" disabled>Checking the test network…</button>;
  if (w.guard === "blocked") {
    return <span className="pill bad" role="alert" title="This table only runs on the test network, and that isn't confirmed yet.">Can&apos;t connect</span>;
  }
  if (w.guard === "unknown") {
    return <button type="button" className="pill" onClick={verifyCluster} title="Couldn't confirm the test network. Connecting stays paused until it can.">Try again</button>;
  }
  return (
    <span className="wallet-wrap">
      <button type="button" className="pill" onClick={() => connectWallet()} disabled={w.busy}>{w.busy ? "Connecting…" : "Connect wallet"}</button>
      <span className="wallet-msg" role="status">{w.error ?? "Use Phantom on devnet"}</span>
    </span>
  );
}
