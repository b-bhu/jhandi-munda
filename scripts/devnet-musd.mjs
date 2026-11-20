#!/usr/bin/env node
/**
 * Devnet MUSD glue for the browser-free path (issue #4, D21, D26).
 *
 *   node scripts/devnet-musd.mjs init   [keypair.json] [rpc]
 *   node scripts/devnet-musd.mjs faucet [keypair.json] [rpc]
 *
 * `init` runs the program's `initialize` once (writes the config PDA and the
 * MUSD mint PDA). `faucet` calls `musd_faucet` from a local wallet and prints
 * the mint address plus Explorer (?cluster=devnet) links for the mint and the
 * faucet transaction — exactly the proof the PR asks for.
 *
 * Devnet only: the RPC argument is refused if it mentions mainnet. The
 * keypair file is a gitignored wallet (keys/… or ~/.config/solana/…); the
 * secret is read into memory and never printed or logged.
 *
 * @solana/web3.js is resolved from app/node_modules (same pinned dependency
 * the UI uses), so the repo keeps a single lockfile for JS packages.
 */

import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const web3 = createRequire(new URL("../app/package.json", import.meta.url))("@solana/web3.js");
const {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} = web3;

/** Program id — matches `declare_id!` in programs/jhandi_munda/src/lib.rs. */
const PROGRAM_ID = new PublicKey("4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH");
/** PDA seeds and fixed program ids — matches constants.rs. */
const seed = (s) => new TextEncoder().encode(s);
const CONFIG_PDA = PublicKey.findProgramAddressSync([seed("config")], PROGRAM_ID)[0];
const MUSD_MINT_PDA = PublicKey.findProgramAddressSync([seed("musd_mint")], PROGRAM_ID)[0];
const MUSD_MINT_AUTHORITY_PDA = PublicKey.findProgramAddressSync(
  [seed("musd_mint_authority")],
  PROGRAM_ID,
)[0];
/** The wallet's faucet cooldown record (D33), seeded ["faucet", wallet]. */
function faucetRecordPda(wallet) {
  return PublicKey.findProgramAddressSync([seed("faucet"), wallet.toBytes()], PROGRAM_ID)[0];
}
const ATA_PROGRAM_ID = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const SPL_TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/** Same derivation as MusdFaucet::musd_ata in the program. */
function musdAta(wallet) {
  return PublicKey.findProgramAddressSync(
    [wallet.toBytes(), SPL_TOKEN_PROGRAM_ID.toBytes(), MUSD_MINT_PDA.toBytes()],
    ATA_PROGRAM_ID,
  )[0];
}

/**
 * Anchor 1.0.2 instruction discriminators: sha256("global:<ix> first 8 bytes.
 * Neither instruction takes arguments, so the data is the discriminator only.
 */
const INIT_DATA = Buffer.from([0xaf, 0xaf, 0x6d, 0x1f, 0x0d, 0x98, 0x9b, 0xed]);
const FAUCET_DATA = Buffer.from([0x2e, 0xc9, 0x3b, 0x58, 0xbd, 0x65, 0x51, 0x03]);

const FAUCET_AMOUNT_TEXT = "100 MUSD";
const DEFAULT_RPC = "https://api.devnet.solana.com";
const DEFAULT_KEYPAIR = "keys/devnet-wallet.json";
const explorer = (path) => `https://explorer.solana.com/${path}?cluster=devnet`;

async function readMusdBalance(connection, wallet) {
  const accounts = await connection.getParsedTokenAccountsByOwner(wallet, {
    mint: MUSD_MINT_PDA,
  });
  let total = 0n;
  for (const { account } of accounts.value) {
    total += BigInt(account.data.parsed?.info?.tokenAmount?.amount ?? 0);
  }
  return total;
}

const musdText = (base) => (Number(base) / 1_000_000).toLocaleString("en-US", { maximumFractionDigits: 6 });

function loadWallet(keyPath) {
  if (!existsSync(resolve(keyPath))) {
    throw new Error(
      `No keypair at ${keyPath}. Create one with: solana-keygen new -o ${keyPath} (gitignored, never committed)`,
    );
  }
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(readFileSync(keyPath, "utf8"))));
}

// Account orders mirror the Rust account structs exactly:
// initialize: config(mut), musd_mint(mut), musd_mint_authority, payer(mut sign),
//             token_program, system_program
// musd_faucet: config, musd_mint(mut), musd_mint_authority, wallet(mut sign),
//             faucet_record(mut), destination(mut), associated_token_program,
//             token_program, system_program
function initializeIx(payer) {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: CONFIG_PDA, isSigner: false, isWritable: true },
      { pubkey: MUSD_MINT_PDA, isSigner: false, isWritable: true },
      { pubkey: MUSD_MINT_AUTHORITY_PDA, isSigner: false, isWritable: false },
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      { pubkey: SPL_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: INIT_DATA,
  });
}

function faucetIx(wallet) {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: CONFIG_PDA, isSigner: false, isWritable: false },
      { pubkey: MUSD_MINT_PDA, isSigner: false, isWritable: true },
      { pubkey: MUSD_MINT_AUTHORITY_PDA, isSigner: false, isWritable: false },
      { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
      { pubkey: faucetRecordPda(wallet.publicKey), isSigner: false, isWritable: true },
      { pubkey: musdAta(wallet.publicKey), isSigner: false, isWritable: true },
      { pubkey: ATA_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SPL_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: FAUCET_DATA,
  });
}

async function main() {
  const [command, keyPath = DEFAULT_KEYPAIR, rpc = DEFAULT_RPC] = process.argv.slice(2);
  if (command !== "init" && command !== "faucet") {
    throw new Error("Usage: node scripts/devnet-musd.mjs <init|faucet> [keypair.json] [rpc]");
  }
  if (/mainnet/i.test(rpc)) {
    throw new Error(`Refusing RPC "${rpc}": this script is devnet only.`);
  }
  const wallet = loadWallet(keyPath);
  const connection = new Connection(rpc, "confirmed");

  console.log(`RPC: ${rpc}`);
  console.log(`Wallet: ${wallet.publicKey.toBase58()}`);
  console.log(`MUSD mint: ${MUSD_MINT_PDA.toBase58()} ${explorer(`address/${MUSD_MINT_PDA.toBase58()}`)}`);
  console.log(`MUSD ATA:  ${musdAta(wallet.publicKey).toBase58()}`);

  const configExists = (await connection.getAccountInfo(CONFIG_PDA)) !== null;
  if (command === "init") {
    if (configExists) {
      console.log("Already initialized: config PDA exists, nothing to do.");
      return;
    }
    const signature = await sendAndConfirmTransaction(
      connection,
      new Transaction().add(initializeIx(wallet)),
      [wallet],
    );
    console.log(`initialize signature: ${signature} ${explorer(`tx/${signature}`)}`);
    return;
  }

  if (!configExists) {
    throw new Error("Program not initialized on this cluster yet. Run: node scripts/devnet-musd.mjs init");
  }
  const before = await readMusdBalance(connection, wallet.publicKey);
  const signature = await sendAndConfirmTransaction(
    connection,
    new Transaction().add(faucetIx(wallet)),
    [wallet],
  );
  const after = await readMusdBalance(connection, wallet.publicKey);
  console.log(`MUSD balance: ${musdText(before)} -> ${musdText(after)} (+${FAUCET_AMOUNT_TEXT})`);
  console.log(`Faucet transaction: ${explorer(`tx/${signature}`)}`);
}

main().catch((e) => {
  console.error(e instanceof Error ? e.message : e);
  process.exitCode = 1;
});
