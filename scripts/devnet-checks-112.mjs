#!/usr/bin/env node
/**
 * Devnet proof run for PR #112's seven checks. Devnet only. The keypair is a
 * gitignored wallet (keys/); its secret is never printed or logged.
 *
 *   node scripts/devnet-checks-112.mjs [keypair.json] [devnetRpc]
 *
 * This script covers checks 2–6 on Solana (admin setup, chip queue, session
 * bet, snapshot, and a base-layer VRF callback proof). Check 1 is the program
 * upgrade (done separately). Check 7 is `scripts/devnet-er-round.mjs` plus
 * `node --test scripts/round-proof.test.mjs`.
 *
 * Missing accounts this PR needs (payout queue, MUNDA mint, stake pool, insurance)
 * are created if they are absent. If the pool vault is empty while LP shares still
 * exist, deposit would divide by zero (a program gap, not fixed here); the script
 * unsicks that state with a temporary MUSD transfer and a full withdraw, then
 * leaves the pool empty so a chip win will join the queue.
 */
import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { explorerUrl as proofLink, callbackProof } from "./lib/round-proof.mjs";

const web3 = createRequire(new URL("../app/package.json", import.meta.url))("@solana/web3.js");
const {
  Connection, Keypair, PublicKey, SystemProgram, Transaction, TransactionInstruction,
  sendAndConfirmTransaction, LAMPORTS_PER_SOL,
} = web3;

const DEFAULT_RPC = "https://api.devnet.solana.com";
const DEFAULT_KEYPAIR = "keys/dev-wallet.json";
const PROGRAM_ID = new PublicKey("4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH");
const SESSION_PROGRAM = new PublicKey("KeyspM2ssCJbqUhQ4k7sveSiY4WjnYsrXkC8oDbwde5");
const TOKEN_PROGRAM = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ATA_PROGRAM = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const DEFAULT_QUEUE = new PublicKey("Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh");
const VRF_PROGRAM = new PublicKey("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz");
const SLOT_HASHES = new PublicKey("SysvarS1otHashes111111111111111111111111111");
const STATUS = ["Idle", "Open", "Rolling", "Rolled"];
const BET_SYMBOL = 2;
const BET_AMOUNT = 1_000_000n;
const SESSION_CAP = 5_000_000n;
const SESSION_SECS = 3600n;
const MULTIPLIERS = [0n, 0n, 3n, 5n, 9n, 26n, 101n];
const WIN_FEE_BPS = 200n;
const CALLBACK_MS = 4 * 60 * 1000;
const MAX_QUEUE_TRIES = 8;
const CREATE_SESSION_DISC = Buffer.from([242, 193, 143, 179, 150, 25, 122, 227]);

const enc = (s) => new TextEncoder().encode(s);
const P = (seeds, program = PROGRAM_ID) => PublicKey.findProgramAddressSync(seeds, program)[0];
const CONFIG = P([enc("config")]);
const POOL = P([enc("pool")]);
const POOL_VAULT = P([enc("pool_vault")]);
const SHARE_MINT = P([enc("pool_share_mint")]);
const INSURANCE_VAULT = P([enc("insurance_vault")]);
const QUEUE = P([enc("payout_queue")]);
const CAGE = P([enc("cage_vault")]);
const MUSD_MINT = P([enc("musd_mint")]);
const MUSD_AUTH = P([enc("musd_mint_authority")]);
const TABLE = P([enc("table")]);
const ROUND = P([enc("round_v2")]);
const PROGRAM_IDENTITY = P([enc("identity")]);
const MUNDA_MINT = P([enc("munda_mint")]);
const MUNDA_AUTH = P([enc("munda_mint_authority")]);
const STAKE_POOL = P([enc("stake_pool")]);
const STAKE_VAULT = P([enc("stake_vault")]);
const STAKE_AUTH = P([enc("stake_authority")]);
const chipsPda = (owner) => P([enc("chips"), owner.toBytes()]);
const capPda = (owner) => P([enc("session_cap"), owner.toBytes()]);
const snapshotPda = (day) => {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(day));
  return P([enc("snapshot"), b]);
};
const sessionTokenPda = (target, signer, authority) => P(
  [enc("session_token"), target.toBytes(), signer.toBytes(), authority.toBytes()],
  SESSION_PROGRAM,
);
const ataOf = (owner, mint) => PublicKey.findProgramAddressSync(
  [owner.toBytes(), TOKEN_PROGRAM.toBytes(), mint.toBytes()], ATA_PROGRAM,
)[0];
const betPda = (roundId, player, symbol) => {
  const id = Buffer.alloc(8);
  id.writeBigUInt64LE(BigInt(roundId));
  return P([enc("bet"), id, player.toBytes(), Uint8Array.of(symbol)]);
};

const explorer = (sig) => `https://explorer.solana.com/tx/${sig}?cluster=devnet`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const u64 = (n) => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; };
const i64 = (n) => { const b = Buffer.alloc(8); b.writeBigInt64LE(BigInt(n)); return b; };
const u8 = (n) => Buffer.from([n]);
const opt = (tag, payload) => Buffer.concat([u8(tag), payload]);

function refuseMainnet(label, url) {
  if (/mainnet/i.test(url)) throw new Error(`Refusing ${label} "${url}": this script is devnet only.`);
}

function loadWallet(keyPath) {
  if (!existsSync(resolve(keyPath))) {
    throw new Error(`No keypair at ${keyPath}. Create one with solana-keygen (gitignored, never committed).`);
  }
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(readFileSync(keyPath, "utf8"))));
}

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
  return new TransactionInstruction({
    programId: PROGRAM_ID, keys, data: Buffer.concat([Buffer.from(def.discriminator), args]),
  });
}

function decodeRound(data) {
  let o = 8;
  const status = data[o++];
  const openedAt = data.readBigInt64LE(o); o += 8;
  const roundId = data.readBigUInt64LE(o); o += 8;
  const n = data.readUInt32LE(o);
  o += 4 + n * 32 + 32;
  const faces = [...data.subarray(o, o + 6)];
  const betCount = data.length >= 392 ? data.readUInt16LE(o + 6) : null;
  const settledCount = data.length >= 392 ? data.readUInt16LE(o + 8) : null;
  return { status, statusName: STATUS[status] ?? `unknown(${status})`, openedAt, roundId, faces, betCount, settledCount };
}

function decodeChips(data) {
  return { owner: new PublicKey(data.subarray(8, 40)), balance: data.readBigUInt64LE(40) };
}

function decodeBet(data) {
  return {
    player: new PublicKey(data.subarray(8, 40)),
    amount: data.readBigUInt64LE(40),
    symbol: data[48],
    roundId: data.readBigUInt64LE(49),
    owed: data.readBigUInt64LE(57),
    settled: data[65] === 1,
    paidWithChips: data[74] === 1,
  };
}

function tokenAmount(data) { return data.readBigUInt64LE(64); }
function mintSupply(data) { return data.readBigUInt64LE(36); }

function matches(faces, symbol) {
  return faces.filter((f) => f === symbol).length;
}

function owedFor(stake, faces, symbol) {
  const k = matches(faces, symbol);
  const gross = stake * (MULTIPLIERS[k] ?? 0n);
  if (gross === 0n) return 0n;
  if (gross <= stake) return gross;
  const profit = gross - stake;
  const kept = profit - (profit * WIN_FEE_BPS / 10_000n);
  return stake + kept;
}

function decodeQueue(data) {
  const head = data.readBigUInt64LE(8);
  const tail = data.readBigUInt64LE(16);
  const entries = [];
  // Zero-copy: 8-byte disc, then two u64s, then [QueueEntry; 100] of (pubkey, u64).
  let o = 24;
  for (let i = 0; i < Number(tail - head) && i < 100; i++) {
    const slot = Number((head + BigInt(i)) % 100n);
    const off = o + slot * 40;
    entries.push({
      key: new PublicKey(data.subarray(off, off + 32)).toBase58(),
      amount: data.readBigUInt64LE(off + 32),
    });
  }
  return { head, tail, length: tail - head, entries };
}

async function send(label, conn, signers, instruction) {
  const wallet = signers[0];
  try {
    const sig = await sendAndConfirmTransaction(conn, new Transaction().add(instruction), signers, {
      commitment: "confirmed",
    });
    console.log(`  ${label}: ${explorer(sig)}`);
    return sig;
  } catch (err) {
    console.error(`  ${label} FAILED: ${err.message ?? err}`);
    throw err;
  }
}

async function findCallbackSig(conn, account) {
  const recent = await conn.getSignaturesForAddress(account, { limit: 30 }).catch(() => []);
  for (const entry of recent) {
    const tx = await conn.getTransaction(entry.signature, {
      commitment: "confirmed", maxSupportedTransactionVersion: 0,
    }).catch(() => null);
    const logs = tx?.meta?.logMessages ?? [];
    const proof = callbackProof(logs);
    if (proof) return { signature: entry.signature, proof, logs };
  }
  return null;
}

async function main() {
  const args = process.argv.slice(2).filter((a) => a !== "--queue-only");
  const queueOnly = process.argv.includes("--queue-only");
  const keyPath = args[0] ?? DEFAULT_KEYPAIR;
  const rpc = args[1] ?? DEFAULT_RPC;
  refuseMainnet("rpc", rpc);
  const wallet = loadWallet(keyPath);
  const idl = loadIdl();
  const base = new Connection(rpc, "confirmed");
  const sigs = [];
  const record = (name, sig) => { sigs.push({ name, sig }); return sig; };
  const results = [];
  const note = (check, status, detail) => {
    results.push({ check, status, detail });
    console.log(`\n== CHECK ${check}: ${status} ==\n${detail}`);
  };

  console.log(`Wallet: ${wallet.publicKey.toBase58()}`);
  console.log(`RPC: ${rpc}`);
  const sol = await base.getBalance(wallet.publicKey);
  console.log(`Balance: ${sol / LAMPORTS_PER_SOL} SOL`);

  const musdAta = ataOf(wallet.publicKey, MUSD_MINT);
  const shareAta = ataOf(wallet.publicKey, SHARE_MINT);
  const chips = chipsPda(wallet.publicKey);

  const info = async (pk) => base.getAccountInfo(pk);
  const requireAdmin = async () => {
    const cfg = await info(CONFIG);
    if (!cfg) throw new Error("config account is missing on devnet");
    const admin = new PublicKey(cfg.data.subarray(8, 40));
    if (!admin.equals(wallet.publicKey)) {
      throw new Error(`This wallet is not the config admin (admin is ${admin.toBase58()}).`);
    }
    return admin;
  };

  const admin = await requireAdmin();
  if (queueOnly) console.log("queue-only: skip setup checks 2, 4 and 5; empty the pool and retry short-pool settles");

  // --- missing accounts this PR needs ---
  const ensure = async (label, pk, builder) => {
    if (await info(pk)) {
      console.log(`${label} exists (${pk.toBase58()})`);
      return null;
    }
    console.log(`${label} missing; creating`);
    return record(label, await send(label, base, [wallet], builder()));
  };

  const insuranceSig = await ensure("init_insurance", INSURANCE_VAULT, () => buildIx(idl, "init_insurance", {
    config: CONFIG, insurance_vault: INSURANCE_VAULT, musd_mint: MUSD_MINT, payer: wallet.publicKey,
    token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
  }));
  const queueSig = await ensure("init_queue", QUEUE, () => buildIx(idl, "init_queue", {
    queue: QUEUE, payer: wallet.publicKey, system_program: SystemProgram.programId,
  }));
  const mundaSig = await ensure("init_munda", MUNDA_MINT, () => buildIx(idl, "init_munda", {
    munda_mint: MUNDA_MINT, munda_mint_authority: MUNDA_AUTH, payer: wallet.publicKey,
    token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
  }));
  const stakingSig = await ensure("init_staking", STAKE_POOL, () => buildIx(idl, "init_staking", {
    stake_pool: STAKE_POOL, stake_vault: STAKE_VAULT, munda_mint: MUNDA_MINT, stake_authority: STAKE_AUTH,
    payer: wallet.publicKey, token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
  }));

  const table = await info(TABLE);
  const pool = await info(POOL);
  const tableOk = !!table;
  const poolOk = !!pool;
  note(2, tableOk && poolOk ? "PASS" : "FAIL",
    `initialize_table / initialize_pool were already run as the config admin (${admin.toBase58()}).\n` +
    `  table: ${TABLE.toBase58()} ${tableOk ? `exists (${table.data.length} bytes)` : "MISSING"}\n` +
    `  pool:  ${POOL.toBase58()} ${poolOk ? `exists (${pool.data.length} bytes)` : "MISSING"}\n` +
    `  insurance: ${INSURANCE_VAULT.toBase58()} ${insuranceSig ? `created ${explorer(insuranceSig)}` : "already existed"}\n` +
    `  A second initialize_table/initialize_pool cannot be sent: the accounts already exist, so Anchor refuses at \`init\` before the new admin check runs. The admin-only constraint is covered by the unit test.`);

  // --- repair insolvent pool (shares > 0, vault == 0): program deposit would divide by zero ---
  async function poolState() {
    const vault = await info(POOL_VAULT);
    const mint = await info(SHARE_MINT);
    const held = await info(shareAta);
    return {
      vault: vault ? tokenAmount(vault.data) : 0n,
      supply: mint ? mintSupply(mint.data) : 0n,
      shares: held ? tokenAmount(held.data) : 0n,
    };
  }

  async function resetPoolToEmpty() {
    let st = await poolState();
    console.log(`  pool vault=${st.vault} supply=${st.supply} wallet_shares=${st.shares}`);
    if (st.supply === 0n && st.vault === 0n) return { repaired: false, ...st };
    if (st.supply > 0n && st.vault === 0n) {
      console.log("  PROGRAM GAP: share supply is > 0 and the vault is 0. deposit would divide by zero; withdraw would pay 0 and refuse. Transferring 1 MUSD into the vault so we can withdraw all shares and return to a 0/0 pool. This is a script workaround, not a program fix.");
      if (st.shares !== st.supply) {
        throw new Error(`This wallet holds ${st.shares} of ${st.supply} LP shares; cannot clean the insolvent pool alone.`);
      }
      const have = tokenAmount((await info(musdAta)).data);
      if (have < 1_000_000n) throw new Error("wallet needs at least 1 MUSD to unsick the empty-vault/nonzero-shares pool");
      record("spl_transfer_to_vault", await send("spl_transfer 1 MUSD into pool vault (workaround)", base, [wallet],
        new TransactionInstruction({
          programId: TOKEN_PROGRAM,
          keys: [
            { pubkey: musdAta, isSigner: false, isWritable: true },
            { pubkey: POOL_VAULT, isSigner: false, isWritable: true },
            { pubkey: wallet.publicKey, isSigner: true, isWritable: false },
          ],
          data: Buffer.concat([u8(3), u64(1_000_000n)]),
        })));
      st = await poolState();
    }
    // If shares are 0 and the vault holds leftover (a lost chip stake), D43 deposit
    // sweeps that orphan to insurance, then we withdraw the new shares to get back to 0/0.
    // Leaving it in the vault would eventually cover a 2-match win and skip the queue.
    if (st.supply === 0n && st.vault > 0n) {
      console.log(`  vault holds ${st.vault} orphan MUSD with 0 shares; D43 deposit 1 MUSD, then withdraw to empty the pool`);
      const have = tokenAmount((await info(musdAta)).data);
      if (have < 1_000_000n) throw new Error("wallet needs 1 MUSD to sweep orphan pool funds via D43");
      record("deposit_d43_sweep", await send("deposit 1 MUSD (D43 orphan sweep)", base, [wallet], buildIx(idl, "deposit", {
        pool: POOL, config: CONFIG, share_mint: SHARE_MINT, vault: POOL_VAULT, depositor: wallet.publicKey, depositor_ata: musdAta,
        depositor_shares: shareAta, insurance_vault: INSURANCE_VAULT, queue: QUEUE, token_program: TOKEN_PROGRAM,
        system_program: SystemProgram.programId, associated_token_program: ATA_PROGRAM,
      }, u64(1_000_000n))));
      st = await poolState();
    }
    if (st.shares > 0n && st.vault > 0n) {
      console.log(`  withdrawing all ${st.shares} LP shares to empty the pool`);
      record("withdraw_all_shares", await send("withdraw all LP shares", base, [wallet], buildIx(idl, "withdraw", {
        pool: POOL, config: CONFIG, share_mint: SHARE_MINT, vault: POOL_VAULT, withdrawer: wallet.publicKey,
        withdrawer_shares: shareAta, withdrawer_ata: musdAta, queue: QUEUE, token_program: TOKEN_PROGRAM,
      }, u64(st.shares))));
      st = await poolState();
    }
    console.log(`  pool after reset: vault=${st.vault} supply=${st.supply} wallet_shares=${st.shares}`);
    return { repaired: true, ...st };
  }

  await resetPoolToEmpty();

  if (queueOnly) {
    console.log("queue-only: skipping snapshot and session setup");
  }

  let session = null, tokenPda = null, cap = null, grantSig = null, createSessSig = null;

  if (!queueOnly) {
  // --- check 5: take_snapshot today, second crank must fail ---
  const day = BigInt(Math.floor(Date.now() / 1000 / 86_400));
  const snap = snapshotPda(day);
  let snapSig = null;
  let snapSecond = "not run";
  try {
    if (await info(snap)) {
      console.log(`5. today's snapshot already exists for day ${day} (${snap.toBase58()}); skipping the first crank`);
      snapSig = "(already existed)";
    } else {
      console.log(`5. take_snapshot for UTC day ${day}`);
      snapSig = record("take_snapshot", await send("take_snapshot", base, [wallet], buildIx(idl, "take_snapshot", {
        payer: wallet.publicKey, snapshot: snap, pool: POOL, share_mint: SHARE_MINT, vault: POOL_VAULT,
        stake_pool: STAKE_POOL, queue: QUEUE, system_program: SystemProgram.programId,
      }, u64(day))));
    }
    try {
      await send("take_snapshot (second, same day — should fail)", base, [wallet], buildIx(idl, "take_snapshot", {
        payer: wallet.publicKey, snapshot: snap, pool: POOL, share_mint: SHARE_MINT, vault: POOL_VAULT,
        stake_pool: STAKE_POOL, queue: QUEUE, system_program: SystemProgram.programId,
      }, u64(day)));
      snapSecond = "FAIL: the second crank succeeded";
    } catch (err) {
      snapSecond = `failed as expected (${(err.message ?? String(err)).split("\n")[0]})`;
    }
    const snapAcc = await info(snap);
    const nav = snapAcc.data.readBigUInt64LE(8 + 8 + 8);
    const poolSize = snapAcc.data.readBigUInt64LE(8 + 8 + 8 + 8);
    note(5, snapSecond.startsWith("FAIL:") ? "FAIL" : "PASS",
      `take_snapshot day ${day}: ${typeof snapSig === "string" && snapSig.startsWith("http") ? snapSig : snapSig === "(already existed)" ? "already existed" : explorer(snapSig)}\n` +
      `  snapshot PDA ${snap.toBase58()} nav_per_share=${nav} pool_size=${poolSize}\n` +
      `  second crank same day: ${snapSecond}`);
  } catch (err) {
    note(5, "FAIL", `take_snapshot failed: ${err.message ?? err}`);
  }

  // --- checks 4 then 3: session bet on an open Solana round, then short-pool settle + pay_queue ---
  // A session signer is generated in memory and never written to the repo.
  session = Keypair.generate();
  console.log(`Session signer pubkey: ${session.publicKey.toBase58()} (secret not printed)`);
  tokenPda = sessionTokenPda(PROGRAM_ID, session.publicKey, wallet.publicKey);
  cap = capPda(wallet.publicKey);

  // Fund the session signer so it can pay the bet tx fee.
  record("fund_session_signer", await send("fund session signer 0.02 SOL", base, [wallet],
    SystemProgram.transfer({ fromPubkey: wallet.publicKey, toPubkey: session.publicKey, lamports: 0.02 * LAMPORTS_PER_SOL })));

  if (await info(cap)) {
    console.log(`grant_session: SessionCap already exists at ${cap.toBase58()}; not re-created (init-only)`);
  } else {
    grantSig = record("grant_session", await send("grant_session", base, [wallet], buildIx(idl, "grant_session", {
      owner: wallet.publicKey, cap, system_program: SystemProgram.programId,
    }, Buffer.concat([session.publicKey.toBuffer(), u64(SESSION_CAP), i64(SESSION_SECS)]))));
  }

  if (await info(tokenPda)) {
    console.log(`create_session: V1 SessionToken already exists at ${tokenPda.toBase58()}`);
  } else {
    const now = Math.floor(Date.now() / 1000);
    const data = Buffer.concat([
      CREATE_SESSION_DISC,
      opt(1, u8(1)),           // Some(true) top_up
      opt(1, i64(now + 3500)), // Some(valid_until)
      opt(1, u64(0)),          // Some(0) extra lamports (already funded)
    ]);
    createSessSig = record("create_session", await send("create_session (session-keys V1)", base, [wallet, session],
      new TransactionInstruction({
        programId: SESSION_PROGRAM,
        keys: [
          { pubkey: tokenPda, isSigner: false, isWritable: true },
          { pubkey: session.publicKey, isSigner: true, isWritable: true },
          { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
          { pubkey: PROGRAM_ID, isSigner: false, isWritable: false },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        data,
      })));
  }

  } // end !queueOnly snapshot + session grant

  let sessionBetSig = queueOnly ? "wallet" : null;
  let sessionBetDetail = queueOnly ? "queue-only retry: wallet-signed place_bet_chips" : "";
  let queued = null;
  let paySig = null;
  let settleSig = null;
  let rollSig = null;
  let callback = null;
  let lastFaces = null;

  async function currentRound() {
    const acc = await info(ROUND);
    if (!acc) throw new Error("round_v2 is missing");
    if (!acc.owner.equals(PROGRAM_ID)) throw new Error(`round_v2 is not on Solana (owner ${acc.owner.toBase58()}); undelegate it first`);
    return decodeRound(acc.data);
  }

  // Make sure we have chips on Solana for the bets.
  let chipAcc = await info(chips);
  let chipBal = chipAcc ? decodeChips(chipAcc.data).balance : 0n;
  if (chipBal < BET_AMOUNT * BigInt(MAX_QUEUE_TRIES)) {
    const need = BET_AMOUNT * BigInt(MAX_QUEUE_TRIES) - chipBal;
    const buy = need < 10_000_000n ? 10_000_000n : need;
    console.log(`buy_chips ${buy} (current balance ${chipBal})`);
    record("buy_chips", await send("buy_chips", base, [wallet], buildIx(idl, "buy_chips", {
      pool: POOL, cage_vault: CAGE, chips, owner: wallet.publicKey, owner_musd_ata: musdAta,
      token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
    }, u64(buy))));
  }

  // If a previous run settled a short-pool win and then stopped, pay the queue and finish.
  {
    const q0 = await info(QUEUE);
    if (q0) {
      const pending = decodeQueue(q0.data);
      if (pending.length > 0n) {
        console.log(`resume: payout queue already holds ${pending.length} ${JSON.stringify(pending.entries, (_, v) => typeof v === "bigint" ? v.toString() : v)}`);
        const st = await poolState();
        const due = pending.entries[0].amount;
        const shortfall = due > st.vault ? due - st.vault : 0n;
        const depositAmt = shortfall + 1_000_000n;
        if (depositAmt > 0n && shortfall > 0n) {
          record("deposit", await send("deposit (to pay the queue)", base, [wallet], buildIx(idl, "deposit", {
            pool: POOL, config: CONFIG, share_mint: SHARE_MINT, vault: POOL_VAULT, depositor: wallet.publicKey, depositor_ata: musdAta,
            depositor_shares: shareAta, insurance_vault: INSURANCE_VAULT, queue: QUEUE, token_program: TOKEN_PROGRAM,
            system_program: SystemProgram.programId, associated_token_program: ATA_PROGRAM,
          }, u64(depositAmt))));
        }
        const chipsBefore = decodeChips((await info(chips)).data).balance;
        const pay = buildIx(idl, "pay_queue", { queue: QUEUE, pool: POOL, pool_vault: POOL_VAULT, token_program: TOKEN_PROGRAM });
        pay.keys.push({ pubkey: chips, isSigner: false, isWritable: true });
        pay.keys.push({ pubkey: CAGE, isSigner: false, isWritable: true });
        paySig = record("pay_queue", await send("pay_queue", base, [wallet], pay));
        const qAfter = decodeQueue((await info(QUEUE)).data);
        const chipsAfter = decodeChips((await info(chips)).data).balance;
        const r0 = await currentRound();
        if (r0.status === 3) {
          record("finish_round", await send("finish_round", base, [wallet], buildIx(idl, "finish_round", { round: ROUND })));
        }
        queued = { due, faces: r0.faces, k: matches(r0.faces, BET_SYMBOL), settleSig: "(from earlier run)", roundId: r0.roundId };
        note(3, qAfter.length === 0n && chipsAfter - chipsBefore === due ? "PASS" : "FAIL",
          `resumed a queued chip win of ${due}.\n  pay_queue ${explorer(paySig)} credited ${chipsAfter - chipsBefore} chips (queue length now ${qAfter.length})`);
      }
    }
  }

  if (!queued) for (let attempt = 1; attempt <= MAX_QUEUE_TRIES; attempt++) {
    await resetPoolToEmpty();
    let r = await currentRound();
    if (r.status !== 0) {
      throw new Error(`round_v2 is ${r.statusName}, expected Idle before opening a new slip`);
    }
    const roundId = r.roundId;
    const bet = betPda(roundId, wallet.publicKey, BET_SYMBOL);
    console.log(`\nAttempt ${attempt}/${MAX_QUEUE_TRIES}: Idle round_id=${roundId}`);

    if (!(await info(bet))) {
      record("open_chip_bet", await send(`open_chip_bet round ${roundId} face ${BET_SYMBOL}`, base, [wallet], buildIx(idl, "open_chip_bet", {
        round: ROUND, bet, bettor: wallet.publicKey, system_program: SystemProgram.programId,
      }, Buffer.concat([u64(roundId), u8(BET_SYMBOL)]))));
    } else {
      const slip = decodeBet((await info(bet)).data);
      if (slip.amount > 0n) throw new Error(`bet slip for round ${roundId} is already filled`);
      console.log("  bet slip already open and empty");
    }

    if (!queueOnly && !sessionBetSig) {
      // Check 4: the session key places the chip bet on Solana.
      sessionBetSig = record("place_bet_chips_session", await send("place_bet_chips_session", base, [session], buildIx(idl, "place_bet_chips_session", {
        table: TABLE, round: ROUND, bet, chips, cap, session_token: tokenPda,
        owner: wallet.publicKey, session_signer: session.publicKey,
      }, Buffer.concat([u8(BET_SYMBOL), u64(BET_AMOUNT)]))));
      sessionBetDetail = `grant_session ${grantSig ? explorer(grantSig) : "(cap already existed)"}\n` +
        `  create_session ${createSessSig ? explorer(createSessSig) : "(token already existed)"} token=${tokenPda.toBase58()}\n` +
        `  place_bet_chips_session ${explorer(sessionBetSig)} (session signer ${session.publicKey.toBase58()}, ${BET_AMOUNT} on face ${BET_SYMBOL})`;
      note(4, "PASS", sessionBetDetail + "\n  (ER delegation of the session token and SessionCap is still UNVERIFIED: this check ran on Solana, as the PR asked first.)");
    } else {
      record("place_bet_chips", await send("place_bet_chips", base, [wallet], buildIx(idl, "place_bet_chips", {
        table: TABLE, round: ROUND, bet, chips, bettor: wallet.publicKey, system_program: SystemProgram.programId,
      }, Buffer.concat([u8(BET_SYMBOL), u64(BET_AMOUNT)]))));
    }

    r = await currentRound();
    const waitMs = Number(r.openedAt + 90n) * 1000 - Date.now() + 3_000;
    if (waitMs > 0) {
      console.log(`  waiting ${Math.ceil(waitMs / 1000)}s for the 90s window to close (opened_at=${r.openedAt})`);
      await sleep(waitMs);
    }

    rollSig = record("request_roll", await send("request_roll (base-layer DEFAULT_QUEUE)", base, [wallet], buildIx(idl, "request_roll", {
      table: TABLE, round: ROUND, payer: wallet.publicKey, oracle_queue: DEFAULT_QUEUE,
      program_identity: PROGRAM_IDENTITY, vrf_program: VRF_PROGRAM, slot_hashes: SLOT_HASHES,
      system_program: SystemProgram.programId,
    })));

    const deadline = Date.now() + CALLBACK_MS;
    let rolled = null;
    while (Date.now() < deadline) {
      const cur = await currentRound();
      if (cur.status === 3) { rolled = cur; break; }
      await sleep(3000);
    }
    if (!rolled) throw new Error("VRF callback did not land on Solana before the timeout");
    lastFaces = rolled.faces;
    const k = matches(rolled.faces, BET_SYMBOL);
    const due = owedFor(BET_AMOUNT, rolled.faces, BET_SYMBOL);
    console.log(`  Rolled faces [${rolled.faces.join(", ")}]; matches on ${BET_SYMBOL}: ${k}; owed=${due}`);

    const found = await findCallbackSig(base, ROUND);
    if (found) {
      callback = found;
      console.log(`  VRF callback: ${explorer(found.signature)}`);
      console.log(`  proof: request=${found.proof.request} faces=[${found.proof.faces.join(", ")}]`);
    } else {
      console.log("  VRF callback: not found in the round's recent Solana signatures");
    }

    const stBefore = await poolState();
    const settle = buildIx(idl, "settle_chip_bets", {
      table: TABLE, round: ROUND, pool: POOL, pool_vault: POOL_VAULT, cage_vault: CAGE,
      token_program: TOKEN_PROGRAM, queue: QUEUE,
    });
    settle.keys.push({ pubkey: bet, isSigner: false, isWritable: true });
    settle.keys.push({ pubkey: chips, isSigner: false, isWritable: true });
    settleSig = record("settle_chip_bets", await send("settle_chip_bets", base, [wallet], settle));

    const q = decodeQueue((await info(QUEUE)).data);
    console.log(`  queue length=${q.length} entries=${JSON.stringify(q.entries, (_, v) => typeof v === "bigint" ? v.toString() : v)}`);
    const stAfter = await poolState();
    console.log(`  pool after settle: vault=${stAfter.vault} supply=${stAfter.supply}`);

    if (due > 0n && q.length > 0n) {
      queued = { due, faces: rolled.faces, k, settleSig, roundId };
      break;
    }
    if (due > 0n && q.length === 0n) {
      console.log("  the win was paid immediately (pool was not short); emptying the pool and trying again");
    } else {
      console.log("  no win this round; finishing and retrying");
    }
    record("finish_round", await send("finish_round", base, [wallet], buildIx(idl, "finish_round", { round: ROUND })));
  }

  if (queued) {
    // Fund the pool enough to pay the head, then crank pay_queue.
    const st = await poolState();
    const shortfall = queued.due > st.vault ? queued.due - st.vault : 0n;
    const depositAmt = shortfall + 1_000_000n;
    console.log(`  depositing ${depositAmt} to cover queued ${queued.due} (vault ${st.vault})`);
    const have = tokenAmount((await info(musdAta)).data);
    if (have < depositAmt) {
      record("musd_faucet", await send("musd_faucet", base, [wallet], buildIx(idl, "musd_faucet", {
        config: CONFIG, musd_mint: MUSD_MINT, musd_mint_authority: MUSD_AUTH,
        faucet_record: P([enc("faucet"), wallet.publicKey.toBytes()]),
        wallet: wallet.publicKey, destination: musdAta, associated_token_program: ATA_PROGRAM,
        token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
      })));
    }
    const chipsBefore = decodeChips((await info(chips)).data).balance;
    record("deposit", await send("deposit (to pay the queue)", base, [wallet], buildIx(idl, "deposit", {
      pool: POOL, config: CONFIG, share_mint: SHARE_MINT, vault: POOL_VAULT, depositor: wallet.publicKey, depositor_ata: musdAta,
      depositor_shares: shareAta, insurance_vault: INSURANCE_VAULT, queue: QUEUE, token_program: TOKEN_PROGRAM,
      system_program: SystemProgram.programId, associated_token_program: ATA_PROGRAM,
    }, u64(depositAmt))));
    const pay = buildIx(idl, "pay_queue", {
      queue: QUEUE, pool: POOL, pool_vault: POOL_VAULT, token_program: TOKEN_PROGRAM,
    });
    pay.keys.push({ pubkey: chips, isSigner: false, isWritable: true });
    pay.keys.push({ pubkey: CAGE, isSigner: false, isWritable: true });
    paySig = record("pay_queue", await send("pay_queue", base, [wallet], pay));
    const qAfter = decodeQueue((await info(QUEUE)).data);
    const chipsAfter = decodeChips((await info(chips)).data).balance;
    record("finish_round", await send("finish_round", base, [wallet], buildIx(idl, "finish_round", { round: ROUND })));
    const credited = chipsAfter - chipsBefore;
    note(3, qAfter.length === 0n && credited === queued.due ? "PASS" : "FAIL",
      `short-pool settle queued a ${queued.due} chip win (${queued.k} matches on face ${BET_SYMBOL}, faces [${queued.faces.join(", ")}]).\n` +
      `  settle_chip_bets ${explorer(queued.settleSig)}\n` +
      `  pay_queue ${explorer(paySig)} credited ${credited} chips (queue length now ${qAfter.length})`);
  } else if (sessionBetSig) {
    note(3, "FAIL", `placed the session bet and settled ${MAX_QUEUE_TRIES} rounds but never queued a chip win (1-match pays 0; P(2+ matches) is about 26%). Last faces: ${lastFaces ? lastFaces.join(", ") : "none"}.`);
  } else {
    note(3, "FAIL", "did not reach a short-pool settle");
    if (!sessionBetSig) note(4, "FAIL", "place_bet_chips_session did not run");
  }

  if (callback) {
    note(6, "PASS",
      `VRF callback log carries the request and the faces.\n` +
      `  ${explorer(callback.signature)}\n` +
      `  request=${callback.proof.request} faces=[${callback.proof.faces.join(", ")}]\n` +
      `  This callback was on Solana (DEFAULT_QUEUE). The ER/validator Explorer link is checked by scripts/devnet-er-round.mjs.`);
  } else {
    note(6, rollSig ? "FAIL" : "FAIL",
      `request_roll ${rollSig ? explorer(rollSig) : "did not run"}, but the callback was not found in the round's recent signatures, or did not contain the proof log.`);
  }

  console.log("\n======= SUMMARY =======");
  for (const r of results) console.log(`CHECK ${r.check}: ${r.status}`);
  console.log("\nTransactions:");
  for (const s of sigs) console.log(`  ${s.name}: ${explorer(s.sig)}`);
  const failed = results.filter((r) => r.status === "FAIL");
  if (failed.length) process.exitCode = 1;
}

main().catch((err) => {
  console.error(`\nDevnet checks for PR #112 failed: ${err.message ?? err}`);
  process.exitCode = 1;
});
