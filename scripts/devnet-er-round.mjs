#!/usr/bin/env node
/**
 * Devnet ER round on round_v2 (Steps 11.2 to 11.4; D20, D39, D40, D41). Run by the PR checker.
 *
 *   node scripts/devnet-er-round.mjs [keypair.json] [devnetRpc] [erRouter]
 *
 * Order:
 *   0. layout check: tables and config only (migrate_table on Solana if old). round_v2 is the
 *      current size; there is no round migration. The old v1 round is not touched (D41).
 *   1. migrate_config, if the config is still in the old 42-byte layout (upgrade authority).
 *   2. init_cage, if the cage vault does not exist.
 *   3. musd_faucet, if the wallet has no MUSD account.
 *   4a. undelegate_chips only if the chip stack is delegated to a DIFFERENT validator than round_v2
 *       (or the router cannot say). Not at the start of every run. If it does not come home, print
 *       the failed Solana tx and stop (round_v2 is not touched).
 *   4b/4c. resume: read the balance on the validator while delegated; buy_chips only if short;
 *       delegate_chip_stack if it is on Solana.
 *   5. init_round (table authority, if round_v2 is missing), then delegate_round.
 *   6. Read the validator's state. place_bet_chips only if the slip is empty (no token transfer).
 *   7. request_roll on DEFAULT_EPHEMERAL_QUEUE after the 90 s window, only while the round is open.
 *   8. wait for the VRF callback (skipped when round_v2 is already Rolled).
 *   9. undelegate_round and undelegate_chips (rollup), wait until Solana owns them, then
 *      settle_chip_bets and finish_round on Solana (only for a one-bet round).
 *
 * Transport: every rollup transaction goes straight to the validator (the fqdn from the router's
 * getDelegationStatus, normally https://devnet-as.magicblock.app), not to the Magic Router. The
 * router refuses a tx that mixes accounts delegated to different ER nodes, because the ephemeral VRF
 * queue is not delegated to one node (PR #109 devnet run). The blockhash comes from the validator.
 *
 * Stuck undelegate: if Solana still does not own an account after the wait, the rollup
 * undelegate is sent once more (2 tries in all), and the Solana-side commit signature from the
 * rollup log is printed (UNVERIFIED). If that also fails, the script stops with the error.
 *
 * Retries: a rollup call that fails with InstructionFallbackNotFound (error 101) is retried,
 * 30 s apart, up to 4 tries. The first rollup call after an upgrade also retries for up to 5 minutes
 * on a seeds error or "already in use" (the old program still running). Other errors are final. Every Solana transaction and the rollup signatures are printed in the summary.
 *
 * Devnet only: any URL that mentions mainnet is refused. The keypair is a gitignored wallet
 * (keys/); its secret is never printed or logged.
 *
 * UNVERIFIED (not checked from the box that wrote this script):
 *   - whether the #[ephemeral] callback brings a chip stack home on a real validator (step 4a);
 *   - whether the rollup accepts a second undelegate for an account it has already scheduled;
 *   - whether the rollup lets a delegated round grow, and whether it accepts commit_and_undelegate
 *     from this program (magic-program CPI);
 *   - how long Solana takes to own an undelegated account again;
 *   - whether the Magic Router accepts sendRawTransaction and getSignatureStatuses for rollup
 *     transactions, as notes.md describes;
 *   - whether place_bet_chips can init a new Bet account on the rollup for a bettor who is not
 *     delegated;
 *   - whether the rollup's Clock matches wall-clock time for the 90 s window.
 *
 * @solana/web3.js is resolved from app/node_modules (same pin as the UI, 1.98.4).
 */

import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const web3 = createRequire(new URL("../app/package.json", import.meta.url))("@solana/web3.js");
const {
  Connection, Keypair, PublicKey, SystemProgram, SYSVAR_CLOCK_PUBKEY, Transaction,
  TransactionInstruction, sendAndConfirmTransaction,
} = web3;

const DEFAULT_RPC = "https://api.devnet.solana.com";
const DEFAULT_ROUTER = "https://devnet-router.magicblock.app";
const DEFAULT_KEYPAIR = "keys/dev-wallet.json";

const PROGRAM_ID = new PublicKey("4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH");
const DLP_ID = new PublicKey("DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh");
const BPF_LOADER_UPGRADEABLE = new PublicKey("BPFLoaderUpgradeab1e11111111111111111111111");
const TOKEN_PROGRAM = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ATA_PROGRAM = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const EPHEMERAL_QUEUE = new PublicKey("5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc");
const VRF_PROGRAM = new PublicKey("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz");
const SLOT_HASHES = new PublicKey("SysvarS1otHashes111111111111111111111111111");
const ASIA_VALIDATOR = "MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57";
const STATUS = ["Idle", "Open", "Rolling", "Rolled"];
const BET_SYMBOL = 2;
const BET_AMOUNT = 1_000_000n;
const BUY_AMOUNT = 10_000_000n;
const WINDOW_SECS = 90n;
const CALLBACK_MS = 4 * 60 * 1000;
const OWNER_WAIT_MS = 10 * 60 * 1000;
const UNDELEGATE_TRIES = 2;
const FALLBACK_PATTERNS = ["InstructionFallbackNotFound", "Error Number: 101", "custom program error: 0x65"];
const FALLBACK_TRIES = 4;
const FALLBACK_WAIT_MS = 30_000;
// After an upgrade the rollup can keep running the old program for ~3-5 minutes (PR #109 devnet
// run: seeds error on the old round PDA). The first rollup call gets this budget, and also retries
// on a seeds error. UNVERIFIED: the time it takes the rollup to load the new program.
const WARMUP_MS = 5 * 60 * 1000;
const SEEDS_PATTERNS = ["ConstraintSeeds", "Error Number: 2006", "seeds constraint was violated"];
// Right after an upgrade the old program can still run: its init shows as "already in use" (System Allocate).
const WARMUP_PATTERNS = [...SEEDS_PATTERNS, "already in use"];
let rollupWarm = false;
// Rounds in the current layout: 392 bytes (see migrate.rs). Old: 388.

const enc = (s) => new TextEncoder().encode(s);
const P = (seeds) => PublicKey.findProgramAddressSync(seeds, PROGRAM_ID)[0];
const CONFIG = P([enc("config")]);
const POOL = P([enc("pool")]);
const POOL_VAULT = P([enc("pool_vault")]);
const SHARE_MINT = P([enc("pool_share_mint")]); // SHARE_MINT_SEED (LP shares)
const INSURANCE_VAULT = P([enc("insurance_vault")]);
const QUEUE = P([enc("queue")]); // QUEUE_SEED: the FIFO payout queue (D30, #110) // D43: orphan MUSD is swept here by deposit
const CAGE = P([enc("cage_vault")]);
const MUSD_MINT = P([enc("musd_mint")]);
// Payout math, copied from payout.rs (MULTIPLIERS, WIN_FEE_BPS = 200). Used only to size the pool top-up.
const MULTIPLIERS = [0n, 0n, 3n, 5n, 9n, 26n, 101n];
const WIN_FEE_BPS = 200n;
const POOL_TOPUP_MARGIN = 1_000_000n; // 1 MUSD spare on top of the shortfall
const MUSD_AUTH = P([enc("musd_mint_authority")]);
const TABLE = P([enc("table")]);
// round_v2 (D41): the v1 round ("round") is left alone.
const ROUND = P([enc("round_v2")]);
const PROGRAM_IDENTITY = P([enc("identity")]);
const PROGRAM_DATA = PublicKey.findProgramAddressSync([PROGRAM_ID.toBytes()], BPF_LOADER_UPGRADEABLE)[0];
const chipsPda = (owner) => P([enc("chips"), owner.toBytes()]);
const faucetPda = (owner) => P([enc("faucet"), owner.toBytes()]);
const ataOf = (owner, mint) => PublicKey.findProgramAddressSync(
  [owner.toBytes(), TOKEN_PROGRAM.toBytes(), mint.toBytes()], ATA_PROGRAM,
)[0];
const betPda = (roundId, player, symbol) => {
  const id = Buffer.alloc(8);
  id.writeBigUInt64LE(BigInt(roundId));
  return P([enc("bet"), id, player.toBytes(), Uint8Array.of(symbol)]);
};
// The SDK's delegation PDAs (ephemeral-rollups-sdk 0.18, pda.rs tags).
const delegateAccounts = (acc) => ({
  buffer: PublicKey.findProgramAddressSync([enc("buffer"), acc.toBytes()], PROGRAM_ID)[0],
  record: PublicKey.findProgramAddressSync([enc("delegation"), acc.toBytes()], DLP_ID)[0],
  metadata: PublicKey.findProgramAddressSync([enc("delegation-metadata"), acc.toBytes()], DLP_ID)[0],
});

const explorer = (sig) => `https://explorer.solana.com/tx/${sig}?cluster=devnet`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function refuseMainnet(label, url) {
  if (/mainnet/i.test(url)) throw new Error(`Refusing ${label} "${url}": this script is devnet only.`);
}

function loadWallet(keyPath) {
  if (!existsSync(resolve(keyPath))) {
    throw new Error(`No keypair at ${keyPath}. Create one with solana-keygen (gitignored, never committed).`);
  }
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(readFileSync(keyPath, "utf8"))));
}

// The IDL (anchor build writes it) gives each instruction's discriminator, the account order,
// and the fixed addresses (system program, the magic program and context), so this script
// cannot drift from the program.
function loadIdl() {
  const path = resolve("target/idl/jhandi_munda.json");
  if (!existsSync(path)) throw new Error("target/idl/jhandi_munda.json is missing: run `anchor build` first.");
  return JSON.parse(readFileSync(path, "utf8"));
}

function buildIx(idl, name, accounts, args = Buffer.alloc(0)) {
  const def = idl.instructions.find((i) => i.name === name);
  if (!def) throw new Error(`IDL has no instruction ${name}`);
  const keys = def.accounts.map((a) => {
    const fixed = a.address ? new PublicKey(a.address) : undefined;
    const key = accounts[a.name] ?? fixed;
    if (!key) throw new Error(`${name}: no address for account "${a.name}"`);
    return { pubkey: key, isSigner: !!a.signer, isWritable: !!a.writable };
  });
  const disc = Buffer.from(def.discriminator);
  return new TransactionInstruction({ programId: PROGRAM_ID, keys, data: Buffer.concat([disc, args]) });
}

const u64 = (n) => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; };
const u8 = (n) => Buffer.from([n]);

// Solana (base layer): one transaction, confirmed.
async function send(label, conn, wallet, instruction) {
  try {
    const sig = await sendAndConfirmTransaction(conn, new Transaction().add(instruction), [wallet], {
      commitment: "confirmed",
    });
    console.log(`  ${label}: ${sig}`);
    return sig;
  } catch (err) {
    console.error(`  ${label} FAILED: ${err.message ?? err}`);
    throw err;
  }
}

// Validator (ER) URL for every rollup transaction. Resolved in main from the router's
// getDelegationStatus (its fqdn). Rollup transactions go straight to the validator: the router
// refuses a tx that mixes accounts delegated to different ER nodes (the ephemeral VRF queue is not
// delegated to one node; PR #109 devnet run). UNVERIFIED: the fqdn field name in the router reply.
const ASIA_VALIDATOR_URL = "https://devnet-as.magicblock.app";

async function routerRpc(routerUrl, method, params) {
  const res = await fetch(routerUrl, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
  });
  const json = await res.json();
  if (json.error) throw new Error(`${method} RPC error: ${JSON.stringify(json.error)}`);
  return json.result;
}

// The router's view of one account: delegated?, and the ER node's fqdn. UNVERIFIED params/fields.
async function delegationStatus(routerUrl, account) {
  try {
    const s = await routerRpc(routerUrl, "getDelegationStatus", [account.toBase58()]);
    return { delegated: !!s?.isDelegated, fqdn: s?.fqdn ?? null, raw: s };
  } catch (err) {
    return { delegated: null, fqdn: null, error: String(err.message ?? err) };
  }
}

async function resolveValidatorUrl(routerUrl, account) {
  const st = await delegationStatus(routerUrl, account);
  if (st.fqdn) return { url: st.fqdn, source: "router getDelegationStatus" };
  return { url: ASIA_VALIDATOR_URL, source: `fallback (UNVERIFIED: ${st.error ?? "the router reply had no fqdn"})` };
}

const sameNode = (a, b) => !!a && !!b && a.replace(/\/+$/, "") === b.replace(/\/+$/, "");

// Rollup: one transaction to the validator, confirmed. Retries (30 s apart):
//  - always on InstructionFallbackNotFound (error 101), up to FALLBACK_TRIES;
//  - on the first rollup call after an upgrade (rollupWarm false), for up to WARMUP_MS, also on a
//    seeds error and on "already in use" (the old program's init still running; PR #109 devnet run).
async function sendEr(label, conn, wallet, instruction) {
  const warm = !rollupWarm;
  const budget = warm ? WARMUP_MS : FALLBACK_TRIES * FALLBACK_WAIT_MS;
  const started = Date.now();
  for (let attempt = 1; ; attempt++) {
    try {
      const sig = await sendErOnce(label, conn, wallet, instruction);
      rollupWarm = true;
      return sig;
    } catch (err) {
      const msg = String(err.message ?? err);
      const fallback = FALLBACK_PATTERNS.some((p) => msg.includes(p));
      const stale = warm && WARMUP_PATTERNS.some((p) => msg.includes(p));
      if (!(fallback || stale) || Date.now() - started >= budget) throw err;
      const why = fallback
        ? "InstructionFallbackNotFound (the validator is still loading the upgrade)"
        : "an error from the old program after the upgrade (seeds / already in use)";
      console.log(`  ${label}: try ${attempt} hit ${why}; waiting 30 s, then retrying`);
      await sleep(FALLBACK_WAIT_MS);
    }
  }
}

async function sendErOnce(label, conn, wallet, instruction) {
  try {
    // UNVERIFIED: that the validator answers getLatestBlockhash (the router did not give one for these accounts).
    const { blockhash } = await conn.getLatestBlockhash("confirmed");
    const tx = new Transaction({ recentBlockhash: blockhash, feePayer: wallet.publicKey }).add(instruction);
    tx.sign(wallet);
    const sig = await conn.sendRawTransaction(tx.serialize(), { skipPreflight: true });
    for (let i = 0; i < 60; i++) {
      const st = (await conn.getSignatureStatuses([sig])).value[0];
      if (st) {
        if (st.err) {
          const full = await conn.getTransaction(sig, { commitment: "confirmed", maxSupportedTransactionVersion: 0 }).catch(() => null);
          const logs = full?.meta?.logMessages?.join("\n    ") ?? "";
          throw new Error(`tx ${sig} failed: ${JSON.stringify(st.err)}${logs ? "\n    " + logs : ""}`);
        }
        if (st.confirmationStatus === "confirmed" || st.confirmationStatus === "finalized") {
          console.log(`  ${label}: ${sig}`);
          return sig;
        }
      }
      await sleep(500);
    }
    throw new Error(`tx ${sig} confirmation timed out`);
  } catch (err) {
    console.error(`  ${label} FAILED: ${err.message ?? err}`);
    throw err;
  }
}

// Waits until `account` is owned by `owner` on Solana (after an undelegate, or a delegate).
// Returns false if the wait times out.
async function waitForOwner(base, account, owner, label, waitMs = OWNER_WAIT_MS) {
  const deadline = Date.now() + waitMs;
  while (Date.now() < deadline) {
    const acc = await base.getAccountInfo(account);
    if (acc && acc.owner.equals(owner)) {
      console.log(`  ${label}: Solana owns ${account.toBase58()} again`);
      return true;
    }
    console.log(`  ${label}: waiting for Solana to own ${account.toBase58()} (owner ${acc ? acc.owner.toBase58() : "missing"})`);
    await sleep(10_000);
  }
  return false;
}

// The rollup's commit log lines for a tx, and any base58 signatures in them (the Solana-side commit).
// UNVERIFIED: the log wording and the signature format are assumed from the 1.1.1 logs, not checked on devnet.
async function commitLogSigs(er, sig) {
  const tx = await er.getTransaction(sig, { commitment: "confirmed", maxSupportedTransactionVersion: 0 }).catch(() => null);
  const lines = (tx?.meta?.logMessages ?? []).filter((l) => /commit/i.test(l));
  const sigs = lines.flatMap((l) => l.match(/[1-9A-HJ-NP-Za-km-z]{80,90}/g) ?? []);
  return { lines, sigs: [...new Set(sigs)] };
}

// Takes `account` home from the rollup: undelegate, then wait for Solana to own it. If Solana
// still does not own it after the wait, send the undelegate once more (UNDELEGATE_TRIES in all).
// If the validator refuses the second one, or Solana still does not own it, stop with the path note.
// UNVERIFIED: whether the validator accepts a second undelegate for an account it has already scheduled.
async function undelegateHome({ label, account, instruction, er, base, wallet, record }) {
  for (let attempt = 1; attempt <= UNDELEGATE_TRIES; attempt++) {
    console.log(`  ${label}: undelegate try ${attempt} of ${UNDELEGATE_TRIES} on the validator`);
    let sig;
    try {
      sig = await sendEr(`${label} undelegate try ${attempt}`, er, wallet, instruction());
    } catch (err) {
      console.error(`  ${label}: the validator refused undelegate try ${attempt}: ${err.message ?? err}`);
      break;
    }
    record(`${label} undelegate try ${attempt}`, sig, "rollup");
    const commit = await commitLogSigs(er, sig);
    commit.lines.forEach((l) => console.log(`    validator log: ${l}`));
    for (const s of commit.sigs) {
      console.log(`    Solana-side commit tx (UNVERIFIED that it is the undelegate): ${explorer(s)}`);
      record(`${label} Solana-side commit (from validator log)`, s, "solana");
    }
    if (await waitForOwner(base, account, PROGRAM_ID, label)) {
      console.log(`  PATH: ${label} came home by ${attempt === 1 ? "the first undelegate" : "a retried undelegate"}`);
      return;
    }
  }
  throw new Error(
    `${label}: Solana still does not own ${account.toBase58()} after ${UNDELEGATE_TRIES} undelegates.\n` +
    `  PATH: stopped at ${label}. The round_v2 account was not touched.`,
  );
}

// Prints the failed Solana transactions that touched `account`, with Explorer links, so a stuck
// undelegate shows its own error. UNVERIFIED: that the validator's undelegate tx is indexed under
// the account's own address (getSignaturesForAddress).
async function printSolanaFailures(base, account) {
  const recent = await base.getSignaturesForAddress(account, { limit: 5 }).catch(() => []);
  const failed = recent.filter((x) => x.err);
  if (failed.length === 0) {
    console.log(`  no failed Solana tx found for ${account.toBase58()} (UNVERIFIED lookup)`);
  }
  for (const f of failed) {
    console.log(`  failed Solana tx: ${explorer(f.signature)}`);
    console.log(`    error: ${JSON.stringify(f.err)}`);
  }
}

// Round account (current 392-byte layout, or the old 388-byte one).
function decodeRound(data) {
  let o = 8;
  const status = data[o++];
  const openedAt = data.readBigInt64LE(o); o += 8;
  const roundId = data.readBigUInt64LE(o); o += 8;
  // players: Borsh Vec (4-byte length, then the live entries only), then request_id [u8; 32], faces [u8; 6], bet_count u16.
  const n = data.readUInt32LE(o);
  o += 4 + n * 32 + 32;
  const faces = [...data.subarray(o, o + 6)];
  const betCount = data.length >= 392 ? data.readUInt16LE(o + 6) : null;
  const settledCount = data.length >= 392 ? data.readUInt16LE(o + 8) : null;
  return { status, statusName: STATUS[status] ?? `unknown(${status})`, openedAt, roundId, faces, betCount, settledCount, players: n };
}

// Chip stack: owner (32) then balance (u64).
function decodeChips(data) {
  return { owner: new PublicKey(data.subarray(8, 40)), balance: data.readBigUInt64LE(40) };
}

// Bet slip (76 bytes after the discriminator): player 32, amount u64, symbol u8, round_id u64,
// owed u64, settled bool, settled_at i64, paid_with_chips bool, bump u8.
function decodeBet(data) {
  return {
    player: new PublicKey(data.subarray(8, 40)),
    amount: data.readBigUInt64LE(40),
    symbol: data[48],
    roundId: data.readBigUInt64LE(49),
    settled: data[65] === 1,
    paidWithChips: data[74] === 1,
  };
}

// Sizes from the IDL types (no hardcoded struct sizes). `Round::players` is a Vec with
// #[max_len(10)], and the IDL does not carry that bound, so it is set here.
const MAX_PLAYERS = 10;
const PRIM = { bool: 1, u8: 1, i8: 1, u16: 2, i16: 2, u32: 4, i32: 4, u64: 8, i64: 8, u128: 16, i128: 16, pubkey: 32 };
function sizeOf(idl, t, field) {
  if (typeof t === "string") {
    if (!(t in PRIM)) throw new Error(`layout check: unknown IDL type ${t}`);
    return PRIM[t];
  }
  if ("array" in t) return sizeOf(idl, t.array[0], field) * t.array[1];
  if ("vec" in t) return 4 + (field === "players" ? MAX_PLAYERS : 0) * sizeOf(idl, t.vec, field);
  if ("defined" in t) {
    const name = t.defined.name ?? t.defined;
    const def = idl.types.find((x) => x.name === name);
    if (!def) throw new Error(`layout check: IDL has no type ${name}`);
    if (def.type.kind === "enum") {
      if (def.type.variants.some((v) => v.fields)) throw new Error(`layout check: ${name} has payloads`);
      return 1;
    }
    return def.type.fields.reduce((n, f) => n + sizeOf(idl, f.type, f.name), 0);
  }
  throw new Error("layout check: unknown IDL type shape");
}
const expectedLen = (idl, name) => 8 + sizeOf(idl, { defined: { name } });

// Layout check for the tables and config (D41). round_v2 is always the current size, so there is
// no round migration here; the old v1 round is never touched.
async function layoutCheck(idl, base, wallet, record) {
  console.log("0. layout check (account size vs the IDL struct)");
  const tableAcc = await base.getAccountInfo(TABLE);
  const table = { name: "TableConfig", len: tableAcc?.data.length ?? 0, want: expectedLen(idl, "TableConfig") };
  const roundAcc = await base.getAccountInfo(ROUND);
  const round = { name: "Round (round_v2, Solana copy)", len: roundAcc?.data.length ?? 0, want: expectedLen(idl, "Round") };
  for (const r of [table, round]) {
    const state = r.len === 0 ? "missing" : r.len === r.want ? "ok" : "not the current size";
    console.log(`   ${r.name}: ${r.len} bytes, expected ${r.want} -> ${state}`);
  }
  if (table.len !== table.want) {
    if (table.len !== 34) throw new Error(`TableConfig is ${table.len} bytes: no known migration.`);
    console.log("0a. migrate_table (Solana, upgrade authority)");
    record("migrate_table", await send("migrate_table", base, wallet, buildIx(idl, "migrate_table", {
      table: TABLE, upgrade_authority: wallet.publicKey, program_data: PROGRAM_DATA,
      system_program: SystemProgram.programId,
    })), "solana");
  }
  if (roundAcc && round.len !== round.want) {
    throw new Error(`round_v2 is ${round.len} bytes, not the current size. D41: there is no migration path for round_v2; stopping.`);
  }
}

// What settle_chip_bets owes one chip slip, in base units: the stake moves cage -> pool first, then the
// win (stake + 98% of the profit, payout.rs settle_win) is paid from the pool. Settle needs the pool
// to hold at least `owed` after the stake arrives.
function owedFor(stake, faces, symbol) {
  const k = BigInt(faces.filter((f) => f === symbol).length);
  const gross = stake * MULTIPLIERS[Number(k)];
  if (gross <= stake) return gross;
  const profit = gross - stake;
  return stake + (profit * (10_000n - WIN_FEE_BPS)) / 10_000n;
}

// Tops up the pool vault with the LP `deposit` (the dev wallet is the depositor) when a settle would
// fail. Devnet only; no program change. Returns the deposit amount (0n if none was needed).
async function topUpPoolFor({ idl, base, wallet, bet, faces, record, rpc }) {
  const vault = await base.getAccountInfo(POOL_VAULT);
  const poolBal = vault.data.readBigUInt64LE(64);
  const slip = decodeBet(((await base.getAccountInfo(bet)) ?? { data: Buffer.alloc(0) }).data);
  if (!slip.amount) throw new Error("the bet slip has no stake on Solana; nothing to settle");
  const owed = owedFor(slip.amount, faces, slip.symbol);
  const shortfall = owed - (poolBal + slip.amount);
  console.log(`   pool vault ${poolBal} base units; stake ${slip.amount}; owed ${owed} (face ${slip.symbol}, ${faces.filter((f) => f === slip.symbol).length} matches)`);
  if (shortfall <= 0n) {
    console.log("   the pool can pay this win; no deposit");
    return 0n;
  }
  const deposit = shortfall + POOL_TOPUP_MARGIN;
  console.log(`   the pool is short by ${shortfall}; depositing ${deposit} base units (LP deposit, dev wallet)`);
  const musdAta = ataOf(wallet.publicKey, MUSD_MINT);
  const have = (await base.getAccountInfo(musdAta))?.data.readBigUInt64LE(64) ?? 0n;
  if (have < deposit) {
    console.log(`   the wallet holds ${have} MUSD base units; musd_faucet first (100 MUSD per call, 1-hour cooldown, D33)`);
    try {
      record("musd_faucet", await send("musd_faucet", base, wallet, buildIx(idl, "musd_faucet", {
        config: CONFIG, musd_mint: MUSD_MINT, musd_mint_authority: MUSD_AUTH, faucet_record: faucetPda(wallet.publicKey),
        wallet: wallet.publicKey, destination: musdAta, associated_token_program: ATA_PROGRAM,
        token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
      })), "solana");
    } catch (err) {
      throw new Error(`the wallet is short of MUSD for the pool top-up (${have} < ${deposit}) and the faucet refused: ${err.message ?? err}`);
    }
  }
  const nowHave = (await base.getAccountInfo(musdAta))?.data.readBigUInt64LE(64) ?? 0n;
  if (nowHave < deposit) throw new Error(`the wallet holds ${nowHave} MUSD base units, below the ${deposit} deposit`);
  // D43: with no shares, deposit sweeps the vault's orphan MUSD to insurance first, so the vault must exist.
  if (!(await base.getAccountInfo(INSURANCE_VAULT))) {
    throw new Error("the insurance vault is missing on devnet: run init_insurance first (deposit sweeps orphan MUSD there, D43). UNVERIFIED whether it exists.");
  }
  record("deposit", await send("deposit (LP, pool top-up)", base, wallet, buildIx(idl, "deposit", {
    pool: POOL, share_mint: SHARE_MINT, vault: POOL_VAULT, depositor: wallet.publicKey, depositor_ata: musdAta,
    depositor_shares: ataOf(wallet.publicKey, SHARE_MINT), insurance_vault: INSURANCE_VAULT, queue: QUEUE, token_program: TOKEN_PROGRAM,
    system_program: SystemProgram.programId, associated_token_program: ATA_PROGRAM,
  }, u64(deposit))), "solana");
  return deposit;
}

// Brings round_v2, the chip stack and the bet slip home (whichever are still on the validator),
// then settles the one chip bet and finishes the round on Solana. Idempotent: each step is skipped
// when it is already done, so a re-run after a partial success continues.
async function settleFromSolana({ idl, base, er, wallet, record, roundId, bet, chips }) {
  const chipsUndelegate = () => ({
    label: "undelegate_chips", account: chips, er, base, wallet, record,
    instruction: () => buildIx(idl, "undelegate_chips", { payer: wallet.publicKey, config: CONFIG, chips },
      wallet.publicKey.toBuffer()),
  });
  if ((await base.getAccountInfo(ROUND)).owner.equals(DLP_ID)) {
    await undelegateHome({
      label: "undelegate_round", account: ROUND, er, base, wallet, record,
      instruction: () => buildIx(idl, "undelegate_round", { payer: wallet.publicKey, config: CONFIG, round: ROUND }),
    });
  } else {
    console.log("9. round_v2 is already on Solana (undelegated); skipping undelegate_round");
  }
  if ((await base.getAccountInfo(chips))?.owner.equals(DLP_ID)) {
    await undelegateHome(chipsUndelegate());
  } else {
    console.log("9. the chip stack is already on Solana; skipping undelegate_chips");
  }
  if ((await base.getAccountInfo(bet))?.owner.equals(DLP_ID)) {
    await undelegateHome({
      label: "undelegate_bet", account: bet, er, base, wallet, record,
      instruction: () => buildIx(idl, "undelegate_bet", { payer: wallet.publicKey, config: CONFIG, bet },
        Buffer.concat([u64(roundId), wallet.publicKey.toBuffer(), u8(BET_SYMBOL)])),
    });
  } else {
    console.log("9. the bet slip is already on Solana; skipping undelegate_bet");
  }

  const home = decodeRound((await base.getAccountInfo(ROUND)).data);
  if (home.status === 0) {
    console.log("9a. round_v2 is already finished (Idle on Solana): nothing left to settle.");
  } else if (home.betCount === 1) {
    const slipHome = await base.getAccountInfo(bet);
    const alreadySettled = !!slipHome && slipHome.owner.equals(PROGRAM_ID) && decodeBet(slipHome.data).settled;
    if (alreadySettled) {
      console.log("9a. the bet slip is already settled on Solana: skipping settle_chip_bets (resume)");
    } else {
      console.log("9a. top up the pool if it is short, then settle_chip_bets on Solana");
      await topUpPoolFor({ idl, base, wallet, bet, faces: home.faces, record });
      const settle = buildIx(idl, "settle_chip_bets", {
        table: TABLE, round: ROUND, pool: POOL, pool_vault: POOL_VAULT, cage_vault: CAGE, token_program: TOKEN_PROGRAM,
        queue: QUEUE,
      });
      settle.keys.push({ pubkey: bet, isSigner: false, isWritable: true }, { pubkey: chips, isSigner: false, isWritable: true });
      try {
        record("settle_chip_bets", await send("settle_chip_bets", base, wallet, settle), "solana");
      } catch (err) {
        if (String(err.message ?? err).includes("InsufficientPoolFunds")) {
          throw new Error("settle_chip_bets: the pool vault cannot pay this chip win now (InsufficientPoolFunds, 6025). The top-up above did not cover it; rerun, or top up the pool by hand. (The program's message still says 'queue is Step 8'; that text is fixed in a later program change.)");
        }
        throw err;
      }
    }
    const homeAfter = decodeRound((await base.getAccountInfo(ROUND)).data);
    if (homeAfter.settledCount !== homeAfter.betCount) {
      throw new Error(`the round has ${homeAfter.betCount} bets and ${homeAfter.settledCount} settled; finish_round needs all settled.`);
    }
    record("finish_round", await send("finish_round", base, wallet, buildIx(idl, "finish_round", { round: ROUND })), "solana");
  } else {
    console.log(`9a. the round holds ${home.betCount} bets; this script settles only a one-bet round (UNVERIFIED: settling every bet is not built here).`);
  }
  const chipsHome = decodeChips((await base.getAccountInfo(chips)).data);
  console.log(`   chips on Solana: ${chipsHome.balance} base units`);
}

function printSummary(sigs, validator) {
  console.log("\n=== Summary ===");
  console.log(`Validator for rollup transactions: ${validator.url}`);
  for (const r of sigs) {
    const link = r.where === "solana"
      ? explorer(r.sig)
      : `(validator ${validator.url}; Explorer link UNVERIFIED) signature ${r.sig}`;
    console.log(`${r.label}: ${r.sig}\n   ${link}`);
  }
}

async function main() {
  const [keyArg, rpcArg, routerArg] = process.argv.slice(2);
  const rpc = rpcArg ?? DEFAULT_RPC;
  const routerUrl = routerArg ?? DEFAULT_ROUTER;
  refuseMainnet("RPC", rpc);
  refuseMainnet("router", routerUrl);

  const idl = loadIdl();
  const wallet = loadWallet(keyArg ?? DEFAULT_KEYPAIR);
  const base = new Connection(rpc, "confirmed");
  // Every rollup transaction goes straight to the validator (not the router); the router is only
  // asked where round_v2 lives.
  const validator = await resolveValidatorUrl(routerUrl, ROUND);
  const er = new Connection(validator.url, "confirmed");
  console.log(`Devnet RPC: ${rpc}`);
  console.log(`Router (getDelegationStatus only): ${routerUrl}`);
  console.log(`Validator for rollup transactions: ${validator.url} (source: ${validator.source})`);
  console.log(`Wallet: ${wallet.publicKey.toBase58()}`);
  console.log(`Round PDA: ${ROUND.toBase58()} (round_v2, D41). The old v1 round is not touched.`);
  if ((await base.getBalance(wallet.publicKey)) < 0.05 * 1e9) throw new Error("The wallet needs more devnet SOL (at least 0.05).");

  const sigs = [];
  const record = (label, sig, where) => sigs.push({ label, sig, where });

  // 0. Layout check (tables and config) and migrate_table if the table is old.
  await layoutCheck(idl, base, wallet, record);

  // 1. The config (42-byte layout -> current).
  const cfg = await base.getAccountInfo(CONFIG);
  if (!cfg) throw new Error("The config is missing on devnet: run initialize first.");
  if (cfg.data.length === 42) {
    console.log("1. config is in the old layout; migrate_config (upgrade authority only)");
    record("migrate_config", await send("migrate_config", base, wallet, buildIx(idl, "migrate_config", {
      config: CONFIG, upgrade_authority: wallet.publicKey, program_data: PROGRAM_DATA,
      system_program: SystemProgram.programId,
    })), "solana");
  } else {
    console.log(`1. config already in the current layout (${cfg.data.length} bytes)`);
  }
  const admin = new PublicKey((await base.getAccountInfo(CONFIG)).data.subarray(8, 40));
  if (!admin.equals(wallet.publicKey)) {
    throw new Error(`This wallet is not the table authority (admin is ${admin.toBase58()}). Only it can delegate (D40).`);
  }

  // 2. The cage vault (chip money, D39).
  if (!(await base.getAccountInfo(CAGE))) {
    console.log("2. cage vault missing; init_cage");
    record("init_cage", await send("init_cage", base, wallet, buildIx(idl, "init_cage", {
      config: CONFIG, pool: POOL, cage_vault: CAGE, musd_mint: MUSD_MINT, payer: wallet.publicKey,
      token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
    })), "solana");
  } else {
    console.log("2. cage vault exists");
  }

  // 2b. The insurance vault (#111): the deposit sweeps orphan MUSD here (D43), so it must exist.
  if (!(await base.getAccountInfo(INSURANCE_VAULT))) {
    console.log("2b. insurance vault missing; init_insurance");
    record("init_insurance", await send("init_insurance", base, wallet, buildIx(idl, "init_insurance", {
      config: CONFIG, insurance_vault: INSURANCE_VAULT, musd_mint: MUSD_MINT, payer: wallet.publicKey,
      token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
    })), "solana");
  } else {
    console.log("2b. insurance vault exists");
  }

  // 3. MUSD for the wallet (the faucet; one claim per hour per wallet).
  const musdAta = ataOf(wallet.publicKey, MUSD_MINT);
  if (!(await base.getAccountInfo(musdAta))) {
    console.log("3. no MUSD account yet; musd_faucet");
    record("musd_faucet", await send("musd_faucet", base, wallet, buildIx(idl, "musd_faucet", {
      config: CONFIG, musd_mint: MUSD_MINT, musd_mint_authority: MUSD_AUTH, faucet_record: faucetPda(wallet.publicKey),
      wallet: wallet.publicKey, destination: musdAta, associated_token_program: ATA_PROGRAM,
      token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
    })), "solana");
  } else {
    console.log("3. MUSD account exists");
  }

  // Resume: round_v2 may already be past the rollup. A finished round, or a Rolled round that is back
  // on Solana (undelegated by an earlier run that stopped before settle), is finished here.
  const roundTopNow = await base.getAccountInfo(ROUND);
  if (roundTopNow.owner.equals(PROGRAM_ID)) {
    const r0 = decodeRound(roundTopNow.data);
    if (r0.status === 0 && r0.roundId > 0n) {
      const prev = await base.getAccountInfo(betPda(r0.roundId - 1n, wallet.publicKey, BET_SYMBOL));
      if (prev && prev.owner.equals(PROGRAM_ID) && decodeBet(prev.data).settled) {
        console.log(`RESUME: round_v2 is already finished (round ${r0.roundId - 1n} is settled and the round is Idle). Nothing to do.`);
        printSummary(sigs, validator);
        return;
      }
    }
    if (r0.status === 3) {
      console.log(`RESUME: round_v2 is Rolled (faces [${r0.faces.join(", ")}]) and already back on Solana; going straight to settle.`);
      await settleFromSolana({
        idl, base, er, wallet, record, roundId: r0.roundId,
        bet: betPda(r0.roundId, wallet.publicKey, BET_SYMBOL), chips: chipsPda(wallet.publicKey),
      });
      printSummary(sigs, validator);
      return;
    }
  }

  // 4a. The chip stack. It is NOT undelegated at the start of every run. Only when it is delegated
  // to a different validator than round_v2 (or the router cannot say) is it brought home first.
  const chips = chipsPda(wallet.publicKey);
  const chipsUndelegate = () => ({
    label: "undelegate_chips", account: chips, er, base, wallet, record,
    instruction: () => buildIx(idl, "undelegate_chips", { payer: wallet.publicKey, config: CONFIG, chips },
      wallet.publicKey.toBuffer()),
  });
  const chipsOnSolana = await base.getAccountInfo(chips);
  if (chipsOnSolana && chipsOnSolana.owner.equals(DLP_ID)) {
    const node = await delegationStatus(routerUrl, chips);
    if (node.delegated === true && sameNode(node.fqdn, validator.url)) {
      console.log("4a. chip stack is delegated to round_v2's validator: no undelegate at the start of this run");
    } else {
      const why = node.delegated === null ? `router status unavailable (${node.error})`
        : node.fqdn ? `delegated to ${node.fqdn}` : "delegated, node unknown (UNVERIFIED fqdn field)";
      console.log(`4a. chip stack is delegated but not to round_v2's validator (${why}): undelegate it first`);
      try {
        await undelegateHome(chipsUndelegate());
      } catch (err) {
        console.error("  STOP: the chip stack did not come home. The round was not touched.");
        await printSolanaFailures(base, chips);
        throw err;
      }
      console.log("  PROOF: the chip stack came home through the callback (Solana owns it again).");
    }
  } else {
    console.log("4a. chip stack is not delegated; nothing to undelegate");
  }

  // 4b. Chips: while delegated the balance is on the validator's copy; otherwise on Solana.
  const solanaChipBalance = async () => {
    const acc = await base.getAccountInfo(chips);
    return acc && acc.owner.equals(PROGRAM_ID) ? decodeChips(acc.data).balance : 0n;
  };
  let chipsDelegated = (await base.getAccountInfo(chips))?.owner.equals(DLP_ID) ?? false;
  let balance = 0n;
  if (chipsDelegated) {
    const copy = await er.getAccountInfo(chips);
    balance = copy ? decodeChips(copy.data).balance : 0n;
  } else {
    balance = await solanaChipBalance();
  }
  if (chipsDelegated && balance < BET_AMOUNT) {
    console.log(`4b. chip stack is delegated but holds ${balance} base units: undelegate it to buy more`);
    await undelegateHome(chipsUndelegate());
    chipsDelegated = false;
    balance = await solanaChipBalance();
  }
  if (chipsDelegated) {
    console.log(`4b. chip stack is delegated and holds ${balance} base units; skipping buy and delegate`);
  } else {
    if (balance >= BET_AMOUNT) {
      console.log(`4b. chip stack holds ${balance} base units; skipping buy`);
    } else {
      console.log("4b. buy_chips (10 MUSD) into the cage vault");
      record("buy_chips", await send("buy_chips", base, wallet, buildIx(idl, "buy_chips", {
        pool: POOL, cage_vault: CAGE, chips, owner: wallet.publicKey, owner_musd_ata: musdAta,
        token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
      }, u64(BUY_AMOUNT))), "solana");
    }
    console.log("4c. delegate_chip_stack (owner signs)");
    const chipAcc = delegateAccounts(chips);
    record("delegate_chip_stack", await send("delegate_chip_stack", base, wallet, buildIx(idl, "delegate_chip_stack", {
      owner: wallet.publicKey, chips, buffer_chips: chipAcc.buffer, delegation_record_chips: chipAcc.record,
      delegation_metadata_chips: chipAcc.metadata, owner_program: PROGRAM_ID, delegation_program: DLP_ID,
      system_program: SystemProgram.programId,
    })), "solana");
  }

  // 4d. The bet slip (D42): opened and delegated on Solana, unless it already is.
  const roundOnSolana = await base.getAccountInfo(ROUND);
  // While delegated, Solana's copy of the round is zeroed (the SDK clears the PDA); the live state is on the validator.
  const roundIdNow = decodeRound((roundOnSolana.owner.equals(DLP_ID) ? await er.getAccountInfo(ROUND) : roundOnSolana).data).roundId;
  const bet = betPda(roundIdNow, wallet.publicKey, BET_SYMBOL);
  let slipBase = await base.getAccountInfo(bet);
  if (!slipBase) {
    console.log(`4d. open_chip_bet for round ${roundIdNow}, face ${BET_SYMBOL} (Solana; the wallet pays the slip's rent)`);
    record("open_chip_bet", await send("open_chip_bet", base, wallet, buildIx(idl, "open_chip_bet", {
      round: ROUND, bet, bettor: wallet.publicKey, system_program: SystemProgram.programId,
    }, Buffer.concat([u64(roundIdNow), u8(BET_SYMBOL)]))), "solana");
    slipBase = await base.getAccountInfo(bet);
  } else {
    console.log("4d. the bet slip exists");
  }
  if (slipBase.owner.equals(PROGRAM_ID)) {
    console.log("4d. delegate_bet (the bettor signs)");
    const slipAcc = delegateAccounts(bet);
    record("delegate_bet", await send("delegate_bet", base, wallet, buildIx(idl, "delegate_bet", {
      owner: wallet.publicKey, bet, buffer_bet: slipAcc.buffer, delegation_record_bet: slipAcc.record,
      delegation_metadata_bet: slipAcc.metadata, owner_program: PROGRAM_ID, delegation_program: DLP_ID,
      system_program: SystemProgram.programId,
    }, Buffer.concat([u64(roundIdNow), u8(BET_SYMBOL)]))), "solana");
  } else {
    console.log("4d. the bet slip is already delegated; skipping delegate_bet");
  }

  // 5. round_v2: init if missing (table authority), then delegate.
  let roundBase = await base.getAccountInfo(ROUND);
  if (!roundBase) {
    console.log("5a. round_v2 missing; init_round (table authority)");
    record("init_round", await send("init_round", base, wallet, buildIx(idl, "init_round", {
      config: CONFIG, round: ROUND, payer: wallet.publicKey, system_program: SystemProgram.programId,
    })), "solana");
    roundBase = await base.getAccountInfo(ROUND);
  } else {
    console.log("5a. round_v2 exists");
  }
  if (roundBase.owner.equals(DLP_ID)) {
    console.log("5b. round_v2 already delegated; skipping delegate_round");
  } else {
    console.log("5b. delegate_round (round_v2) to the Asia validator");
    const roundAcc = delegateAccounts(ROUND);
    record("delegate_round", await send("delegate_round", base, wallet, buildIx(idl, "delegate_round", {
      payer: wallet.publicKey, config: CONFIG, round: ROUND, buffer_round: roundAcc.buffer,
      delegation_record_round: roundAcc.record, delegation_metadata_round: roundAcc.metadata,
      owner_program: PROGRAM_ID, delegation_program: DLP_ID, system_program: SystemProgram.programId,
    })), "solana");
  }

  // 6. Read the rollup state first (resume from where devnet is). Fill the slip only if it is empty.
  const readEr = async (pk, what) => {
    const acc = await er.getAccountInfo(pk);
    if (!acc) throw new Error(`${what} is not on the validator yet (${validator.url}).`);
    return acc;
  };
  let roundOnEr = decodeRound((await readEr(ROUND, "round_v2")).data);
  const slipOnEr = await er.getAccountInfo(bet);
  const slipFilled = !!slipOnEr && decodeBet(slipOnEr.data).amount > 0n;
  if (slipFilled) {
    console.log(`6. the bet slip is already filled on the validator (${decodeBet(slipOnEr.data).amount} base units); skipping place_bet_chips`);
  } else {
    if (roundOnEr.status !== 0 && roundOnEr.status !== 1) {
      throw new Error(`round_v2 is ${roundOnEr.statusName} and the bet slip is empty: nothing to place.`);
    }
    console.log("6. place_bet_chips on the validator (fills the slip opened on Solana, D42)");
    record("place_bet_chips", await sendEr("place_bet_chips (validator)", er, wallet, buildIx(idl, "place_bet_chips", {
      table: TABLE, round: ROUND, bet, chips, bettor: wallet.publicKey, system_program: SystemProgram.programId,
    }, Buffer.concat([u8(BET_SYMBOL), u64(BET_AMOUNT)]))), "rollup");
    roundOnEr = decodeRound((await readEr(ROUND, "round_v2")).data);
  }

  // 7. request_roll only while the round is still open. Rolling: wait. Rolled: nothing to do.
  if (roundOnEr.status === 0 || roundOnEr.status === 1) {
    console.log("7. request_roll on DEFAULT_EPHEMERAL_QUEUE (validator), after the 90 s window");
    const opened = roundOnEr.openedAt;
    for (;;) {
      const now = (await readEr(SYSVAR_CLOCK_PUBKEY, "the clock")).data.readBigInt64LE(32);
      if (now >= opened + WINDOW_SECS) break;
      console.log(`   waiting: validator clock ${now}, window closes at ${opened + WINDOW_SECS}`);
      await sleep(5000);
    }
    record("request_roll", await sendEr("request_roll (validator)", er, wallet, buildIx(idl, "request_roll", {
      table: TABLE, round: ROUND, payer: wallet.publicKey, oracle_queue: EPHEMERAL_QUEUE,
      program_identity: PROGRAM_IDENTITY, vrf_program: VRF_PROGRAM, slot_hashes: SLOT_HASHES,
      system_program: SystemProgram.programId,
    })), "rollup");
  } else {
    console.log(`7. round_v2 is ${roundOnEr.statusName}; skipping request_roll`);
  }

  // 8. The VRF callback: the round turns Rolled with six faces. Skipped when already Rolled.
  let final = roundOnEr.status === 3 ? roundOnEr : null;
  if (!final) {
    console.log("8. waiting for the VRF callback on the validator");
    const deadline = Date.now() + CALLBACK_MS;
    while (Date.now() < deadline) {
      const r = decodeRound((await readEr(ROUND, "round_v2")).data);
      if (r.status === 3) { final = r; break; }
      await sleep(3000);
    }
    if (!final) {
      console.log("The round did not reach Rolled before the timeout (UNVERIFIED: the callback on the validator).");
      process.exitCode = 1;
    }
  }
  if (final) console.log(`Round ${final.roundId} is ${final.statusName}; faces: [${final.faces.join(", ")}]`);

  // 9. Undelegate round, chips and bet (D40, bettor or table authority), then settle and finish on Solana.
  if (final) {
    await settleFromSolana({ idl, base, er, wallet, record, roundId: roundIdNow, bet, chips });
  }

  printSummary(sigs, validator);
}

main().catch((err) => {
  console.error(`\nDevnet ER round failed: ${err.message ?? err}`);
  process.exitCode = 1;
});
