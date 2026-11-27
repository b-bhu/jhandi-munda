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
const FAUCET_BASE_UNITS = 100_000_000;
const FAUCET_TEXT = "100 MUSD";

/** Devnet only. The script at scripts/devnet-musd.mjs refuses mainnet URLs. */
const CONNECTION = new Connection("https://api.devnet.solana.com", "confirmed");
const explorer = (path: string) => `https://explorer.solana.com/${path}?cluster=devnet`;

/** Minimal wallet surface used here: connect + sign-and-send one transaction. */
type BrowserWallet = {
  connect: () => Promise<{ publicKey: PublicKey }>;
  signAndSendTransaction: (transaction: Transaction) => Promise<{ signature: string }>;
};

declare global {
  interface Window {
    phantom?: { solana?: BrowserWallet };
    solana?: BrowserWallet;
  }
}

function findWallet(): BrowserWallet | undefined {
  if (typeof window === "undefined") return undefined;
  const wallet = window.phantom?.solana ?? window.solana;
  if (!wallet?.connect || !wallet?.signAndSendTransaction) return undefined;
  return wallet;
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
        throw new Error("No devnet wallet found. Install Phantom, switch it to Solana devnet, then try again.");
      }
      const { publicKey } = await browserWallet.connect();
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

      setStatus("Confirming on devnet…");
      const confirmed = await CONNECTION.confirmTransaction(signature, "confirmed");
      if (confirmed.value.err) {
        throw new Error(`Faucet call failed onchain: ${JSON.stringify(confirmed.value.err)}`);
      }

      const after = await readMusdBalance(publicKey);
      setBalanceBase(after);
      setStatus(`Balance is now ${formatMusd(after)} MUSD (+${FAUCET_TEXT} per click).`);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="center">
      <h1>Get MUSD</h1>
      <p>
        Devnet test chips for the Jhandi Munda Casino. MUSD is our $1 test
        dollar (D21, D26) with no real value, minted on devnet only.
      </p>
      <p>
        The faucet mints {FAUCET_TEXT} ({FAUCET_BASE_UNITS.toString()} base units,
        6 decimals) to your wallet per click.
      </p>
      <p>
        Mint: <code>{MUSD_MINT.toBase58()}</code> —{" "}
        <a href={explorer(`address/${MUSD_MINT.toBase58()}`)}>open on Solana Explorer</a>
      </p>
      <Link href="/">← Back to the casino</Link>
      <p>
        <button onClick={getMusd} disabled={busy}>
          {busy ? "Working…" : "Get MUSD"}
        </button>
      </p>
      {wallet !== null && (
        <p>
          Wallet: <code>{wallet}</code>
        </p>
      )}
      {balanceBase !== null && <p>MUSD balance: {formatMusd(balanceBase)} MUSD</p>}
      {txSig !== null && (
        <p>
          Faucet transaction:{" "}
          <a href={explorer(`tx/${txSig}`)}>{txSig}</a>
        </p>
      )}
      {status !== null && <p>{status}</p>}
      {error !== null && <p role="alert">{error}</p>}
    </main>
  );
}
