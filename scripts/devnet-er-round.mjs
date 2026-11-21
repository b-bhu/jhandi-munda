#!/usr/bin/env node
/**
 * Devnet ER round, end to end (Steps 11.2 to 11.4; D20, D39, D40). Run by the PR checker.
 *
 *   node scripts/devnet-er-round.mjs [keypair.json] [devnetRpc] [erRouter]
 *
 * Steps. Each prints its signature and an Explorer link (Solana) or its rollup signature.
 *   0. layout check: prints each account's size against the IDL struct. Table: migrate_table
 *      (Solana). Round: if delegated and old-size, undelegate it on the rollup, wait until
 *      Solana owns it, migrate_round on Solana (the wallet pays rent), finish_round if
 *      Rolled, delegate again. A round not delegated and old-size: migrate_round on Solana.
 *   1. migrate_config, if the config is still in the old 42-byte layout (upgrade authority).
 *   2. init_cage, if the cage vault does not exist.
 *   3. musd_faucet, if the wallet has no MUSD account.
 *   4. chips: buy (on Solana) if needed, then delegate_chip_stack (Solana). If the chip stack
 *      is delegated but short of chips, undelegate it on the rollup first.
 *   5. delegate_round (Solana), if not delegated.
 *   6. place_bet_chips on the rollup (no token transfer).
 *   7. request_roll on the rollup, on DEFAULT_EPHEMERAL_QUEUE, after the 90 s window.
 *   8. wait for the VRF callback: the round turns Rolled with six faces.
 *   9. commit and undelegate the round and the chip stack (rollup), wait until Solana owns
 *      them, settle_chip_bets and finish_round on Solana (only if the round holds one bet).
 *
 * Stuck undelegate: if Solana still does not own a round or chip stack after the wait, the
 * undelegate is sent once more on the rollup (2 tries in all) and the Solana-side commit
 * signature from the rollup log is printed (UNVERIFIED). If that also fails, the script stops and
 * prints "PATH: fresh round NOT taken": the round PDA has no round id in its seeds, so a fresh
 * round needs a program change, which this script does not make.
 *
 * Retries: a rollup call that fails with InstructionFallbackNotFound (error 101) is retried,
 * 30 s apart, up to 4 tries (the rollup takes about 2 minutes to load an upgrade). Every
 * other error is final.
 *
 * Devnet only: any URL that mentions mainnet is refused. The keypair is a gitignored wallet
 * (keys/); its secret is never printed or logged.
 *
 * UNVERIFIED (not checked from the box that wrote this script):
 *   - whether the rollup lets a delegated round grow (paying rent), and whether it accepts
 *     commit_and_undelegate from this program (the rollup's magic-program CPI);
 *   - how long Solana takes to own an undelegated account again;
 *   - whether the Magic Router accepts sendRawTransaction and getSignatureStatuses for
 *     rollup transactions, as notes.md describes;
 *   - whether place_bet_chips can init a new Bet account on the rollup for a bettor who is
 *     not delegated;
 *   - whether the rollup's Clock matches wall-clock time for the 90 s window.
 * If a step fails, the script prints the error and exits 1. It does not retry other errors.
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
// Rounds in the current layout: 392 bytes (see migrate.rs). Old: 388.
const ROUND_OLD_LEN = 388;

const enc = (s) => new TextEncoder().encode(s);
const P = (seeds) => PublicKey.findProgramAddressSync(seeds, PROGRAM_ID)[0];
const CONFIG = P([enc("config")]);
const POOL = P([enc("pool")]);
const POOL_VAULT = P([enc("pool_vault")]);
const CAGE = P([enc("cage_vault")]);
const MUSD_MINT = P([enc("musd_mint")]);
const MUSD_AUTH = P([enc("musd_mint_authority")]);
const TABLE = P([enc("table")]);
const ROUND = P([enc("round")]);
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

// Rollup: one transaction through the router, confirmed. Retries on the post-upgrade fallback error.
async function sendEr(label, routerUrl, conn, wallet, instruction, routedAccounts) {
  for (let attempt = 1; ; attempt++) {
    try {
      return await sendErOnce(label, routerUrl, conn, wallet, instruction, routedAccounts);
    } catch (err) {
      const msg = String(err.message ?? err);
      const retryable = FALLBACK_PATTERNS.some((p) => msg.includes(p));
      if (!retryable || attempt >= FALLBACK_TRIES) throw err;
      console.log(`  ${label}: try ${attempt} hit InstructionFallbackNotFound (the rollup is still loading the upgrade); waiting 30 s, then retrying`);
      await sleep(FALLBACK_WAIT_MS);
    }
  }
}

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

async function sendErOnce(label, routerUrl, conn, wallet, instruction, routedAccounts) {
  try {
    // UNVERIFIED: that the router answers getBlockhashForAccounts for these accounts (notes.md).
    const { blockhash } = await routerRpc(routerUrl, "getBlockhashForAccounts", [
      routedAccounts.map((p) => p.toBase58()),
    ]);
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
// If the rollup refuses the second one, or Solana still does not own it, stop with the path note.
// UNVERIFIED: whether the rollup accepts a second undelegate for a round it has already scheduled.
async function undelegateHome({ label, account, instruction, routerUrl, er, base, wallet, record }) {
  for (let attempt = 1; attempt <= UNDELEGATE_TRIES; attempt++) {
    console.log(`  ${label}: undelegate try ${attempt} of ${UNDELEGATE_TRIES} on the rollup`);
    let sig;
    try {
      sig = await sendEr(`${label} undelegate try ${attempt}`, routerUrl, er, wallet, instruction(), [account]);
    } catch (err) {
      console.error(`  ${label}: the rollup refused undelegate try ${attempt}: ${err.message ?? err}`);
      break;
    }
    record(`${label} undelegate try ${attempt}`, sig, "rollup");
    const commit = await commitLogSigs(er, sig);
    commit.lines.forEach((l) => console.log(`    rollup log: ${l}`));
    for (const s of commit.sigs) {
      console.log(`    Solana-side commit tx (UNVERIFIED that it is the undelegate): ${explorer(s)}`);
      record(`${label} Solana-side commit (from rollup log)`, s, "solana");
    }
    if (await waitForOwner(base, account, PROGRAM_ID, label)) {
      console.log(`  PATH: ${label} came home by retrying the undelegate (attempt ${attempt})`);
      return;
    }
  }
  throw new Error(
    `${label}: Solana still does not own ${account.toBase58()} after ${UNDELEGATE_TRIES} undelegates.\n` +
    `  PATH: fresh round NOT taken. The round PDA is seeds ["round"] and the table PDA is seeds ["table"]; ` +
    `neither carries a round id, so a fresh round at a new address needs a program change (a new seed). ` +
    `Not made in this script. Stopping here.`,
  );
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
  return { status, statusName: STATUS[status] ?? `unknown(${status})`, openedAt, roundId, faces, betCount, players: n };
}

// Chip stack: owner (32) then balance (u64).
function decodeChips(data) {
  return { owner: new PublicKey(data.subarray(8, 40)), balance: data.readBigUInt64LE(40) };
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

async function layoutCheck(idl, base, er, wallet, routerUrl, record) {
  console.log("0. layout check (account size vs the IDL struct)");
  const tableAcc = await base.getAccountInfo(TABLE);
  const table = { name: "TableConfig (Solana)", len: tableAcc?.data.length ?? 0, want: expectedLen(idl, "TableConfig") };
  const roundBase = await base.getAccountInfo(ROUND);
  const roundDelegated = !!roundBase && roundBase.owner.equals(DLP_ID);
  const roundLen = roundDelegated ? (await er.getAccountInfo(ROUND))?.data.length ?? 0 : roundBase?.data.length ?? 0;
  const round = { name: `Round (${roundDelegated ? "rollup copy" : "Solana"})`, len: roundLen, want: expectedLen(idl, "Round") };
  const cfgAcc = await base.getAccountInfo(CONFIG);
  const config = { name: "ProgramConfig (Solana)", len: cfgAcc?.data.length ?? 0, want: expectedLen(idl, "ProgramConfig") };
  for (const r of [table, round, config]) {
    console.log(`   ${r.name}: ${r.len} bytes, expected ${r.want} -> ${r.len === r.want ? "ok" : "needs migration"}`);
  }

  if (table.len !== table.want) {
    if (table.len !== 34) throw new Error(`TableConfig is ${table.len} bytes: no known migration.`);
    console.log("0a. migrate_table (Solana, upgrade authority)");
    record("migrate_table", await send("migrate_table", base, wallet, buildIx(idl, "migrate_table", {
      table: TABLE, upgrade_authority: wallet.publicKey, program_data: PROGRAM_DATA,
      system_program: SystemProgram.programId,
    })), "solana");
  }

  if (round.len === round.want) return;
  if (round.len !== ROUND_OLD_LEN) throw new Error(`Round is ${round.len} bytes: no known migration.`);
  const migrateAccounts = {
    round: ROUND, upgrade_authority: wallet.publicKey, program_data: PROGRAM_DATA,
    system_program: SystemProgram.programId,
  };

  if (!roundDelegated) {
    console.log("0b. migrate_round on Solana (upgrade authority)");
    record("migrate_round", await send("migrate_round", base, wallet, buildIx(idl, "migrate_round", migrateAccounts)), "solana");
    return;
  }

  // The round is delegated and old-size: take it home, migrate it on Solana, delegate it again.
  // UNVERIFIED: whether the rollup allows the resize at all (growing a delegated account).
  console.log("0b. the delegated round is old-size: undelegate it on the rollup (table authority)");
  await undelegateHome({
    label: "undelegate_round", account: ROUND, routerUrl, er, base, wallet, record,
    instruction: () => buildIx(idl, "undelegate_round", { payer: wallet.publicKey, config: CONFIG, round: ROUND }),
  });

  console.log("0c. migrate_round on Solana (the wallet pays the rent here)");
  record("migrate_round", await send("migrate_round", base, wallet, buildIx(idl, "migrate_round", migrateAccounts)), "solana");
  const migrated = decodeRound((await base.getAccountInfo(ROUND)).data);
  if (migrated.status === 3) {
    console.log("0d. the round is Rolled: finish_round on Solana (no bet counts were kept, see migrate.rs)");
    record("finish_round", await send("finish_round", base, wallet, buildIx(idl, "finish_round", { round: ROUND })), "solana");
  }

  console.log("0e. delegate_round again (table authority)");
  const roundAcc = delegateAccounts(ROUND);
  record("delegate_round", await send("delegate_round", base, wallet, buildIx(idl, "delegate_round", {
    payer: wallet.publicKey, config: CONFIG, round: ROUND, buffer_round: roundAcc.buffer,
    delegation_record_round: roundAcc.record, delegation_metadata_round: roundAcc.metadata,
    owner_program: PROGRAM_ID, delegation_program: DLP_ID, system_program: SystemProgram.programId,
  })), "solana");
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
  const er = new Connection(routerUrl, "confirmed");
  console.log(`Devnet RPC: ${rpc}`);
  console.log(`Rollup router: ${routerUrl} (region: Asia, validator ${ASIA_VALIDATOR}, validator 1.1.1)`);
  console.log(`Wallet: ${wallet.publicKey.toBase58()}`);
  if ((await base.getBalance(wallet.publicKey)) < 0.05 * 1e9) throw new Error("The wallet needs more devnet SOL (at least 0.05).");

  const sigs = [];
  const record = (label, sig, where) => sigs.push({ label, sig, where });

  // 0. Layout check and the migrations it needs.
  await layoutCheck(idl, base, er, wallet, routerUrl, record);

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

  // 4. Chips: enough for the bet, on the rollup, and delegated.
  const chips = chipsPda(wallet.publicKey);
  const chipsBase = await base.getAccountInfo(chips);
  const chipsDelegated = !!chipsBase && chipsBase.owner.equals(DLP_ID);
  let chipsOnEr = chipsDelegated ? decodeChips((await er.getAccountInfo(chips)).data) : null;
  if (chipsDelegated && chipsOnEr.balance >= BET_AMOUNT) {
    console.log("4. chip stack is delegated and holds enough chips; skipping buy");
  } else {
    if (chipsDelegated) {
      console.log("4. chip stack is delegated but short of chips: undelegate_chips on the rollup");
      await undelegateHome({
        label: "undelegate_chips", account: chips, routerUrl, er, base, wallet, record,
        instruction: () => buildIx(idl, "undelegate_chips", { payer: wallet.publicKey, config: CONFIG, chips },
          wallet.publicKey.toBuffer()),
      });
    }
    console.log("4. buy_chips (10 MUSD) into the cage vault");
    record("buy_chips", await send("buy_chips", base, wallet, buildIx(idl, "buy_chips", {
      pool: POOL, cage_vault: CAGE, chips, owner: wallet.publicKey, owner_musd_ata: musdAta,
      token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
    }, u64(BUY_AMOUNT))), "solana");
    console.log("4. delegate_chip_stack (owner signs)");
    const chipAcc = delegateAccounts(chips);
    record("delegate_chip_stack", await send("delegate_chip_stack", base, wallet, buildIx(idl, "delegate_chip_stack", {
      owner: wallet.publicKey, chips, buffer_chips: chipAcc.buffer, delegation_record_chips: chipAcc.record,
      delegation_metadata_chips: chipAcc.metadata, owner_program: PROGRAM_ID, delegation_program: DLP_ID,
      system_program: SystemProgram.programId,
    })), "solana");
  }

  // 5. The round, delegated.
  const roundBase = await base.getAccountInfo(ROUND);
  if (roundBase && roundBase.owner.equals(DLP_ID)) {
    console.log("5. round already delegated; skipping delegate_round");
  } else {
    console.log("5. delegate_round to the Asia validator");
    const roundAcc = delegateAccounts(ROUND);
    record("delegate_round", await send("delegate_round", base, wallet, buildIx(idl, "delegate_round", {
      payer: wallet.publicKey, config: CONFIG, round: ROUND, buffer_round: roundAcc.buffer,
      delegation_record_round: roundAcc.record, delegation_metadata_round: roundAcc.metadata,
      owner_program: PROGRAM_ID, delegation_program: DLP_ID, system_program: SystemProgram.programId,
    })), "solana");
  }

  // 6. A chip bet on the rollup (no token transfer). UNVERIFIED: the Bet init there.
  console.log("6. place_bet_chips on the rollup (UNVERIFIED: init of the Bet account there)");
  const roundOnEr = decodeRound((await er.getAccountInfo(ROUND)).data);
  const betRoundId = roundOnEr.roundId;
  const bet = betPda(betRoundId, wallet.publicKey, BET_SYMBOL);
  const betSig = await sendEr("place_bet_chips (rollup)", routerUrl, er, wallet, buildIx(idl, "place_bet_chips", {
    table: TABLE, round: ROUND, bet, chips, bettor: wallet.publicKey, system_program: SystemProgram.programId,
  }, Buffer.concat([u8(BET_SYMBOL), u64(BET_AMOUNT)])), [ROUND, chips]);
  record("place_bet_chips", betSig, "rollup");

  // 7. Wait out the 90 s window on the rollup clock, then request the roll there.
  console.log("7. request_roll on DEFAULT_EPHEMERAL_QUEUE (rollup)");
  const opened = decodeRound((await er.getAccountInfo(ROUND)).data).openedAt;
  for (;;) {
    const now = (await er.getAccountInfo(SYSVAR_CLOCK_PUBKEY)).data.readBigInt64LE(32);
    if (now >= opened + WINDOW_SECS) break;
    console.log(`   waiting: rollup clock ${now}, window closes at ${opened + WINDOW_SECS}`);
    await sleep(5000);
  }
  record("request_roll", await sendEr("request_roll (rollup)", routerUrl, er, wallet, buildIx(idl, "request_roll", {
    table: TABLE, round: ROUND, payer: wallet.publicKey, oracle_queue: EPHEMERAL_QUEUE,
    program_identity: PROGRAM_IDENTITY, vrf_program: VRF_PROGRAM, slot_hashes: SLOT_HASHES,
    system_program: SystemProgram.programId,
  }), [ROUND]), "rollup");

  // 8. The VRF callback: the round turns Rolled with six faces.
  console.log("8. waiting for the VRF callback on the rollup");
  const deadline = Date.now() + CALLBACK_MS;
  let final = null;
  while (Date.now() < deadline) {
    const acc = await er.getAccountInfo(ROUND);
    const r = acc ? decodeRound(acc.data) : null;
    if (r && r.status === 3) { final = r; break; }
    await sleep(3000);
  }
  if (!final) {
    console.log("The round did not reach Rolled before the timeout (UNVERIFIED: the callback on the rollup).");
    process.exitCode = 1;
  } else {
    console.log(`Round ${final.roundId} is ${final.statusName}; faces: [${final.faces.join(", ")}]`);
  }

  // 9. Commit and undelegate the round and the chip stack (the table authority), then settle on Solana.
  if (final) {
    console.log("9. commit and undelegate the round and the chip stack (rollup)");
    await undelegateHome({
      label: "undelegate_round", account: ROUND, routerUrl, er, base, wallet, record,
      instruction: () => buildIx(idl, "undelegate_round", { payer: wallet.publicKey, config: CONFIG, round: ROUND }),
    });
    await undelegateHome({
      label: "undelegate_chips", account: chips, routerUrl, er, base, wallet, record,
      instruction: () => buildIx(idl, "undelegate_chips", { payer: wallet.publicKey, config: CONFIG, chips },
        wallet.publicKey.toBuffer()),
    });

    const home = decodeRound((await base.getAccountInfo(ROUND)).data);
    if (home.betCount === 1) {
      console.log("9a. settle_chip_bets on Solana, then finish_round");
      const settle = buildIx(idl, "settle_chip_bets", {
        table: TABLE, round: ROUND, pool: POOL, pool_vault: POOL_VAULT, cage_vault: CAGE, token_program: TOKEN_PROGRAM,
      });
      settle.keys.push({ pubkey: bet, isSigner: false, isWritable: true }, { pubkey: chips, isSigner: false, isWritable: true });
      record("settle_chip_bets", await send("settle_chip_bets", base, wallet, settle), "solana");
      record("finish_round", await send("finish_round", base, wallet, buildIx(idl, "finish_round", { round: ROUND })), "solana");
    } else {
      console.log(`9a. the round holds ${home.betCount} bets; this script settles only a one-bet round (UNVERIFIED: settling every bet is not built here).`);
    }
    const chipsHome = decodeChips((await base.getAccountInfo(chips)).data);
    console.log(`   chips on Solana: ${chipsHome.balance} base units`);
  }

  console.log("\n=== Summary ===");
  for (const r of sigs) {
    const link = r.where === "solana"
      ? explorer(r.sig)
      : `(rollup via ${routerUrl}; Explorer link UNVERIFIED) signature ${r.sig}`;
    console.log(`${r.label}: ${r.sig}\n   ${link}`);
  }
}

main().catch((err) => {
  console.error(`\nDevnet ER round failed: ${err.message ?? err}`);
  process.exitCode = 1;
});
