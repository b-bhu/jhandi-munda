#!/usr/bin/env node
/**
 * Devnet VRF proof trail for Step 5.3 (issue #19, D11, D24).
 *
 *   node scripts/devnet-vrf-roll.mjs [keypair.json] [rpc]
 *
 * Initializes the table if needed, places a $1 MUSD bet to open a round,
 * waits until the on-chain Clock is past the 90 s join window, calls
 * `request_roll`, then polls the round until status is Rolled. Prints the
 * six faces, the round address, and Explorer (?cluster=devnet) links for
 * the request and VRF callback / fulfillment transactions.
 *
 * Devnet only: the RPC is refused if it mentions mainnet. The keypair file
 * is a gitignored wallet; the secret is never printed or logged.
 *
 * @solana/web3.js is resolved from app/node_modules (same pin as the UI).
 */

import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const web3 = createRequire(new URL("../app/package.json", import.meta.url))("@solana/web3.js");
const {
  Connection, Keypair, PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY,
  Transaction, TransactionInstruction, sendAndConfirmTransaction,
} = web3;

const PROGRAM_ID = new PublicKey("4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH");
const VRF_PROGRAM_ID = new PublicKey("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz");
const DEFAULT_QUEUE = new PublicKey("Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh");
const SLOT_HASHES = new PublicKey("SysvarS1otHashes111111111111111111111111111");
const ATA_PROGRAM = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const TOKEN_PROGRAM = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

const seed = (s) => new TextEncoder().encode(s);
const pda = (...s) => PublicKey.findProgramAddressSync(s, PROGRAM_ID)[0];
const CONFIG = pda(seed("config"));
const MUSD_MINT = pda(seed("musd_mint"));
const MUSD_AUTH = pda(seed("musd_mint_authority"));
const POOL = pda(seed("pool"));
const VAULT = pda(seed("pool_vault"));
const SHARE_MINT = pda(seed("pool_share_mint"));
const TABLE = pda(seed("table"));
const ROUND = pda(seed("round"));
const PROGRAM_IDENTITY = pda(seed("identity"));
const faucetPda = (w) => pda(seed("faucet"), w.toBytes());
const musdAta = (w) => PublicKey.findProgramAddressSync(
  [w.toBytes(), TOKEN_PROGRAM.toBytes(), MUSD_MINT.toBytes()], ATA_PROGRAM,
)[0];
function betPda(roundId, player, symbol) {
  const id = Buffer.alloc(8);
  id.writeBigUInt64LE(BigInt(roundId));
  return pda(seed("bet"), id, player.toBytes(), Uint8Array.of(symbol));
}

// Anchor 1.0.2: sha256("global:<ix>") first 8 bytes.
const DISC = {
  initializePool: Buffer.from([95, 180, 10, 172, 84, 174, 232, 40]),
  initializeTable: Buffer.from([223, 143, 246, 102, 122, 200, 108, 147]),
  musdFaucet: Buffer.from([46, 201, 59, 88, 189, 101, 81, 3]),
  placeBet: Buffer.from([222, 62, 67, 220, 63, 166, 126, 33]),
  requestRoll: Buffer.from([98, 118, 98, 29, 96, 208, 255, 97]),
  callbackRoll: Buffer.from([129, 76, 217, 160, 252, 234, 19, 238]),
};
const MIN_BET = 1_000_000n;
const ROUND_SECS = 90n;
const CALLBACK_MS = 3 * 60 * 1000;
const DEFAULT_RPC = "https://api.devnet.solana.com";
const DEFAULT_KEYPAIR = "keys/dev-wallet.json";
const STATUS = ["Idle", "Open", "Rolling", "Rolled"];
const explorer = (path) => `https://explorer.solana.com/${path}?cluster=devnet`;
const meta = (pubkey, isSigner, isWritable) => ({ pubkey, isSigner, isWritable });

function loadWallet(keyPath) {
  if (!existsSync(resolve(keyPath))) {
    throw new Error(`No keypair at ${keyPath}. Create one with: solana-keygen new -o ${keyPath} (gitignored, never committed)`);
  }
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(readFileSync(keyPath, "utf8"))));
}

const ix = (keys, data) => new TransactionInstruction({ programId: PROGRAM_ID, keys, data });
const send = (c, w, instruction) => sendAndConfirmTransaction(c, new Transaction().add(instruction), [w]);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function clockUnix(c) {
  const acc = await c.getAccountInfo(SYSVAR_CLOCK_PUBKEY);
  if (!acc) throw new Error("Clock sysvar is missing — unexpected on devnet.");
  // slot u64, epoch_start_timestamp i64, epoch u64, leader_schedule_epoch u64, unix_timestamp i64
  return acc.data.readBigInt64LE(32);
}

function decodeRound(data) {
  let o = 8;
  const status = data[o++];
  const openedAt = data.readBigInt64LE(o); o += 8;
  const roundId = data.readBigUInt64LE(o); o += 8;
  const n = data.readUInt32LE(o); o += 4;
  const players = [];
  for (let i = 0; i < n; i++, o += 32) players.push(new PublicKey(data.subarray(o, o + 32)));
  const requestId = data.subarray(o, o + 32); o += 32;
  const faces = [...data.subarray(o, o + 6)];
  return { status, statusName: STATUS[status] ?? `unknown(${status})`, openedAt, roundId, players, requestId, faces };
}

async function readRound(c) {
  const acc = await c.getAccountInfo(ROUND);
  return acc ? decodeRound(acc.data) : null;
}

/** Same rule as `faces_from_randomness` in callback_roll.rs: skip bytes >= 252. */
function facesFromRandomness(randomness) {
  const faces = [];
  for (const byte of randomness) {
    if (faces.length === 6) break;
    if (byte < 252) faces.push((byte % 6) + 1);
  }
  if (faces.length !== 6) throw new Error("Too few usable VRF bytes to make six faces");
  return faces;
}


function b58decode(s) {
  const A = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
  const bytes = [];
  for (const ch of s) {
    let v = A.indexOf(ch);
    if (v < 0) throw new Error("invalid base58");
    for (let i = 0; i < bytes.length; i++) {
      v += bytes[i] * 58;
      bytes[i] = v & 0xff;
      v >>= 8;
    }
    while (v > 0) {
      bytes.push(v & 0xff);
      v >>= 8;
    }
  }
  for (const ch of s) {
    if (ch !== "1") break;
    bytes.push(0);
  }
  return Buffer.from(bytes.reverse());
}

function ixData(raw) {
  if (Buffer.isBuffer(raw.data) || raw.data instanceof Uint8Array) return Buffer.from(raw.data);
  if (typeof raw.data === "string") return b58decode(raw.data);
  return Buffer.from(raw.data ?? []);
}

function walkInstructions(tx) {
  const message = tx.transaction.message;
  const keys = message.getAccountKeys ? message.getAccountKeys() : null;
  const compiled = message.compiledInstructions ?? message.instructions;
  const programOf = (raw) => {
    if (raw.programId) return raw.programId;
    if (keys && typeof keys.get === "function") return keys.get(raw.programIdIndex);
    return message.accountKeys[raw.programIdIndex];
  };
  const decode = (raw) => ({ programId: programOf(raw).toBase58(), data: ixData(raw) });
  const out = compiled.map(decode);
  for (const inner of tx.meta?.innerInstructions ?? []) {
    for (const raw of inner.instructions) out.push(decode(raw));
  }
  return out;
}

async function findCallback(c, afterSlot) {
  const sigs = await c.getSignaturesForAddress(ROUND, { limit: 30 });
  for (const info of sigs) {
    if (afterSlot != null && info.slot < afterSlot) continue;
    const tx = await c.getTransaction(info.signature, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });
    if (!tx) continue;
    for (const raw of walkInstructions(tx)) {
      if (raw.programId === PROGRAM_ID.toBase58() && raw.data.length >= 72 && DISC.callbackRoll.equals(raw.data.subarray(0, 8))) {
        return { signature: info.signature, randomness: raw.data.subarray(8, 40) };
      }
    }
  }
  return null;
}

async function musdBalance(c, wallet) {
  const accounts = await c.getParsedTokenAccountsByOwner(wallet, { mint: MUSD_MINT });
  return accounts.value.reduce((t, { account }) => t + BigInt(account.data.parsed?.info?.tokenAmount?.amount ?? 0), 0n);
}

async function main() {
  const [keyPath = DEFAULT_KEYPAIR, rpc = DEFAULT_RPC] = process.argv.slice(2);
  if (/mainnet/i.test(rpc)) throw new Error(`Refusing RPC "${rpc}": this script is devnet only.`);
  const wallet = loadWallet(keyPath);
  const c = new Connection(rpc, "confirmed");
  const payer = wallet.publicKey;

  console.log(`RPC: ${rpc}`);
  console.log(`Wallet: ${payer.toBase58()}`);
  console.log(`Program: ${PROGRAM_ID.toBase58()} ${explorer(`address/${PROGRAM_ID.toBase58()}`)}`);
  console.log(`Round PDA: ${ROUND.toBase58()} ${explorer(`address/${ROUND.toBase58()}`)}`);

  if (!(await c.getAccountInfo(CONFIG))) {
    throw new Error("Program not initialized on this cluster yet. Run: node scripts/devnet-musd.mjs init");
  }
  if (!(await c.getAccountInfo(POOL))) {
    const sig = await send(c, wallet, ix([
      meta(CONFIG, false, false), meta(POOL, false, true), meta(SHARE_MINT, false, true),
      meta(VAULT, false, true), meta(MUSD_MINT, false, false), meta(payer, true, true),
      meta(TOKEN_PROGRAM, false, false), meta(SystemProgram.programId, false, false),
    ], DISC.initializePool));
    console.log(`initialize_pool: ${sig} ${explorer(`tx/${sig}`)}`);
  }
  if (!(await c.getAccountInfo(TABLE))) {
    const sig = await send(c, wallet, ix([
      meta(TABLE, false, true), meta(ROUND, false, true),
      meta(payer, true, true), meta(SystemProgram.programId, false, false),
    ], DISC.initializeTable));
    console.log(`initialize_table: ${sig} ${explorer(`tx/${sig}`)}`);
  }

  let round = await readRound(c);
  if (!round) throw new Error("Round account is missing after table init.");

  let requestSig = null;
  let requestSlot = null;

  if (round.statusName === "Idle") {
    if (await musdBalance(c, payer) < MIN_BET) {
      const sig = await send(c, wallet, ix([
        meta(CONFIG, false, false), meta(MUSD_MINT, false, true), meta(MUSD_AUTH, false, false),
        meta(payer, true, true), meta(faucetPda(payer), false, true), meta(musdAta(payer), false, true),
        meta(ATA_PROGRAM, false, false), meta(TOKEN_PROGRAM, false, false), meta(SystemProgram.programId, false, false),
      ], DISC.musdFaucet));
      console.log(`faucet: ${sig} ${explorer(`tx/${sig}`)}`);
    }
    const symbol = 1;
    const data = Buffer.alloc(17);
    DISC.placeBet.copy(data, 0);
    data.writeUInt8(symbol, 8);
    data.writeBigUInt64LE(MIN_BET, 9);
    const sig = await send(c, wallet, ix([
      meta(TABLE, false, false), meta(ROUND, false, true),
      meta(betPda(round.roundId, payer, symbol), false, true),
      meta(POOL, false, false), meta(VAULT, false, true), meta(payer, true, true),
      meta(musdAta(payer), false, true), meta(TOKEN_PROGRAM, false, false),
      meta(SystemProgram.programId, false, false),
    ], data));
    console.log(`place_bet $1 on face ${symbol}: ${sig} ${explorer(`tx/${sig}`)}`);
    round = await readRound(c);
  }

  if (round.statusName === "Open") {
    const closeAt = round.openedAt + ROUND_SECS;
    let now = await clockUnix(c);
    console.log(`Join window opened_at=${round.openedAt} close_at=${closeAt} clock=${now}`);
    while (now < closeAt) {
      console.log(`Waiting for the 90 s window to close (${closeAt - now}s left, on-chain Clock).`);
      const remaining = Number(closeAt - now);
      await sleep(Math.min(5_000, Math.max(500, remaining * 1000)));
      now = await clockUnix(c);
    }
    requestSig = await send(c, wallet, ix([
      meta(TABLE, false, false), meta(ROUND, false, true), meta(payer, true, true),
      meta(DEFAULT_QUEUE, false, true), meta(PROGRAM_IDENTITY, false, false),
      meta(VRF_PROGRAM_ID, false, false), meta(SLOT_HASHES, false, false),
      meta(SystemProgram.programId, false, false),
    ], DISC.requestRoll));
    console.log(`request_roll: ${requestSig} ${explorer(`tx/${requestSig}`)}`);
    const tx = await c.getTransaction(requestSig, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });
    requestSlot = tx?.slot ?? null;
    round = await readRound(c);
  } else if (round.statusName === "Rolling") {
    console.log("A roll is already pending — polling for the callback.");
  }

  if (round.statusName !== "Rolled") {
    const started = Date.now();
    while (Date.now() - started < CALLBACK_MS) {
      await sleep(3000);
      round = await readRound(c);
      console.log(`Polling round status: ${round.statusName} (${Math.round((Date.now() - started) / 1000)}s)`);
      if (round.statusName === "Rolled") break;
    }
  }

  if (round.statusName !== "Rolled") {
    console.error("Devnet VRF did not fulfill within 3 minutes. Not inventing an Explorer link. Leave this issue open and retry later.");
    process.exitCode = 2;
    return;
  }

  const callback = await findCallback(c, requestSlot);
  console.log("---");
  console.log(`Round: ${ROUND.toBase58()} ${explorer(`address/${ROUND.toBase58()}`)}`);
  console.log(`Faces: ${round.faces.join(", ")}`);
  if (requestSig) console.log(`Request tx: ${explorer(`tx/${requestSig}`)}`);
  if (!callback) {
    console.error("Round is Rolled, but the callback transaction was not found on the round account.");
    process.exitCode = 2;
    return;
  }
  const fromCallback = facesFromRandomness(callback.randomness);
  const match = fromCallback.every((f, i) => f === round.faces[i]);
  console.log(`Callback tx: ${explorer(`tx/${callback.signature}`)}`);
  console.log(`Faces from callback randomness: ${fromCallback.join(", ")}`);
  console.log(`Faces match callback: ${match ? "yes" : "NO"}`);
  if (!match) process.exitCode = 2;
}

main().catch((e) => {
  console.error(e instanceof Error ? e.message : e);
  if (e?.logs) console.error(e.logs.join("\n"));
  process.exitCode = 1;
});
