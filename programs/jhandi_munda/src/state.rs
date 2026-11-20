use anchor_lang::prelude::*;

/// Program-wide settings, first written by `initialize`. Later steps (pool,
/// table, faucet) read or extend this account.
#[account]
#[derive(InitSpace)]
pub struct ProgramConfig {
    /// The MUSD mint this program controls.
    pub musd_mint: Pubkey,
    /// Bump seed for the config PDA itself.
    pub bump: u8,
    /// Bump seed for the MUSD mint authority PDA.
    pub musd_bump: u8,
}

/// The house pool (D4, D27), created empty by `initialize_pool`: a zero MUSD
/// vault and a zero share supply. Later steps (deposit/withdraw) move MUSD in
/// and out of the vault and mint/burn pool shares against the NAV, so this
/// account is the one every other instruction settles against.
#[account]
#[derive(InitSpace)]
pub struct PoolState {
    /// The Step 2 MUSD mint this pool owns money in (reused, never re-created).
    pub musd_mint: Pubkey,
    /// The pool share (LP) mint (D27). Its authority is this pool PDA, so only
    /// pool instructions can mint or burn shares.
    pub share_mint: Pubkey,
    /// The vault token account holding pool MUSD; owned by this pool PDA.
    pub vault: Pubkey,
    /// Bump seed for this pool PDA. The pool PDA is the vault's authority and
    /// the share mint authority, so it signs every pool CPI.
    pub bump: u8,
    /// Bump seed for the vault PDA.
    pub vault_bump: u8,
    /// Bump seed for the share mint PDA.
    pub share_mint_bump: u8,
}

/// NAV of one pool share (D27), reported as MUSD base units per share scaled
/// x1_000_000 for precision: a $1.00 NAV reads as exactly 1_000_000.
///
/// `nav_per_share(0, 0)` -> 1_000_000: the empty pool's NAV is *defined* as
/// $1.00 (D4 starts the pool unseeded), so the first deposits mint shares
/// 1:1 with MUSD and later deposits price off the real vault/supply ratio.
pub fn nav_per_share(vault_amount: u64, share_supply: u64) -> u64 {
    if share_supply == 0 {
        // Empty pool: NAV is $1.00 by definition.
        return 1_000_000;
    }
    // NAV = vault / shares, scaled x1_000_000. The multiply runs in u128: in
    // plain u64 it already overflows once the *whole vault* passes ~18.4M
    // MUSD (u64::MAX / 1_000_000 base units), whatever the share count. Only
    // then could the final value also miss u64, so it saturates. The deposit
    // step should use checked math for pricing shares anyway.
    let nav = (vault_amount as u128 * 1_000_000) / share_supply as u128;
    u64::try_from(nav).unwrap_or(u64::MAX)
}

/// The MUSD faucet cooldown record for one wallet (D33): a small PDA at
/// `["faucet", user_pubkey]`, created by the wallet's first faucet call, that
/// stores when that wallet last claimed. One record per wallet, so every
/// wallet's cooldown is independent.
#[account]
#[derive(InitSpace)]
pub struct FaucetRecord {
    /// `Clock` `unix_timestamp` of the wallet's last successful claim.
    pub last_claim_time: i64,
    /// Bump seed for the record PDA itself.
    pub bump: u8,
}
