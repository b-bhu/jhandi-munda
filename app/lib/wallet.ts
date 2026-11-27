/*
 * Wallet connect and the devnet guard. No @solana/web3.js here: the injected wallet (Phantom) is
 * talked to through its plain browser API, and the cluster check is one JSON-RPC call.
 *
 * The guard: before a wallet can connect, the RPC this app uses must report the DEVNET genesis
 * hash. Anything else (mainnet, a typo, no answer) blocks connecting. Fail closed. An injected
 * wallet does not tell a page which network it is set to, so the UI also tells people to set the
 * wallet to devnet.
 */
import { useSyncExternalStore } from "react";

export const DEVNET_GENESIS = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";
export const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL ?? "https://api.devnet.solana.com";

/** devnet: verified. blocked: the RPC is another cluster. unknown: could not ask. checking: asking now. */
export type GuardState = "checking" | "devnet" | "blocked" | "unknown";

/** Asks the RPC for its genesis hash. Takes `fetchFn` so it can be tested without a network. */
export async function checkCluster(url: string = RPC_URL, fetchFn: typeof fetch = fetch): Promise<GuardState> {
  try {
    const res = await fetchFn(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", id: 1, method: "getGenesisHash" }),
    });
    const json = (await res.json()) as { result?: unknown };
    if (typeof json.result !== "string") return "unknown";
    return json.result === DEVNET_GENESIS ? "devnet" : "blocked";
  } catch {
    return "unknown";
  }
}

interface InjectedWallet {
  connect: (opts?: { onlyIfTrusted?: boolean }) => Promise<{ publicKey: { toString(): string } }>;
  disconnect?: () => Promise<void>;
}
function injected(): InjectedWallet | undefined {
  if (typeof window === "undefined") return undefined;
  const win = window as unknown as { phantom?: { solana?: InjectedWallet }; solana?: InjectedWallet };
  const w = win.phantom?.solana ?? win.solana;
  return w?.connect ? w : undefined;
}

export interface WalletState { address: string | null; guard: GuardState; busy: boolean; error: string | null }
const SERVER_STATE: WalletState = { address: null, guard: "checking", busy: false, error: null };
let state: WalletState = SERVER_STATE;
const listeners = new Set<() => void>();
function set(patch: Partial<WalletState>) { state = { ...state, ...patch }; listeners.forEach((l) => l()); }

export async function verifyCluster() {
  set({ guard: "checking", error: null });
  set({ guard: await checkCluster() });
}

/** Connects the injected wallet, but only when the guard says devnet. `silent` reuses an earlier approval. */
export async function connectWallet(silent = false) {
  if (state.guard !== "devnet") return set({ error: "Connecting is blocked until the app is on Solana devnet." });
  const w = injected();
  if (!w) return set({ error: "No wallet found. Install Phantom and set it to Solana devnet." });
  set({ busy: true, error: null });
  try {
    const { publicKey } = await w.connect(silent ? { onlyIfTrusted: true } : undefined);
    set({ address: publicKey.toString(), busy: false });
  } catch (e) {
    set({ busy: false, error: silent ? null : e instanceof Error ? e.message : "The wallet did not connect." });
  }
}

export async function disconnectWallet() {
  try { await injected()?.disconnect?.(); } catch { /* the wallet may already be gone */ }
  set({ address: null, error: null });
}

export function useWallet(): WalletState {
  return useSyncExternalStore(
    (cb) => { listeners.add(cb); return () => { listeners.delete(cb); }; },
    () => state,
    () => SERVER_STATE,
  );
}
