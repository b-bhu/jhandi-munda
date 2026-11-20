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
