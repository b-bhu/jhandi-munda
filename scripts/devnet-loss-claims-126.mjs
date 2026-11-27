#!/usr/bin/env node
/**
 * Devnet proof for #124 / #126 (D50, D51): loss claims open MUNDA claims for BOTH a wallet (MUSD)
 * bet and a chip bet, with the 5x early-loser bonus, and Take now on a loss claim is refused
 * (LossClaimWaitOnly, 6065). Also a wallet win claim redeemed with Take now. Devnet only.
 *
 *   node scripts/devnet-loss-claims-126.mjs [keypair.json] [devnetRpc]
 *
 * Each round: wallet bets $1 on faces 1-3 (MUSD), chip bets $1 on faces 4-6. Six faces, six dice, so
 * at least three faces lose. Retries a new round until it has seen a wallet loss, a chip loss and a
 * wallet win. The keypair secret is never printed. Redeem by Wait after 24h is not covered here.
 */
import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const web3 = createRequire(new URL("../app/package.json", import.meta.url))("@solana/web3.js");
const { Connection, Keypair, PublicKey, SystemProgram, Transaction, TransactionInstruction } = web3;

const RPC = process.argv[3] ?? "https://api.devnet.solana.com";
if (/mainnet/i.test(RPC)) throw new Error("devnet only");
const PROGRAM_ID = new PublicKey("4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH");
const TOKEN_PROGRAM = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ATA_PROGRAM = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const DEFAULT_QUEUE = new PublicKey("Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh");
const VRF_PROGRAM = new PublicKey("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz");
const SLOT_HASHES = new PublicKey("SysvarS1otHashes111111111111111111111111111");
const ONE = 1_000_000n;
const EARLY_CAP = 1_000_000_000n;
const enc = (s) => new TextEncoder().encode(s);
const P = (seeds) => PublicKey.findProgramAddressSync(seeds, PROGRAM_ID)[0];
const [CONFIG, POOL, POOL_VAULT, SHARE_MINT, INSURANCE_VAULT, QUEUE, CAGE, MUSD_MINT, TABLE, ROUND, IDENTITY,
  MUNDA_MINT, MUNDA_AUTH, STAKE_POOL, STAKE_VAULT, INS_AUTH, LOSS_TALLY] =
  ["config", "pool", "pool_vault", "pool_share_mint", "insurance_vault", "payout_queue", "cage_vault", "musd_mint",
    "table", "round_v2", "identity", "munda_mint", "munda_mint_authority", "stake_pool", "stake_vault", "insurance",
    "loss_tally"].map((s) => P([enc(s)]));
const u64 = (n) => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; };
const ata = (o, m) => PublicKey.findProgramAddressSync([o.toBytes(), TOKEN_PROGRAM.toBytes(), m.toBytes()], ATA_PROGRAM)[0];
const betPda = (rid, who, sym) => P([enc("bet"), u64(rid), who.toBytes(), Uint8Array.of(sym)]);
const claimPda = (bet) => P([enc("claim"), bet.toBytes()]);
const explorer = (s) => `https://explorer.solana.com/tx/${s}?cluster=devnet`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const wallet = Keypair.fromSecretKey(new Uint8Array(JSON.parse(readFileSync(resolve(process.argv[2] ?? "keys/dev-wallet.json"), "utf8"))));
const idl = JSON.parse(readFileSync(resolve("target/idl/jhandi_munda.json"), "utf8"));
const conn = new Connection(RPC, "confirmed");
const musdAta = ata(wallet.publicKey, MUSD_MINT);
const shareAta = ata(wallet.publicKey, SHARE_MINT);
const chips = P([enc("chips"), wallet.publicKey.toBytes()]);
const STAKE = P([enc("stake"), wallet.publicKey.toBytes()]);

function ix(name, accounts, args = Buffer.alloc(0)) {
  const def = idl.instructions.find((i) => i.name === name);
  const keys = def.accounts.map((a) => {
    const key = accounts[a.name] ?? (a.address ? new PublicKey(a.address) : undefined);
    if (!key) throw new Error(`${name}: missing ${a.name}`);
    return { pubkey: key, isSigner: !!a.signer, isWritable: !!a.writable };
  });
  return new TransactionInstruction({ programId: PROGRAM_ID, keys, data: Buffer.concat([Buffer.from(def.discriminator), args]) });
}
const sigs = [];
async function send(label, instruction, { expectFail = false } = {}) {
  const tx = new Transaction().add(instruction);
  tx.feePayer = wallet.publicKey;
  tx.recentBlockhash = (await conn.getLatestBlockhash()).blockhash;
  tx.sign(wallet);
  const sig = await conn.sendRawTransaction(tx.serialize(), { skipPreflight: expectFail });
  const res = await conn.confirmTransaction(sig, "confirmed");
  if (!expectFail && res.value.err) throw new Error(`${label} failed: ${JSON.stringify(res.value.err)}`);
  sigs.push({ label, sig });
  console.log(`  ${label}: ${explorer(sig)}`);
  if (expectFail) {
    const t = await conn.getTransaction(sig, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });
    const log = (t?.meta?.logMessages ?? []).find((l) => /Error Code|custom program error/.test(l)) ?? "";
    console.log(`    on-chain result: err=${JSON.stringify(t?.meta?.err)}  ${log.trim()}`);
    return { sig, err: t?.meta?.err, log };
  }
  return sig;
}
const acct = async (pk) => conn.getAccountInfo(pk);
const tokBal = async (pk) => { const a = await acct(pk); return a ? a.data.readBigUInt64LE(64) : 0n; };
const tally = async () => { const a = await acct(LOSS_TALLY); return a ? a.data.readBigUInt64LE(8) : 0n; };
const lossMunda = (before, stake) => { const room = before > EARLY_CAP ? 0n : EARLY_CAP - before; const inside = stake < room ? stake : room; return inside * 5n + (stake - inside); };
function decodeRound(d) {
  let o = 8; const status = d[o++]; const openedAt = d.readBigInt64LE(o); o += 8; const roundId = d.readBigUInt64LE(o); o += 8;
  const n = d.readUInt32LE(o); o += 4 + n * 32 + 32; return { status, openedAt, roundId, faces: [...d.subarray(o, o + 6)] };
}
const decodeBet = (d) => ({ amount: d.readBigUInt64LE(40), owed: d.readBigUInt64LE(57), settled: d[65] === 1, chip: d[74] === 1 });
const decodeClaim = (d) => ({ asset: d[72] === 0 ? "MUSD" : "MUNDA", amount: d.readBigUInt64LE(73), unlockAt: d.readBigInt64LE(81), earlyBps: d.readUInt16LE(89), redeemed: d[91] === 1 });
const redeemAccounts = (bet) => ({
  bet, claim: claimPda(bet), player: wallet.publicKey, pool: POOL, queue: QUEUE, pool_vault: POOL_VAULT,
  insurance_vault: INSURANCE_VAULT, insurance_authority: INS_AUTH, player_ata: musdAta, munda_mint: MUNDA_MINT,
  munda_mint_authority: MUNDA_AUTH, stake_pool: STAKE_POOL, stake: STAKE, stake_vault: STAKE_VAULT, token_program: TOKEN_PROGRAM,
});
const roundNow = async () => decodeRound((await acct(ROUND)).data);

async function main() {
  console.log(`Wallet ${wallet.publicKey.toBase58()}  (secret not printed)  RPC ${RPC}`);
  const proven = { walletLoss: null, chipLoss: null, walletWin: null, takeNowRefused: [] };
  const musd = await tokBal(musdAta);
  const vault = await tokBal(POOL_VAULT);
  const chipAcc = await acct(chips);
  const chipBal = chipAcc ? chipAcc.data.readBigUInt64LE(40) : 0n;
  console.log(`MUSD ${musd}  pool vault ${vault}  chips ${chipBal}  insurance ${await tokBal(INSURANCE_VAULT)}  loss_tally ${await tally()}`);
  if (vault < 40n * ONE) {
    const amt = 40n * ONE - vault + ONE;
    console.log(`topping the pool up with ${amt} test MUSD (deposit)`);
    await send("deposit", ix("deposit", {
      pool: POOL, config: CONFIG, share_mint: SHARE_MINT, vault: POOL_VAULT, depositor: wallet.publicKey, depositor_ata: musdAta,
      depositor_shares: shareAta, insurance_vault: INSURANCE_VAULT, queue: QUEUE, token_program: TOKEN_PROGRAM,
      system_program: SystemProgram.programId, associated_token_program: ATA_PROGRAM,
    }, u64(amt)));
  }
  if (chipBal < 12n * ONE) {
    await send("buy_chips", ix("buy_chips", {
      pool: POOL, cage_vault: CAGE, chips, owner: wallet.publicKey, owner_musd_ata: musdAta,
      token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId,
    }, u64(12n * ONE - chipBal)));
  }

  for (let attempt = 1; attempt <= 4; attempt++) {
    let r = await roundNow();
    if (r.status !== 0) throw new Error(`round status ${r.status}, expected Idle`);
    const rid = r.roundId;
    console.log(`\n=== Attempt ${attempt}: round ${rid} ===`);
    const chipSyms = [4, 5, 6], walletSyms = [1, 2, 3];
    for (const s of chipSyms) {
      await send(`open_chip_bet face ${s}`, ix("open_chip_bet", { round: ROUND, bet: betPda(rid, wallet.publicKey, s), bettor: wallet.publicKey, system_program: SystemProgram.programId }, Buffer.concat([u64(rid), Buffer.from([s])])));
    }
    for (const s of chipSyms) {
      await send(`place_bet_chips face ${s} $1`, ix("place_bet_chips", { table: TABLE, round: ROUND, bet: betPda(rid, wallet.publicKey, s), chips, bettor: wallet.publicKey, system_program: SystemProgram.programId }, Buffer.concat([Buffer.from([s]), u64(ONE)])));
    }
    for (const s of walletSyms) {
      await send(`place_bet (MUSD) face ${s} $1`, ix("place_bet", { table: TABLE, round: ROUND, bet: betPda(rid, wallet.publicKey, s), pool: POOL, vault: POOL_VAULT, bettor: wallet.publicKey, bettor_ata: musdAta, token_program: TOKEN_PROGRAM, system_program: SystemProgram.programId }, Buffer.concat([Buffer.from([s]), u64(ONE)])));
    }
    r = await roundNow();
    const wait = Number(r.openedAt + 90n) * 1000 - Date.now() + 3000;
    if (wait > 0) { console.log(`  waiting ${Math.ceil(wait / 1000)}s for the 90 s window`); await sleep(wait); }
    await send("request_roll", ix("request_roll", { table: TABLE, round: ROUND, payer: wallet.publicKey, oracle_queue: DEFAULT_QUEUE, program_identity: IDENTITY, vrf_program: VRF_PROGRAM, slot_hashes: SLOT_HASHES }));
    const deadline = Date.now() + 4 * 60 * 1000;
    let rolled = null;
    while (Date.now() < deadline) { const c = await roundNow(); if (c.status === 3) { rolled = c; break; } await sleep(3000); }
    if (!rolled) throw new Error("VRF callback timed out");
    console.log(`  faces [${rolled.faces.join(", ")}]`);
    const sc = ix("settle_chip_bets", { table: TABLE, round: ROUND, pool: POOL, pool_vault: POOL_VAULT, cage_vault: CAGE, token_program: TOKEN_PROGRAM, queue: QUEUE });
    for (const s of chipSyms) { sc.keys.push({ pubkey: betPda(rid, wallet.publicKey, s), isSigner: false, isWritable: true }); sc.keys.push({ pubkey: chips, isSigner: false, isWritable: true }); }
    await send("settle_chip_bets", sc);
    const sb = ix("settle_bets", { table: TABLE, round: ROUND });
    for (const s of walletSyms) sb.keys.push({ pubkey: betPda(rid, wallet.publicKey, s), isSigner: false, isWritable: true });
    await send("settle_bets", sb);
    await send("finish_round", ix("finish_round", { round: ROUND }));

    for (const s of [...walletSyms, ...chipSyms]) {
      const bet = betPda(rid, wallet.publicKey, s);
      const b = decodeBet((await acct(bet)).data);
      const kind = b.chip ? "chip" : "wallet";
      const k = rolled.faces.filter((f) => f === s).length;
      console.log(`  ${kind} bet face ${s}: ${k} match(es), owed ${b.owed}`);
      if (b.chip && b.owed > 0n) {
        const res = await send(`open_claim on a WINNING chip bet (expect refusal)`, ix("open_claim", { bet, claim: claimPda(bet), loss_tally: LOSS_TALLY, payer: wallet.publicKey, system_program: SystemProgram.programId }), { expectFail: true });
        console.log(`    ${res.err ? "refused as expected (ChipBetSettledToChips)" : "UNEXPECTED: claim opened"}`);
        continue;
      }
      if (!b.chip && b.owed > 0n && proven.walletWin) continue;
      if (!b.chip && b.owed === 0n && proven.walletLoss) continue;
      if (b.chip && b.owed === 0n && proven.chipLoss) continue;
      const before = await tally();
      await send(`open_claim (${kind} ${b.owed > 0n ? "WIN" : "LOSS"}, face ${s})`, ix("open_claim", { bet, claim: claimPda(bet), loss_tally: LOSS_TALLY, payer: wallet.publicKey, system_program: SystemProgram.programId }));
      const c = decodeClaim((await acct(claimPda(bet))).data);
      const after = await tally();
      if (b.owed === 0n) {
        const exp = lossMunda(before, b.amount);
        const ok = c.asset === "MUNDA" && c.amount === exp && c.earlyBps === 0;
        console.log(`    ${kind} LOSS claim: ${c.asset} ${c.amount} (expected ${exp} = 5 x ${b.amount} within the first $1,000; tally ${before} -> ${after}); unlock_at ${c.unlockAt} early_exit_bps ${c.earlyBps}  ${ok ? "OK" : "MISMATCH"}`);
        const rec = { face: s, claim: c, expected: exp, ok, bet: bet.toBase58(), tallyBefore: before, tallyAfter: after };
        if (b.chip) proven.chipLoss = rec; else proven.walletLoss = rec;
        const tn = await send(`redeem_take_now on the ${kind} LOSS claim (expect refusal)`, ix("redeem_take_now", redeemAccounts(bet)), { expectFail: true });
        proven.takeNowRefused.push({ kind, err: tn.err, log: tn.log, sig: tn.sig });
      } else {
        console.log(`    wallet WIN claim: ${c.asset} ${c.amount}, early_exit_bps ${c.earlyBps}, unlock_at ${c.unlockAt}`);
        const before95 = await tokBal(musdAta);
        const sig = await send("redeem_take_now on the WIN claim", ix("redeem_take_now", redeemAccounts(bet)));
        const got = (await tokBal(musdAta)) - before95;
        console.log(`    Take now paid ${got} of ${c.amount} (expected ${(c.amount * 9500n) / 10000n})`);
        proven.walletWin = { face: s, claim: c, got, sig };
      }
    }
    if (proven.walletLoss && proven.chipLoss && proven.walletWin) break;
    console.log("  not all three cases seen yet; another round");
  }
  const j = (v) => JSON.stringify(v, (_, x) => (typeof x === "bigint" ? x.toString() : x), 2);
  console.log("\n=== SUMMARY ===\n" + j(proven));
  console.log("\nTransactions:"); for (const s of sigs) console.log(`  ${s.label}: ${explorer(s.sig)}`);
}
main().catch((e) => { console.error("FAILED:", e.message ?? e); process.exitCode = 1; });
