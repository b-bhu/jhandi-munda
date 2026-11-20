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
