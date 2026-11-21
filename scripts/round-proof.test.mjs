// Run: node --test scripts/round-proof.test.mjs   (no packages needed)
import { test } from "node:test";
import assert from "node:assert/strict";
import { callbackProof, explorerUrl, roundProof } from "./lib/round-proof.mjs";

const SIG = "5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc".padEnd(88, "A"); // shape only, not a real tx
const REQUEST = "ab".repeat(32);

test("explorerUrl builds a devnet transaction link", () => {
  const url = explorerUrl(SIG);
  assert.equal(url, `https://explorer.solana.com/tx/${SIG}?cluster=devnet`);
  assert.ok(url.length > 0);
});

test("explorerUrl refuses something that is not a signature", () => {
  assert.throws(() => explorerUrl("not a signature"), /not a Solana transaction signature/);
  assert.throws(() => explorerUrl(undefined), /not a Solana transaction signature/);
});

test("callbackProof reads the request and faces from the callback log", () => {
  const logs = ["Program log: Instruction: CallbackRollDice", `Program log: VRF callback: round 0 request ${REQUEST} faces [2, 2, 5, 6, 1, 3]`];
  const p = callbackProof(logs);
  assert.equal(p.request, REQUEST);
  assert.deepEqual(p.faces, [2, 2, 5, 6, 1, 3]);
  assert.equal(callbackProof(["Program log: something else"]), null);
});

test("roundProof returns faces, bet summaries and a non-empty callback link", () => {
  const proof = roundProof({
    faces: [2, 2, 5, 6, 1, 3],
    bets: [{ player: "Alice1111111111111111111111111111111111111", symbol: 2, amount: 10_000_000, settled: true }],
    callbackSignature: SIG,
    callbackLogs: [`Program log: VRF callback: round 0 request ${REQUEST} faces [2, 2, 5, 6, 1, 3]`],
  });
  assert.deepEqual(proof.faces, [2, 2, 5, 6, 1, 3]);
  assert.equal(proof.bets.length, 1);
  assert.equal(proof.bets[0].symbol, 2);
  assert.ok(proof.callbackUrl.startsWith("https://explorer.solana.com/tx/"));
  assert.equal(proof.callbackRequest, REQUEST);
  assert.equal(proof.callbackFacesMatch, true);
});
