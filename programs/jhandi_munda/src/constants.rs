use anchor_lang::prelude::*;

/// Program config PDA seed. One config account per program.
pub const CONFIG_SEED: &[u8] = b"config";

/// MUSD mint PDA seed. The mint account is a PDA of this program, owned by the
/// SPL token program, so its address is fixed for a given program deployment.
pub const MUSD_MINT_SEED: &[u8] = b"musd_mint";

/// MUSD mint authority PDA seed. No keypair for this exists anywhere; only this
/// program can sign as it (never print or commit a mint authority keypair).
pub const MUSD_MINT_AUTHORITY_SEED: &[u8] = b"musd_mint_authority";

/// House pool (D4, D27) PDA seed. One pool per program: the house everything
/// else settles against. Created by `initialize_pool`, starts empty.
pub const POOL_SEED: &[u8] = b"pool";

/// MUSD vault token account PDA seed. The vault is a plain SPL token account
/// owned by the pool PDA (`initialize_account3` in the pool handler), so only
/// pool instructions ever move its MUSD.
pub const POOL_VAULT_SEED: &[u8] = b"pool_vault";

/// Pool share (LP) mint PDA seed (D27). Deposits mint shares, withdraws burn
/// them; its mint authority is the pool PDA, not the MUSD mint authority.
pub const SHARE_MINT_SEED: &[u8] = b"pool_share_mint";

/// Table config PDA seed. One table per program (D25).
pub const TABLE_SEED: &[u8] = b"table";

/// Round PDA seed. One round account per table, reused round after round (D5).
pub const ROUND_SEED: &[u8] = b"round";

/// Round length: 90 seconds start to finish (D8). Stored in `TableConfig`.
pub const ROUND_LENGTH_SECS: i64 = 90;

/// Minimum bet: $1 MUSD in base units (D3).
pub const MIN_BET: u64 = 1_000_000;

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

/// Bytes reserved for an SPL token mint account (spl Mint::LEN). Used by the
/// MUSD mint in `initialize` and by the pool-share (LP) mint, same layout.
/// (Kept under the MUSD name for continuity with the Step 2 scaffolding.)
pub const MUSD_MINT_SPACE: usize = 82;

/// Bytes reserved for an SPL token account (spl state::Account::LEN) — the
/// pool's MUSD vault.
pub const TOKEN_ACCOUNT_SPACE: usize = 165;
