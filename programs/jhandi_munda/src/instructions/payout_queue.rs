//! The FIFO payout queue account (D4, D30; #29). The queue is created empty here.
//! Enqueueing a win the pool cannot pay (#30) and paying the head (#31) come in
//! the next steps.
//!
//! The queue lives on Solana (base layer). It is a single zero-copy account of
//! about 4 KB, under the 10,240-byte ER commit cap.

use anchor_lang::prelude::*;

use crate::{
    state::PayoutQueue,
    QUEUE_SEED,
};

#[derive(Accounts)]
pub struct InitQueue<'info> {
    /// The payout queue, created empty: head and tail both 0.
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<PayoutQueue>(),
        seeds = [QUEUE_SEED],
        bump
    )]
    pub queue: AccountLoader<'info, PayoutQueue>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_init_queue(ctx: Context<InitQueue>) -> Result<()> {
    let mut queue = ctx.accounts.queue.load_init()?;
    queue.head = 0;
    queue.tail = 0;
    msg!("Payout queue initialized (empty)");
    Ok(())
}
