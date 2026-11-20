pub mod constants;
pub mod error;
pub mod instructions;
pub mod payout;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH");

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
