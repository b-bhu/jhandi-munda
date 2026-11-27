"use client";

/*
 * Get MUSD — the devnet faucet button (issue #4, decisions D21, D26).
 *
 * One page, one button: click connects the browser wallet, then calls the
 * devnet `musd_faucet` instruction, which mints 100 MUSD to the caller's
 * canonical MUSD ATA (once per wallet per hour, the faucet cooldown, D33).
 * The mint address and an Explorer link (?cluster=devnet) are shown.
 * Devnet only — this never touches mainnet or real funds.
 */

import Link from "next/link";
import { useState } from "react";
import { Buffer } from "buffer";
import {
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { injected, NO_PHANTOM_MESSAGE } from "@/lib/wallet";

/** Program id — matches `declare_id!` in programs/jhandi_munda/src/lib.rs. */
const PROGRAM_ID = new PublicKey("4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH");

/** PDA seeds and fixed program ids — matches constants.rs. */
const seed = (s: string) => new TextEncoder().encode(s);
const CONFIG = PublicKey.findProgramAddressSync([seed("config")], PROGRAM_ID)[0];
const MUSD_MINT = PublicKey.findProgramAddressSync([seed("musd_mint")], PROGRAM_ID)[0];
const MUSD_MINT_AUTHORITY = PublicKey.findProgramAddressSync(
  [seed("musd_mint_authority")],
  PROGRAM_ID,
)[0];
/** The wallet's faucet cooldown record (D33), seeded ["faucet", wallet]. */
function faucetRecord(wallet: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([seed("faucet"), wallet.toBytes()], PROGRAM_ID)[0];
}
const SPL_TOKEN_PROGRAM = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ATA_PROGRAM = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const SYSTEM_PROGRAM = SystemProgram.programId;

/** The wallet's canonical MUSD ATA, same derivation as MusdFaucet::musd_ata. */
function musdAta(wallet: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [wallet.toBytes(), SPL_TOKEN_PROGRAM.toBytes(), MUSD_MINT.toBytes()],
    ATA_PROGRAM,
  )[0];
}

/**
 * Anchor 1.0.2 instruction discriminator for `musd_faucet`:
 * sha256("global:musd_faucet") first 8 bytes. The instruction takes no
 * arguments, so this is the whole payload.
 */
const MUSD_FAUCET_DATA = Buffer.from([0x2e, 0xc9, 0x3b, 0x58, 0xbd, 0x65, 0x51, 0x03]);

/** Documented faucet amount: 100 MUSD per call, 6 decimals like USDC (D33). */

/** Devnet only. The script at scripts/devnet-musd.mjs refuses mainnet URLs. */
const CONNECTION = new Connection("https://api.devnet.solana.com", "confirmed");
const explorer = (path: string) => `https://explorer.solana.com/${path}?cluster=devnet`;

/** Phantom-only: reuse the shared helper so a generic window.solana hijack cannot sign. */
function findWallet() {
  const wallet = injected();
  const signAndSendTransaction = wallet?.signAndSendTransaction;
  if (!wallet || !signAndSendTransaction) return undefined;
  return { connect: wallet.connect.bind(wallet), signAndSendTransaction };
}

/** Reads the wallet's MUSD balance across its token accounts, in base units. */
async function readMusdBalance(wallet: PublicKey): Promise<number> {
  const accounts = await CONNECTION.getParsedTokenAccountsByOwner(wallet, { mint: MUSD_MINT });
  let total = 0;
  for (const { account } of accounts.value) {
    const parsed = account.data as { parsed?: { info?: { tokenAmount?: { amount?: string } } } };
    total += Number(parsed.parsed?.info?.tokenAmount?.amount ?? 0);
  }
  return total;
}

/** Formats base units as a MUSD number string (6 decimals, like USDC). */
function formatMusd(base: number): string {
  return (base / 1_000_000).toLocaleString("en-US", { maximumFractionDigits: 6 });
}

export default function GetMusdPage() {
  const [wallet, setWallet] = useState<string | null>(null);
  const [balanceBase, setBalanceBase] = useState<number | null>(null);
  const [txSig, setTxSig] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  /** One button: connect (if needed), then call the faucet. */
  async function getMusd() {
    setError(null);
    setStatus(null);
    setBusy(true);
    try {
      const browserWallet = findWallet();
      if (!browserWallet) {
        throw new Error(NO_PHANTOM_MESSAGE);
      }
      const { publicKey: rawKey } = await browserWallet.connect();
      const publicKey = new PublicKey(rawKey.toString());
      setWallet(publicKey.toBase58());

      const instruction = new TransactionInstruction({
        programId: PROGRAM_ID,
        keys: [
          { pubkey: CONFIG, isSigner: false, isWritable: false },
          { pubkey: MUSD_MINT, isSigner: false, isWritable: true },
          { pubkey: MUSD_MINT_AUTHORITY, isSigner: false, isWritable: false },
          { pubkey: publicKey, isSigner: true, isWritable: true },
          { pubkey: faucetRecord(publicKey), isSigner: false, isWritable: true },
          { pubkey: musdAta(publicKey), isSigner: false, isWritable: true },
          { pubkey: ATA_PROGRAM, isSigner: false, isWritable: false },
          { pubkey: SPL_TOKEN_PROGRAM, isSigner: false, isWritable: false },
          { pubkey: SYSTEM_PROGRAM, isSigner: false, isWritable: false },
        ],
        data: MUSD_FAUCET_DATA,
      });

      const { signature } = await browserWallet.signAndSendTransaction(
        new Transaction().add(instruction),
      );
      setTxSig(signature);

      setStatus("Confirming…");
      const confirmed = await CONNECTION.confirmTransaction(signature, "confirmed");
      if (confirmed.value.err) {
        throw new Error("Couldn't add the chips. Please try again.");
      }

      const after = await readMusdBalance(publicKey);
      setBalanceBase(after);
      setStatus(`You now have ${formatMusd(after)} MUSD. Each click adds 100.`);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="center">
      <h1>Get free test chips</h1>
      <p>
        Grab 100 MUSD to play with. They&apos;re test-only and have no real value.
      </p>
      <p className="dim small">
        Details: mint <code>{MUSD_MINT.toBase58()}</code>
        {" — "}
        <a href={explorer(`address/${MUSD_MINT.toBase58()}`)}>open on Solana Explorer</a>
      </p>
      <Link href="/" className="backlink">← Back to the casino</Link>
      <p>
        <button className="btn" onClick={getMusd} disabled={busy}>
          {busy ? "Working…" : "Get MUSD"}
        </button>
      </p>
      {wallet !== null && (
        <p>
          Wallet: <code>{wallet}</code>
        </p>
      )}
      {balanceBase !== null && <p>Your balance: {formatMusd(balanceBase)} MUSD</p>}
      {txSig !== null && (
        <p>
          Transaction:{" "}
          <a href={explorer(`tx/${txSig}`)}>{txSig}</a>
        </p>
      )}
      {status !== null && <p>{status}</p>}
      {error !== null && <p role="alert">{error}</p>}
    </div>
  );
}
