/*
 * The only place money gets formatted. MUSD and MUNDA both have 6 decimals; amounts travel as
 * bigint base units and are never turned into floats.
 */
export const DECIMALS = 6;
const ONE = 10n ** BigInt(DECIMALS);

/** "1,234.56": whole part with commas and `dp` decimals (rounded half up). Default 2. */
export function formatAmount(base: bigint, dp = 2): string {
  const neg = base < 0n;
  let v = neg ? -base : base;
  const scale = 10n ** BigInt(DECIMALS - dp);
  v = (v + scale / 2n) / scale; // round to dp decimals
  const unit = 10n ** BigInt(dp);
  const whole = (v / unit).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const frac = dp > 0 ? "." + (v % unit).toString().padStart(dp, "0") : "";
  return (neg ? "-" : "") + whole + frac;
}

export const formatMusd = (base: bigint, dp = 2) => `${formatAmount(base, dp)} MUSD`;
export const formatMunda = (base: bigint, dp = 2) => `${formatAmount(base, dp)} MUNDA`;

/** Parses a typed amount like "12.5" to base units. Returns null for anything else. */
export function parseAmount(text: string): bigint | null {
  const m = /^(\d{1,12})(?:\.(\d{1,6}))?$/.exec(text.trim());
  if (!m) return null;
  return BigInt(m[1]) * ONE + BigInt((m[2] ?? "").padEnd(DECIMALS, "0"));
}

/** "1.25x" style multiplier from basis points of 1 (e.g. NAV 1.25 = 1_250_000 / 1_000_000). */
export function formatRatio(num: bigint, den: bigint, dp = 4): string {
  if (den === 0n) return "1.0000";
  const scaled = (num * 10n ** BigInt(dp)) / den;
  const unit = 10n ** BigInt(dp);
  return `${scaled / unit}.${(scaled % unit).toString().padStart(dp, "0")}`;
}

/** "mm:ss" for a countdown in whole seconds (never negative). */
export function formatClock(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
}

/** "23h 59m" / "4m 10s" for a wait that can be long. */
export function formatWait(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  if (s >= 3600) return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
  if (s >= 60) return `${Math.floor(s / 60)}m ${s % 60}s`;
  return `${s}s`;
}

/** First 8 and last 6 characters of a transaction signature. */
export const shortSig = (sig: string) => (sig.length > 16 ? `${sig.slice(0, 8)}…${sig.slice(-6)}` : sig);

export const shortKey = (key: string) => (key.length > 10 ? `${key.slice(0, 4)}…${key.slice(-4)}` : key);

export const explorerTx = (sig: string) => `https://explorer.solana.com/tx/${sig}?cluster=devnet`;
export const explorerAddress = (addr: string) => `https://explorer.solana.com/address/${addr}?cluster=devnet`;
