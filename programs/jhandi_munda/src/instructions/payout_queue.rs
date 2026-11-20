//! The FIFO payout queue (D4, D30; #29, #30).
//!
//! When the pool cannot pay a win, the win joins the back of the queue instead of
//! failing (`enqueue`, called from `redeem_wait`). Paying the head comes in 8.3.
//!
//! The queue lives on Solana (base layer). It is a single zero-copy account of
//! about 4 KB, under the 10,240-byte ER commit cap.

use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    state::{PayoutQueue, QueueEntry},
    QUEUE_CAPACITY, QUEUE_SEED,
};

/// Adds a win to the back of the queue. Fails with `QueueFull` at capacity.
pub fn enqueue(queue: &mut PayoutQueue, claim: Pubkey, amount: u64) -> Result<()> {
    require!(queue.tail - queue.head < QUEUE_CAPACITY as u64, ErrorCode::QueueFull);
    let slot = (queue.tail % QUEUE_CAPACITY as u64) as usize;
    queue.entries[slot] = QueueEntry { claim, amount };
    queue.tail += 1;
    Ok(())
}

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
