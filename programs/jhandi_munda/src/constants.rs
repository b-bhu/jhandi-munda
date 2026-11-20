use anchor_lang::prelude::*;

/// Program config PDA seed. One config account per program.
pub const CONFIG_SEED: &[u8] = b"config";

/// MUSD mint PDA seed. The mint account is a PDA of this program, owned by the
/// SPL token program, so its address is fixed for a given program deployment.
pub const MUSD_MINT_SEED: &[u8] = b"musd_mint";

/// MUSD mint authority PDA seed. No keypair for this exists anywhere; only this
/// program can sign as it (never print or commit a mint authority keypair).
pub const MUSD_MINT_AUTHORITY_SEED: &[u8] = b"musd_mint_authority";

/// MUSD (Munda USD) uses 6 decimals, the same as USDC, so 1 MUSD = 1_000_000
/// base units and all amounts read like USDC amounts. It is our $1 stand-in on
/// devnet (D21, D26). No real USDC and no faucet in this step.
pub const MUSD_DECIMALS: u8 = 6;

/// SPL Token (non-2022) program id.
pub const SPL_TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Bytes reserved for an SPL token mint account (spl Mint::LEN).
pub const MUSD_MINT_SPACE: usize = 82;
