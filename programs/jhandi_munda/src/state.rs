use anchor_lang::prelude::*;

use crate::{QUEUE_CAPACITY, STAKE_LOCK_SECS, STAKE_MAX_LOTS};

/// Program-wide settings, first written by `initialize`. Later steps (pool,
/// table, faucet) read or extend this account.
#[account]
#[derive(InitSpace)]
pub struct ProgramConfig {
    /// The table authority (D40): the only key that can delegate the round. Set
    /// to the payer of `initialize`.
    pub admin: Pubkey,
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
    /// Fee pots, in MUSD base units, accrued by `settle` (D15): the treasury
    /// share (20%), the devs share (10%) and the stakers share (70%).
    pub fee_treasury: u64,
    pub fee_devs: u64,
    pub fee_stakers: u64,
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
    /// The join window has closed and a VRF roll is pending (D11, Step 5.1).
    Rolling,
    /// The six faces are written; waiting for settle (Step 6).
    Rolled,
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
    /// The caller seed of the pending VRF request, which is its id. Zero when
    /// no roll is pending.
    pub request_id: [u8; 32],
    /// The six dice faces (1 to 6) of the last roll. Zero until a roll lands.
    pub faces: [u8; 6],
    /// Number of bets in this round. Settle must see exactly this many.
    pub bet_count: u16,
    /// Bets settled so far, across however many `settle_bets` calls.
    pub settled_count: u16,
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
    /// The round this bet belongs to. Its seeds include this too.
    pub round_id: u64,
    /// What settle owes this bet, after the 2% win fee. Zero for a loss.
    pub owed: u64,
    /// Set once settle has processed the bet, so it cannot be counted twice.
    pub settled: bool,
    /// Clock time the bet was settled. Claims unlock from here (D7, D8).
    pub settled_at: i64,
    /// Bump seed for the bet PDA.
    pub bump: u8,
}

/// Which asset a claim pays: MUSD for a win, MUNDA for a loss (D13).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimAsset {
    Musd,
    Munda,
}

/// One settled result turned into a claim (D13): what it pays, when it unlocks,
/// and the early-exit cut for "Take now". One claim per bet, at the PDA
/// `["claim", bet]`. Created by `open_claim`; redeemed in Step 7.2 and 7.3.
#[account]
#[derive(InitSpace)]
pub struct Claim {
    /// The bet this claim settles.
    pub bet: Pubkey,
    /// The wallet the claim pays.
    pub player: Pubkey,
    /// What it pays: MUSD for a win, MUNDA for a loss.
    pub asset: ClaimAsset,
    /// The amount in the asset's base units: owed MUSD for a win.
    pub amount: u64,
    /// Clock time from which "Wait" pays in full: settled_at + 90 s for a win (D7, D8).
    pub unlock_at: i64,
    /// The "Take now" cut in basis points (D7: 500 = 5% for a win).
    pub early_exit_bps: u16,
    /// Set once the claim is paid out, by Wait or by Take now (one redeem only).
    pub redeemed: bool,
    /// True while a win waits in the payout queue for the pool to fund it (D4).
    pub queued: bool,
    /// After a "Take now" win: the full amount the insurance fund advanced at
    /// 95%, which the pool owes back in full (D7). Zero once collected.
    pub insurance_owed: u64,
    /// Bump seed for the claim PDA.
    pub bump: u8,
}

/// One unpaid win waiting in the payout queue: the claim, and what it is owed.
/// Plain fixed-size data (40 bytes, no padding), so it can be stored zero-copy.
#[zero_copy(unsafe)]
pub struct QueueEntry {
    pub claim: Pubkey,
    pub amount: u64,
}

/// FIFO payout queue (D4, D30). A ring buffer: `head` is the oldest entry and
/// `tail` the next free slot, both counted up forever and taken modulo
/// `QUEUE_CAPACITY`. Length is `tail - head`. It is zero-copy because it is
/// too big to deserialize onto the BPF stack.
#[account(zero_copy(unsafe))]
#[repr(C)]
pub struct PayoutQueue {
    pub head: u64,
    pub tail: u64,
    pub entries: [QueueEntry; QUEUE_CAPACITY],
}

/// Pool-wide MUNDA staking totals (D15). `stakers` counts every stake account
/// ever created, so the fee crank can check it was given all of them.
#[account]
#[derive(InitSpace)]
pub struct StakePool {
    /// MUNDA staked across all wallets, in base units.
    pub total_staked: u64,
    /// Number of stake accounts created.
    pub stakers: u32,
    /// Bump seed for the stake pool PDA.
    pub bump: u8,
}

/// One stake lot (D29, Step 10.2): an amount staked at one time. Each lot has its
/// own 24-hour clock from `staked_at`. An empty lot has `amount` 0.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct StakeLot {
    /// MUNDA in this lot, in base units.
    pub amount: u64,
    /// Clock time this lot was staked.
    pub staked_at: i64,
}

/// One wallet's MUNDA stake (D15, D27, D29). Holding MUNDA alone earns nothing:
/// only staked lots that have matured take part in the 70% fee slice.
#[account]
#[derive(InitSpace)]
pub struct Stake {
    /// The wallet that staked.
    pub owner: Pubkey,
    /// Total MUNDA staked across the lots, in base units.
    pub amount: u64,
    /// The stake lots. Each has its own 24-hour clock (up to `STAKE_MAX_LOTS`).
    pub lots: [StakeLot; STAKE_MAX_LOTS],
    /// Fee rewards credited to this stake, in MUSD base units, not yet claimed.
    pub pending: u64,
    /// Bump seed for the stake PDA.
    pub bump: u8,
}

impl Stake {
    /// MUNDA that earns rewards at `now`: each lot counts once it has been staked
    /// for `STAKE_LOCK_SECS` (D29). Younger lots count for nothing yet.
    pub fn eligible(&self, now: i64) -> u64 {
        self.lots
            .iter()
            .filter(|lot| lot.amount > 0 && now >= lot.staked_at + STAKE_LOCK_SECS)
            .map(|lot| lot.amount)
            .sum()
    }
}

/// A player's chips at the cage (D39): MUSD bought on Solana and spent on the
/// rollup without token transfers. One per wallet, at `["chips", owner]`.
#[account]
#[derive(InitSpace)]
pub struct ChipStack {
    /// The wallet the chips belong to.
    pub owner: Pubkey,
    /// Chips held, in MUSD base units.
    pub balance: u64,
    /// Bump seed for the chip stack PDA.
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
