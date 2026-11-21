#!/usr/bin/env node
/**
 * Devnet ER round, end to end (Step 11.2 + 11.3, D39, D40). Run by the PR checker.
 *
 *   node scripts/devnet-er-round.mjs [keypair.json] [devnetRpc] [erRouter]
 *
 * Steps (each prints its signature and an Explorer link):
 *   0. layout check: prints each account's size against the size the IDL types
 *      give, then runs migrate_table (Solana) and migrate_round (wherever the round
 *      lives: Solana if not delegated, the rollup if delegated) when needed.
 *   1. migrate_config, if the config is still in the old 42-byte layout (upgrade
 *      authority only).
 *   2. init_cage, if the cage vault does not exist yet.
 *   3. musd_faucet, if the wallet has no MUSD account.
 *   4. buy_chips (10 MUSD) on Solana: MUSD moves into the cage vault (D39).
 *   5. delegate_chip_stack on Solana (the owner signs).
 *   6. delegate_round on Solana (table authority only, D40), to the Asia validator.
 *   7. place_bet_chips on the Ephemeral Rollup (the Magic Router), no token transfer.
 *   8. request_roll on the Ephemeral Rollup, on DEFAULT_EPHEMERAL_QUEUE (D11, D20).
 *   9. poll the rollup until the round is Rolled, then print the six faces.
 *
 * Devnet only: any URL that mentions mainnet is refused. The keypair is a
 * gitignored wallet (keys/); its secret is never printed or logged.
 *
 * UNVERIFIED (not checked from the box that wrote this script):
 *   - whether the Magic Router accepts sendRawTransaction and getSignatureStatuses
 *     for rollup transactions, as notes.md describes;
 *   - whether place_bet_chips can `init` a new Bet account on the rollup, paid by a
 *     bettor who is not delegated (Blitz asked for this check);
 *   - whether the rollup's Clock matches wall-clock time for the 90 s window.
 * If a step fails, the script prints the error and exits 1; nothing is retried silently.
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
const DEFAULT_QUEUE = new PublicKey("Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh");
const EPHEMERAL_QUEUE = new PublicKey("5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc");
const ASIA_VALIDATOR = "MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57";
const STATUS = ["Idle", "Open", "Rolling", "Rolled"];
const BET_SYMBOL = 2;
const BET_AMOUNT = 1_000_000n;
const BUY_AMOUNT = 10_000_000n;
const WINDOW_SECS = 90n;
const CALLBACK_MS = 4 * 60 * 1000;

const enc = (s) => new TextEncoder().encode(s);
const pda = (program, ...seeds) => PublicKey.findProgramAddressSync(seeds, program)[0];
const P = (seeds) => PublicKey.findProgramAddressSync(seeds, PROGRAM_ID)[0];
const CONFIG = P([enc("config")]);
const POOL = P([enc("pool")]);
const CAGE = P([enc("cage_vault")]);
const MUSD_MINT = P([enc("musd_mint")]);
const MUSD_AUTH = P([enc("musd_mint_authority")]);
const TABLE = P([enc("table")]);
const ROUND = P([enc("round")]);
const QUEUE = P([enc("payout_queue")]);
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
  buffer: pda(PROGRAM_ID, enc("buffer"), acc.toBytes()),
  record: pda(DLP_ID, enc("delegation"), acc.toBytes()),
  metadata: pda(DLP_ID, enc("delegation-metadata"), acc.toBytes()),
});

const explorer = (sig) => `https://explorer.solana.com/tx/${sig}?cluster=devnet`;

function refuseMainnet(label, url) {
  if (/mainnet/i.test(url)) throw new Error(`Refusing ${label} "${url}": this script is devnet only.`);
}

function loadWallet(keyPath) {
  if (!existsSync(resolve(keyPath))) {
    throw new Error(`No keypair at ${keyPath}. Create one with solana-keygen (gitignored, never committed).`);
  }
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(readFileSync(keyPath, "utf8"))));
}

// The IDL (anchor build writes it) gives each instruction's discriminator and the
// account order, so this script cannot drift from the program.
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

// The rollup takes about 2 minutes to load a new program binary after an upgrade.
// Until then it answers InstructionFallbackNotFound (error 101, 0x65). Only that
// error is retried: wait 30 s, try again, up to 4 times. Every other error is final.
const FALLBACK_PATTERNS = ["InstructionFallbackNotFound", "Error Number: 101", "custom program error: 0x65"];
const FALLBACK_TRIES = 4;
const FALLBACK_WAIT_MS = 30_000;

async function sendEr(label, routerUrl, conn, wallet, instruction, routedAccounts) {
  for (let attempt = 1; ; attempt++) {
    try {
      return await sendErOnce(label, routerUrl, conn, wallet, instruction, routedAccounts);
    } catch (err) {
      const msg = String(err.message ?? err);
      const retryable = FALLBACK_PATTERNS.some((p) => msg.includes(p));
      if (!retryable || attempt >= FALLBACK_TRIES) throw err;
      console.log(`  ${label}: try ${attempt} hit InstructionFallbackNotFound (rollup still loading the upgrade); waiting 30 s, then retrying`);
      await new Promise((r) => setTimeout(r, FALLBACK_WAIT_MS));
    }
  }
}

async function sendErOnce(label, routerUrl, conn, wallet, instruction, routedAccounts) {
  try {
    const { blockhash } = await routerRpc(routerUrl, "getBlockhashForAccounts", [
      routedAccounts.map((p) => p.toBase58()),
    ]);
    const tx = new Transaction({ recentBlockhash: blockhash, feePayer: wallet.publicKey }).add(instruction);
    tx.sign(wallet);
    const sig = await conn.sendRawTransaction(tx.serialize(), { skipPreflight: true });
    for (let i = 0; i < 40; i++) {
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
      await new Promise((r) => setTimeout(r, 500));
    }
    throw new Error(`tx ${sig} confirmation timed out`);
  } catch (err) {
    console.error(`  ${label} FAILED: ${err.message ?? err}`);
    throw err;
  }
}

// Sizes from the IDL types (no hardcoded struct sizes). `Round::players` is a Vec
// with #[max_len(10)], and the IDL does not carry that bound, so it is set here.
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
/** Expected account data length: 8-byte discriminator + the struct's fields. */
const expectedLen = (idl, name) => 8 + sizeOf(idl, { defined: { name } });

async function layoutCheck(idl, base, er, wallet, routerUrl, record) {
  console.log("0. layout check (account size vs the IDL struct)");
  const rows = [];
  const tableAcc = await base.getAccountInfo(TABLE);
  const table = { name: "TableConfig (Solana)", len: tableAcc?.data.length ?? 0, want: expectedLen(idl, "TableConfig") };
  rows.push(table);
  const roundBase = await base.getAccountInfo(ROUND);
  const roundDelegated = !!roundBase && roundBase.owner.equals(DLP_ID);
  const roundAcc = roundDelegated ? await er.getAccountInfo(ROUND) : roundBase;
  const round = { name: `Round (${roundDelegated ? "rollup copy" : "Solana"})`, len: roundAcc?.data.length ?? 0, want: expectedLen(idl, "Round") };
  rows.push(round);
  const cfgAcc = await base.getAccountInfo(CONFIG);
  rows.push({ name: "ProgramConfig (Solana)", len: cfgAcc?.data.length ?? 0, want: expectedLen(idl, "ProgramConfig") });
  const chipsAcc = await base.getAccountInfo(chipsPda(wallet.publicKey));
  if (chipsAcc) rows.push({ name: "ChipStack (Solana)", len: chipsAcc.data.length, want: expectedLen(idl, "ChipStack") });
  for (const r of rows) {
    console.log(`   ${r.name}: ${r.len} bytes, expected ${r.want} -> ${r.len === r.want ? "ok" : "needs migration"}`);
  }

  if (table.len !== table.want) {
    if (table.len !== 34) throw new Error(`TableConfig is ${table.len} bytes: no known migration.`);
    console.log("0a. migrate_table (Solana, upgrade authority)");
    const sig = await send("migrate_table", base, wallet, buildIx(idl, "migrate_table", {
      table: TABLE, upgrade_authority: wallet.publicKey, program_data: PROGRAM_DATA,
      system_program: SystemProgram.programId,
    }));
    record("migrate_table", sig, "solana");
  }

  if (round.len !== round.want) {
    if (round.len !== 388) throw new Error(`Round is ${round.len} bytes: no known migration.`);
    const accounts = {
      round: ROUND, upgrade_authority: wallet.publicKey, program_data: PROGRAM_DATA,
      system_program: SystemProgram.programId,
    };
    if (roundDelegated) {
      // The delegated copy lives on the rollup, so the migration runs there.
      // UNVERIFIED: whether the rollup allows resizing a delegated account.
      console.log("0b. migrate_round on the rollup (UNVERIFIED: resizing a delegated account)");
      const sig = await sendEr("migrate_round (rollup)", routerUrl, er, wallet,
        buildIx(idl, "migrate_round", accounts), [ROUND]);
      record("migrate_round", sig, "rollup");
      // A Rolled leftover round is finished on the rollup, so the bet can open a new one.
      const after = decodeRound((await er.getAccountInfo(ROUND)).data);
      if (after.status === 3) {
        console.log("0c. the migrated round is Rolled; finish_round on the rollup");
        const fsig = await sendEr("finish_round (rollup)", routerUrl, er, wallet,
          buildIx(idl, "finish_round", { round: ROUND }), [ROUND]);
        record("finish_round", fsig, "rollup");
      }
    } else {
      console.log("0b. migrate_round on Solana (upgrade authority)");
      const sig = await send("migrate_round", base, wallet, buildIx(idl, "migrate_round", accounts));
      record("migrate_round", sig, "solana");
    }
  }
}

function decodeRound(data) {
  let o = 8;
  const status = data[o++];
  const openedAt = data.readBigInt64LE(o); o += 8;
  const roundId = data.readBigUInt64LE(o); o += 8;
  const n = data.readUInt32LE(o); o += 4 + n * 32;
  const faces = [...data.subarray(o + 32, o + 38)];
  return { status, statusName: STATUS[status] ?? `unknown(${status})`, openedAt, roundId, faces };
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
  const balance = await base.getBalance(wallet.publicKey);
  if (balance < 0.05 * 1e9) throw new Error("The wallet needs more devnet SOL (at least 0.05).");

  const sigs = [];
  const record = (label, sig, where) => sigs.push({ label, sig, where });

  // 0. Layout check and the migrations it needs (table, and the round wherever it lives).
  await layoutCheck(idl, base, er, wallet, routerUrl, record);

  // 1. Migrate the config if it is still in the old 42-byte layout.
  const cfg = await base.getAccountInfo(CONFIG);
  if (!cfg) throw new Error("The config is missing on devnet: run initialize first.");
  if (cfg.data.length === 42) {
    console.log("1. config is in the old layout; migrate_config (upgrade authority only)");
    const sig = await send("migrate_config", base, wallet, buildIx(idl, "migrate_config", {
      config: CONFIG, upgrade_authority: wallet.publicKey, program_data: PROGRAM_DATA,
      system_program: SystemProgram.programId,
    }));
    record("migrate_config", sig, "solana");
  } else {
    console.log(`1. config already in the current layout (${cfg.data.length} bytes)`);
  }
  // The config now starts with `admin`: only that key can delegate the round (D40).
  const admin = new PublicKey((await base.getAccountInfo(CONFIG)).data.subarray(8, 40));
  if (!admin.equals(wallet.publicKey)) {
    throw new Error(`This wallet is not the table authority (admin is ${admin.toBase58()}). Only it can delegate (D40).`);
  }

  // 2. The cage vault (chip money, D39).
  if (!(await base.getAccountInfo(CAGE))) {
    console.log("2. cage vault missing; init_cage");
    const sig = await send("init_cage", base, wallet, buildIx(idl, "init_cage", {
      config: CONFIG, pool: POOL, cage_vault: CAGE, musd_mint: MUSD_MINT, payer: wallet.publicKey,
      token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
    }));
    record("init_cage", sig, "solana");
  } else {
    console.log("2. cage vault exists");
  }

  // 3. MUSD for the wallet (the faucet; one claim per hour per wallet).
  const musdAta = ataOf(wallet.publicKey, MUSD_MINT);
  if (!(await base.getAccountInfo(musdAta))) {
    console.log("3. no MUSD account yet; musd_faucet");
    const sig = await send("musd_faucet", base, wallet, buildIx(idl, "musd_faucet", {
      config: CONFIG, musd_mint: MUSD_MINT, musd_mint_authority: MUSD_AUTH, faucet_record: faucetPda(wallet.publicKey),
      wallet: wallet.publicKey, destination: musdAta, associated_token_program: ATA_PROGRAM,
      token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
    }));
    record("musd_faucet", sig, "solana");
  } else {
    console.log("3. MUSD account exists");
  }

  // 4-5. Buy chips and delegate the chip stack (skip if already delegated).
  const chips = chipsPda(wallet.publicKey);
  const chipsBase = await base.getAccountInfo(chips);
  if (chipsBase && chipsBase.owner.equals(DLP_ID)) {
    console.log("4-5. chip stack already delegated on Solana; skipping buy_chips & delegate_chip_stack");
  } else {
    console.log("4. buy_chips (10 MUSD) into the cage vault");
    record("buy_chips", await send("buy_chips", base, wallet, buildIx(idl, "buy_chips", {
      pool: POOL, cage_vault: CAGE, chips, owner: wallet.publicKey, owner_musd_ata: musdAta,
      token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
    }, u64(BUY_AMOUNT))), "solana");

    console.log("5. delegate_chip_stack (owner signs)");
    const chipAcc = delegateAccounts(chips);
    record("delegate_chip_stack", await send("delegate_chip_stack", base, wallet, buildIx(idl, "delegate_chip_stack", {
      owner: wallet.publicKey, chips, buffer_chips: chipAcc.buffer, delegation_record_chips: chipAcc.record,
      delegation_metadata_chips: chipAcc.metadata, owner_program: PROGRAM_ID, delegation_program: DLP_ID,
      system_program: SystemProgram.programId,
    })), "solana");
  }

  // 6. Delegate the round (the table authority only, D40; skip if already delegated).
  const roundBase = await base.getAccountInfo(ROUND);
  if (roundBase && roundBase.owner.equals(DLP_ID)) {
    console.log("6. round already delegated on Solana; skipping delegate_round");
  } else {
    console.log("6. delegate_round to the Asia validator");
    const roundAcc = delegateAccounts(ROUND);
    record("delegate_round", await send("delegate_round", base, wallet, buildIx(idl, "delegate_round", {
      payer: wallet.publicKey, config: CONFIG, round: ROUND, buffer_round: roundAcc.buffer,
      delegation_record_round: roundAcc.record, delegation_metadata_round: roundAcc.metadata,
      owner_program: PROGRAM_ID, delegation_program: DLP_ID, system_program: SystemProgram.programId,
    })), "solana");
  }

  // 7. A chip bet on the rollup (no token transfer).
  console.log("7. place_bet_chips on the rollup (UNVERIFIED: init of the Bet account there)");
  const before = await er.getAccountInfo(ROUND);
  const roundOnEr = before ? decodeRound(before.data) : null;
  if (!roundOnEr) throw new Error("The round is not readable on the rollup yet. Retry after a few seconds.");
  const bet = betPda(roundOnEr.roundId, wallet.publicKey, BET_SYMBOL);
  const betSig = await sendEr("place_bet_chips (rollup)", routerUrl, er, wallet, buildIx(idl, "place_bet_chips", {
    table: TABLE, round: ROUND, bet, chips, bettor: wallet.publicKey, system_program: SystemProgram.programId,
  }, Buffer.concat([u8(BET_SYMBOL), u64(BET_AMOUNT)])), [ROUND, chips]);
  record("place_bet_chips", betSig, "rollup");

  // 8. Wait out the 90 s window on the rollup clock, then request the roll there.
  console.log("8. request_roll on DEFAULT_EPHEMERAL_QUEUE (rollup)");
  const afterBetAcc = await er.getAccountInfo(ROUND);
  const afterBet = afterBetAcc ? decodeRound(afterBetAcc.data) : roundOnEr;
  const opened = afterBet.openedAt;
  const windowEnd = opened + WINDOW_SECS;
  for (;;) {
    const clock = await er.getAccountInfo(SYSVAR_CLOCK_PUBKEY);
    const now = clock.data.readBigInt64LE(32);
    if (now >= windowEnd) break;
    console.log(`   waiting: rollup clock ${now}, window closes at ${windowEnd}`);
    await new Promise((r) => setTimeout(r, 5000));
  }
  const reqSig = await sendEr("request_roll (rollup)", routerUrl, er, wallet, buildIx(idl, "request_roll", {
    table: TABLE, round: ROUND, payer: wallet.publicKey, oracle_queue: EPHEMERAL_QUEUE,
    program_identity: PROGRAM_IDENTITY, vrf_program: new PublicKey("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz"),
    slot_hashes: new PublicKey("SysvarS1otHashes111111111111111111111111111"),
    system_program: SystemProgram.programId,
  }), [ROUND]);
  record("request_roll", reqSig, "rollup");

  // 9. Wait for the callback: the round turns Rolled with six faces.
  console.log("9. waiting for the VRF callback on the rollup");
  const deadline = Date.now() + CALLBACK_MS;
  let final = null;
  while (Date.now() < deadline) {
    const acc = await er.getAccountInfo(ROUND);
    const r = acc ? decodeRound(acc.data) : null;
    if (r && r.status === 3) { final = r; break; }
    await new Promise((res) => setTimeout(res, 3000));
  }

  console.log("\n=== Summary ===");
  for (const r of sigs) {
    const link = r.where === "solana" ? explorer(r.sig) : `(rollup via ${routerUrl}; Explorer link UNVERIFIED) signature ${r.sig}`;
    console.log(`${r.label}: ${r.sig}\n   ${link}`);
  }
  if (final) {
    console.log(`Round ${final.roundId} is ${final.statusName}; faces: [${final.faces.join(", ")}]`);
  } else {
    console.log("The round did not reach Rolled before the timeout (UNVERIFIED: the callback on the rollup).");
    process.exitCode = 1;
  }
}

main().catch((err) => {
  console.error(`\nDevnet ER round failed: ${err.message ?? err}`);
  process.exitCode = 1;
});
