use anchor_lang::prelude::*;

/// Program config PDA seed. One config account per program.
pub const CONFIG_SEED: &[u8] = b"config";

/// MUSD mint PDA seed. The mint account is a PDA of this program, owned by the
/// SPL token program, so its address is fixed for a given program deployment.
pub const MUSD_MINT_SEED: &[u8] = b"musd_mint";

/// MUSD mint authority PDA seed. No keypair for this exists anywhere; only this
/// program can sign as it (never print or commit a mint authority keypair).
pub const MUSD_MINT_AUTHORITY_SEED: &[u8] = b"musd_mint_authority";

/// Faucet cooldown record PDA, second seed: one record per wallet, so the
/// full derivation is `["faucet", wallet_pubkey]` (D33).
pub const FAUCET_RECORD_SEED: &[u8] = b"faucet";

/// Faucet cooldown: each wallet can claim at most once per hour (D33).
pub const FAUCET_COOLDOWN_SECS: i64 = 60 * 60;

/// MUSD (Munda USD) uses 6 decimals, the same as USDC, so 1 MUSD = 1_000_000
/// base units and all amounts read like USDC amounts. It is our $1 stand-in on
/// devnet (D21, D26). The `musd_faucet` instruction mints FAUCET_AMOUNT
/// base units per call.
pub const MUSD_DECIMALS: u8 = 6;

/// MUSD the faucet mints per call: 100 MUSD (100 x 1_000_000, 6 decimals).
/// No per-day cap, but one call per wallet per hour (D33); documented in
/// README.md (D21, D26).
pub const FAUCET_AMOUNT: u64 = 100_000_000;

/// Associated Token Account program (devnet and mainnet). The faucet CPIs
/// `create_idempotent` to it so the wallet's MUSD ATA exists before minting.
pub const ATA_PROGRAM_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// SPL Token (non-2022) program id.
pub const SPL_TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Bytes reserved for an SPL token mint account (spl Mint::LEN).
pub const MUSD_MINT_SPACE: usize = 82;
