pub mod constants;
pub mod error;
pub mod instructions;
pub mod payout;
pub mod state;

use anchor_lang::prelude::*;
// `#[ephemeral]` adds `process_undelegation`, the callback the delegation program calls on
// Solana when a delegated account is undelegated (notes.md §2, SDK 0.18.0).
// UNVERIFIED on devnet: the callback path is covered only by LiteSVM (no real delegation program).
use ephemeral_rollups_sdk::anchor::ephemeral;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH");

#[ephemeral]
#[program]
pub mod jhandi_munda {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handler(ctx)
    }

    pub fn musd_faucet(ctx: Context<MusdFaucet>) -> Result<()> {
        faucet::handler(ctx)
    }

    pub fn initialize_pool(ctx: Context<InitializePool>) -> Result<()> {
        pool_init::handler(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        pool_deposit::handler(ctx, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        pool_withdraw::handler(ctx, amount)
    }

    pub fn initialize_table(ctx: Context<InitializeTable>) -> Result<()> {
        table_init::handler(ctx)
    }

    /// Creates the versioned round PDA (D41) on an existing table. Table authority only.
    pub fn init_round(ctx: Context<InitRound>) -> Result<()> {
        table_init::handle_init_round(ctx)
    }

    /// #50: anyone takes today's (UTC) snapshot of the house. One per day.
    pub fn take_snapshot(ctx: Context<TakeSnapshot>, day: u64) -> Result<()> {
        snapshot::handle_take_snapshot(ctx, day)
    }

    pub fn place_bet(ctx: Context<PlaceBet>, symbol: u8, amount: u64) -> Result<()> {
        place_bet::handler(ctx, symbol, amount)
    }

    pub fn request_roll(ctx: Context<RequestRoll>) -> Result<()> {
        request_roll::handler(ctx)
    }

    pub fn open_claim(ctx: Context<OpenClaim>) -> Result<()> {
        claim_open::handle_open_claim(ctx)
    }

    pub fn init_insurance(ctx: Context<InitInsurance>) -> Result<()> {
        claim_redeem::handle_init_insurance(ctx)
    }

    pub fn init_queue(ctx: Context<InitQueue>) -> Result<()> {
        payout_queue::handle_init_queue(ctx)
    }

    pub fn pay_queue<'info>(ctx: Context<'info, PayQueue<'info>>) -> Result<()> {
        payout_queue::handle_pay_queue(ctx)
    }

    pub fn init_staking(ctx: Context<InitStaking>) -> Result<()> {
        staking::handle_init_staking(ctx)
    }

    pub fn stake(ctx: Context<StakeMunda>, amount: u64) -> Result<()> {
        staking::handle_stake(ctx, amount)
    }

    pub fn unstake(ctx: Context<UnstakeMunda>, amount: u64) -> Result<()> {
        staking::handle_unstake(ctx, amount)
    }

    pub fn set_transfers_enabled(ctx: Context<SetTransfersEnabled>, enabled: bool) -> Result<()> {
        staking::handle_set_transfers_enabled(ctx, enabled)
    }

    pub fn init_fee_vaults(ctx: Context<InitFeeVaults>) -> Result<()> {
        fees::handle_init_fee_vaults(ctx)
    }

    pub fn distribute_fees<'info>(ctx: Context<'info, DistributeFees<'info>>) -> Result<()> {
        fees::handle_distribute_fees(ctx)
    }

    pub fn distribute_overflow<'info>(ctx: Context<'info, DistributeOverflow<'info>>) -> Result<()> {
        fees::handle_distribute_overflow(ctx)
    }

    pub fn claim_stake_rewards(ctx: Context<ClaimStakeRewards>) -> Result<()> {
        fees::handle_claim_stake_rewards(ctx)
    }

    pub fn delegate_round<'info>(ctx: Context<'info, DelegateRound<'info>>) -> Result<()> {
        er_delegate::handle_delegate_round(ctx)
    }

    pub fn buy_chips(ctx: Context<BuyChips>, amount: u64) -> Result<()> {
        chips::handle_buy_chips(ctx, amount)
    }

    pub fn migrate_config<'info>(ctx: Context<'info, MigrateConfig<'info>>) -> Result<()> {
        migrate::handle_migrate_config(ctx)
    }

    pub fn migrate_table<'info>(ctx: Context<'info, MigrateTable<'info>>) -> Result<()> {
        migrate::handle_migrate_table(ctx)
    }

    pub fn migrate_round<'info>(ctx: Context<'info, MigrateRound<'info>>) -> Result<()> {
        migrate::handle_migrate_round(ctx)
    }

    pub fn undelegate_round<'info>(ctx: Context<'info, UndelegateRound<'info>>) -> Result<()> {
        undelegate::handle_undelegate_round(ctx)
    }

    pub fn undelegate_chips<'info>(ctx: Context<'info, UndelegateChips<'info>>, owner: Pubkey) -> Result<()> {
        undelegate::handle_undelegate_chips(ctx, owner)
    }

    pub fn init_cage(ctx: Context<InitCage>) -> Result<()> {
        chips::handle_init_cage(ctx)
    }

    pub fn settle_chip_bets<'info>(ctx: Context<'info, SettleChipBets<'info>>) -> Result<()> {
        settle::handle_settle_chip_bets(ctx)
    }

    pub fn cash_out(ctx: Context<CashOut>, amount: u64) -> Result<()> {
        chips::handle_cash_out(ctx, amount)
    }

    pub fn place_bet_chips(ctx: Context<PlaceBetChips>, symbol: u8, amount: u64) -> Result<()> {
        chips::handle_place_bet_chips(ctx, symbol, amount)
    }

    /// #46: the wallet grants a session signer a spend cap, for at most 60 minutes.
    pub fn grant_session(ctx: Context<GrantSession>, session_signer: Pubkey, cap: u64, valid_for_secs: i64) -> Result<()> {
        chips::handle_grant_session(ctx, session_signer, cap, valid_for_secs)
    }

    /// #46: a session signer places a chip bet within the wallet's cap and expiry.
    pub fn place_bet_chips_session(ctx: Context<PlaceBetChipsSession>, symbol: u8, amount: u64) -> Result<()> {
        chips::handle_place_bet_chips_session(ctx, symbol, amount)
    }

    pub fn delegate_chip_stack<'info>(ctx: Context<'info, DelegateChipStack<'info>>) -> Result<()> {
        chips::handle_delegate_chip_stack(ctx)
    }

    /// D42: opens an empty chip bet slip on Solana (the bettor pays the rent here).
    pub fn open_chip_bet(ctx: Context<OpenChipBet>, round_id: u64, symbol: u8) -> Result<()> {
        chips::handle_open_chip_bet(ctx, round_id, symbol)
    }

    /// D42: delegates the bettor's own slip to the rollup, with the round.
    pub fn delegate_bet<'info>(ctx: Context<'info, DelegateBet<'info>>, round_id: u64, symbol: u8) -> Result<()> {
        chips::handle_delegate_bet(ctx, round_id, symbol)
    }

    /// D42: brings a chip bet slip home from the rollup (bettor or table authority).
    pub fn undelegate_bet<'info>(
        ctx: Context<'info, UndelegateBet<'info>>,
        round_id: u64,
        player: Pubkey,
        symbol: u8,
    ) -> Result<()> {
        undelegate::handle_undelegate_bet(ctx, round_id, player, symbol)
    }

    pub fn init_munda(ctx: Context<InitMunda>) -> Result<()> {
        claim_redeem::handle_init_munda(ctx)
    }

    pub fn redeem_wait(ctx: Context<RedeemClaim>) -> Result<()> {
        claim_redeem::handle_redeem_wait(ctx)
    }

    pub fn redeem_take_now(ctx: Context<RedeemClaim>) -> Result<()> {
        claim_redeem::handle_redeem_take_now(ctx)
    }

    pub fn collect_insurance(ctx: Context<CollectInsurance>) -> Result<()> {
        claim_redeem::handle_collect_insurance(ctx)
    }

    pub fn settle_bets(ctx: Context<SettleBets>) -> Result<()> {
        settle::handle_settle_bets(ctx)
    }

    pub fn finish_round(ctx: Context<FinishRound>) -> Result<()> {
        settle::handle_finish_round(ctx)
    }

    pub fn callback_roll_dice(
        ctx: Context<CallbackRollDice>,
        randomness: [u8; 32],
        request_id: [u8; 32],
    ) -> Result<()> {
        callback_roll::handler(ctx, randomness, request_id)
    }
}
