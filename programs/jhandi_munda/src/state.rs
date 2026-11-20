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

/// Table settings (D3, D6, D8, D25), created once by `initialize_table`. The
/// round clock and bet limits live here so later steps read one place.
#[account]
#[derive(InitSpace)]
pub struct TableConfig {
    /// Round length in seconds, start to finish (D8): 90.
    pub round_length_secs: i64,
    /// Minimum bet in MUSD base units (D3): $1 = 1_000_000.
    pub min_bet: u64,
    /// Maximum bet in MUSD base units. Only enforced when `max_bet_enabled`.
    pub max_bet: u64,
    /// Off by default on devnet (D6). A cap that exists but is switched off.
    pub max_bet_enabled: bool,
    /// Bump seed for the table config PDA.
    pub bump: u8,
}

/// Where the table's one round is in its life. Only `Idle` (no round running)
/// and `Open` (a round is running) are used now; later steps add the rest.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundStatus {
    /// No round running; any player's first bet will open one (D5).
    Idle,
    /// A round is running and its 90 s timer has started (D31).
    Open,
}

/// The table's round account (D5, D25): one per table, reused round after
/// round. Created vacant (`Idle`) by `initialize_table`.
#[account]
#[derive(InitSpace)]
pub struct Round {
    /// Current round status.
    pub status: RoundStatus,
    /// Unix timestamp the round opened (Clock, not slots). 0 while idle.
    pub opened_at: i64,
    /// Round counter: starts at 0, settle bumps it later. Part of every bet's
    /// seeds, so each round's bets are separate accounts.
    pub round_id: u64,
    /// Distinct players in this round, at most 10 (D25). The count is
    /// `players.len()`.
    #[max_len(10)]
    pub players: Vec<Pubkey>,
    /// Bump seed for the round PDA.
    pub bump: u8,
}

/// One player's bet in the table's current round (D3, D31). Created by
/// `place_bet`, which also opens the round when it is idle.
#[account]
#[derive(InitSpace)]
pub struct Bet {
    /// The wallet that placed the bet.
    pub player: Pubkey,
    /// Stake in MUSD base units.
    pub amount: u64,
    /// The die face (1 to 6) this bet is on.
    pub symbol: u8,
    /// Bump seed for the bet PDA.
    pub bump: u8,
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
