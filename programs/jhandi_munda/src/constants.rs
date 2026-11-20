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

/// Bet record PDA seed: `["bet", round_id, player, symbol]`, one account per
/// player per die face per round.
pub const BET_SEED: &[u8] = b"bet";

/// Soft cap on distinct players per round (D25). Matches `#[max_len]` on `Round::players`.
pub const MAX_PLAYERS: usize = 10;

/// Claim PDA seed: `["claim", bet_pubkey]`, one claim per settled bet (D13).
pub const CLAIM_SEED: &[u8] = b"claim";

/// Win "Take now" cut: 500 BPS, so the player gets 95% now (D7).
pub const WIN_EARLY_EXIT_BPS: u16 = 500;

/// Insurance fund authority PDA seed. Signs the "Take now" payouts (D7).
pub const INSURANCE_SEED: &[u8] = b"insurance";

/// Insurance fund MUSD vault PDA seed: a token account owned by the insurance PDA.
pub const INSURANCE_VAULT_SEED: &[u8] = b"insurance_vault";

/// MUNDA mint PDA seed (D22, placeholder name). Minted only on a loss-claim
/// redeem, never up front (D26).
pub const MUNDA_MINT_SEED: &[u8] = b"munda_mint";

/// MUNDA mint authority PDA seed: only the program signs MUNDA mints.
pub const MUNDA_MINT_AUTHORITY_SEED: &[u8] = b"munda_mint_authority";

/// MUNDA uses 6 decimals, like MUSD, so 1 MUNDA = 1_000_000 base units and one
/// base unit of a loss mints one base unit of MUNDA (D26).
pub const MUNDA_DECIMALS: u8 = 6;

/// A loss claim's "Wait" unlocks 24 hours after settle (D23).
pub const LOSS_UNLOCK_SECS: i64 = 24 * 60 * 60;

/// A loss claim's "Take now" tax: 1000 BPS, so 90% now and 10% never minted (D23).
pub const LOSS_EARLY_EXIT_BPS: u16 = 1_000;

/// Payout queue PDA seed (D4, D30): one queue for the whole table, on Solana.
pub const QUEUE_SEED: &[u8] = b"payout_queue";

/// Queue capacity in entries: 100 unpaid wins. At 40 bytes an entry the account
/// is about 4 KB, well under the 10,240-byte ER commit cap.
pub const QUEUE_CAPACITY: usize = 100;

/// Stake pool PDA seed (D15): totals for every MUNDA staker.
pub const STAKE_POOL_SEED: &[u8] = b"stake_pool";

/// Stake account PDA seed: ["stake", owner]. One per wallet. Stakers are separate
/// from pool depositors (D27).
pub const STAKE_SEED: &[u8] = b"stake";

/// Staked MUNDA vault PDA seed. Its authority is the stake authority PDA.
pub const STAKE_VAULT_SEED: &[u8] = b"stake_vault";

/// Stake authority PDA seed: signs unstakes out of the stake vault.
pub const STAKE_AUTHORITY_SEED: &[u8] = b"stake_authority";

/// Stake lots per wallet (Step 10.2): each has its own clock. Unstaking frees lots.
pub const STAKE_MAX_LOTS: usize = 4;

/// Stake lock (D29): a stake earns fee rewards only after it has sat this long.
/// Unstaking is allowed at any time; only rewards wait.
pub const STAKE_LOCK_SECS: i64 = 24 * 60 * 60;

/// Dev treasury vault PDA seed (D15: 10% of each win fee).
pub const DEV_VAULT_SEED: &[u8] = b"dev_vault";

/// Treasury vault PDA seed: the 20% fee slice once insurance is full (D18).
pub const TREASURY_VAULT_SEED: &[u8] = b"treasury_vault";

/// Treasury authority PDA seed: owns the dev and treasury vaults.
pub const TREASURY_AUTHORITY_SEED: &[u8] = b"treasury_authority";

/// Insurance target (D18), a demo number: 1,000 MUSD. Until the insurance vault
/// holds this, the 20% slice fills it. After that, the 20% slice goes to treasury.
pub const INSURANCE_TARGET: u64 = 1_000_000_000;

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
