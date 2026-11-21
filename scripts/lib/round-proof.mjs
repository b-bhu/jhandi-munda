// Proof trail for a settled round (#53, D24, D11). Pure functions: no network, no keys.
//
// The VRF callback's transaction log names the request and the six faces ("VRF callback: round N
// request <hex> faces [..]", see callback_roll.rs). The Explorer link to that transaction is the
// proof a fairness page shows. Nothing here is stored on-chain beyond the faces the round holds.

const SIGNATURE = /^[1-9A-HJ-NP-Za-km-z]{43,90}$/;

// Solana Explorer link for a transaction. Devnet by default.
export function explorerUrl(signature, cluster = "devnet") {
  if (typeof signature !== "string" || !SIGNATURE.test(signature)) {
    throw new Error(`not a Solana transaction signature: ${signature}`);
  }
  return `https://explorer.solana.com/tx/${signature}?cluster=${cluster}`;
}

// Finds the callback's log line and reads the request and the faces from it. Returns null when the
// logs have no callback line.
export function callbackProof(logs) {
  const line = (logs ?? []).find((l) => l.includes("VRF callback:"));
  if (!line) return null;
  const request = /request ([0-9a-f]{64})/.exec(line)?.[1] ?? null;
  const faces = /faces \[([\d,\s]+)\]/.exec(line)?.[1]
    .split(",")
    .map((x) => Number(x.trim())) ?? null;
  return { line, request, faces };
}

// The fairness view of a settled round: faces, bet summaries, and the callback's link.
// `bets` are { player (base58), symbol, amount (base units), settled }.
export function roundProof({ faces, bets, callbackSignature, callbackLogs }) {
  const proof = callbackProof(callbackLogs);
  return {
    faces,
    bets: bets.map((b) => ({ player: b.player, symbol: b.symbol, amount: b.amount, settled: b.settled })),
    callbackUrl: explorerUrl(callbackSignature),
    callbackRequest: proof?.request ?? null,
    callbackFacesMatch: proof?.faces ? proof.faces.join(",") === faces.join(",") : null,
  };
}
