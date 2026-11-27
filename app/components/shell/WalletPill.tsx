"use client";
import { useEffect } from "react";
import { connectWallet, disconnectWallet, useWallet, verifyCluster } from "@/lib/wallet";
import { shortKey } from "@/lib/format";

/** The wallet pill in the top bar. It stays disabled unless the devnet guard has passed. */
export function WalletPill() {
  const w = useWallet();

  useEffect(() => {
    verifyCluster().then(() => connectWallet(true));
  }, []);

  if (w.address) {
    return <button type="button" className="pill on" onClick={disconnectWallet} title="Disconnect">● {shortKey(w.address)}</button>;
  }
  if (w.guard === "checking") return <button type="button" className="pill" disabled>Checking devnet…</button>;
  if (w.guard === "blocked") {
    return <span className="pill bad" role="alert" title="This app only works on Solana devnet. Its RPC is a different cluster, so connecting is blocked.">Not devnet · blocked</span>;
  }
  if (w.guard === "unknown") {
    return <button type="button" className="pill" onClick={verifyCluster} title="Could not confirm the RPC is Solana devnet. Connecting stays blocked until it can.">Retry devnet check</button>;
  }
  return (
    <span className="wallet-wrap">
      <button type="button" className="pill" onClick={() => connectWallet()} disabled={w.busy}>{w.busy ? "Connecting…" : "Connect wallet"}</button>
      <span className="wallet-msg" role="status">{w.error ?? "Devnet only · set your wallet to devnet"}</span>
    </span>
  );
}
