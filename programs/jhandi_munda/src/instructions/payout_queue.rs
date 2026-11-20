//! The FIFO payout queue (D4, D30; #29, #30, #31).
//!
//! When the pool cannot pay a win, the win joins the back of the queue instead of
//! failing (`enqueue`, called from `redeem_wait`). `pay_queue` pays the head
//! first, and only the head: as soon as the pool cannot cover the head it stops,
//! so a newer winner is never paid ahead of an older one (no skipping, D30).
//! Anyone can call `pay_queue` when MUSD has flowed into the pool.
//!
//! The queue lives on Solana (base layer). It is a single zero-copy account of
//! about 4 KB, under the 10,240-byte ER commit cap.

use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    instructions::claim_redeem::{spl_transfer, token_balance},
    state::{Claim, PayoutQueue, PoolState, QueueEntry},
    POOL_SEED, POOL_VAULT_SEED, QUEUE_CAPACITY, QUEUE_SEED, SPL_TOKEN_PROGRAM_ID,
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

#[derive(Accounts)]
pub struct PayQueue<'info> {
    /// The payout queue. Its head is paid first, and only the head.
    #[account(mut, seeds = [QUEUE_SEED], bump)]
    pub queue: AccountLoader<'info, PayoutQueue>,

    /// The house pool: its vault pays the queue.
    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool MUSD vault, seeds pin it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump)]
    pub pool_vault: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,
    // Remaining accounts, in pairs, in queue order: each queued claim (writable),
    // then that claim's player MUSD ATA (writable).
}

/// Pays queued wins from the head while the pool can cover the head. Stops at
/// the first head it cannot cover, so nothing behind it is paid (D30).
pub fn handle_pay_queue<'info>(ctx: Context<'info, PayQueue<'info>>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    let mut queue = ctx.accounts.queue.load_mut()?;
    let pool = &ctx.accounts.pool;
    let mut pairs = ctx.remaining_accounts.chunks_exact(2);

    while queue.head < queue.tail {
        let entry = queue.entries[(queue.head % QUEUE_CAPACITY as u64) as usize];
        // Strict FIFO: if the head cannot be paid in full, stop here.
        if token_balance(&ctx.accounts.pool_vault)? < entry.amount {
            break;
        }
        let Some(pair) = pairs.next() else { break };
        let (claim_info, ata_info) = (&pair[0], &pair[1]);
        require_keys_eq!(claim_info.key(), entry.claim, ErrorCode::QueueOrderMismatch);

        let mut claim: Account<Claim> = Account::try_from(claim_info)?;
        require!(claim.queued && !claim.redeemed, ErrorCode::QueueOrderMismatch);
        require_keys_eq!(
            ata_info.key(),
            crate::musd_ata(&claim.player, &pool.musd_mint),
            ErrorCode::InvalidAtaAddress
        );
        claim.queued = false;
        claim.redeemed = true;
        claim.exit(&crate::ID)?;

        let seeds: &[&[u8]] = &[POOL_SEED, &[pool.bump]];
        let (vault_info, pool_info) = (ctx.accounts.pool_vault.to_account_info(), pool.to_account_info());
        let ata_owned = ata_info.clone();
        spl_transfer(&vault_info, &ata_owned, &pool_info, entry.amount, &[seeds])?;
        queue.head += 1;
        msg!("Queue paid {} base MUSD; {} left in queue", entry.amount, queue.tail - queue.head);
    }
    Ok(())
}
