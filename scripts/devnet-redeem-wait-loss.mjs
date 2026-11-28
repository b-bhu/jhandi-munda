#!/usr/bin/env node
// One-off: redeem round 13 loss claims by Wait (24h later), then check unstake is refused. Devnet only.
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


(async()=>{try{
const rid = 13n;
const hex = (d)=>d?Buffer.from(d).toString("hex"):null;
for (const [kind,s] of [["wallet loss",1],["chip loss",5]]) {
  const bet = betPda(rid, wallet.publicKey, s);
  const c = decodeClaim((await acct(claimPda(bet))).data);
  console.log(kind, bet.toBase58(), JSON.stringify(c,(_,x)=>typeof x==="bigint"?x.toString():x), "now", Math.floor(Date.now()/1000));
  if (c.redeemed) { console.log("already redeemed"); continue; }
  const v0 = await tokBal(STAKE_VAULT); const s0 = (await acct(STAKE))?.data;
  const res = await send("redeem_wait "+kind, ix("redeem_wait", redeemAccounts(bet)), {expectFail:true});
  const v1 = await tokBal(STAKE_VAULT); const s1 = (await acct(STAKE))?.data;
  console.log("  err", JSON.stringify(res.err), res.log, "vault delta", (v1-v0).toString(), "stake before", s0?hex(s0):"none", "after", hex(s1));
}
const SA = P([enc("stake_authority")]);
const r = await send("unstake 1 MUNDA", ix("unstake", {config: CONFIG, stake_pool: STAKE_POOL, stake: STAKE, stake_vault: STAKE_VAULT, stake_authority: SA, owner: wallet.publicKey, owner_munda_ata: ata(wallet.publicKey, MUNDA_MINT), token_program: TOKEN_PROGRAM}, u64(1000000)), {expectFail:true});
console.log("  unstake err", JSON.stringify(r.err), r.log);

}catch(e){console.error("FAILED:",e.message)}})();
