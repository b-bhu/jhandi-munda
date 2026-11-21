//! The FIFO payout queue (D4, D30; #29, #30, #31).
//!
//! When the pool cannot pay a win, the win joins the back of the queue instead of
//! failing (`enqueue`, called from `redeem_wait`). `pay_queue` pays the head
//! first, and only the head: as soon as the pool cannot cover the head it stops,
//! so a newer winner is never paid ahead of an older one (no skipping, D30).
//! Anyone can call `pay_queue` when MUSD has flowed into the pool.
//!
//! #110: a chip win that the pool cannot pay joins the same queue, keyed by the
//! winner's ChipStack instead of a Claim. `pay_queue` tells the two apart by the
//! account's type: a ChipStack is paid from the pool vault into the cage vault and
//! credited to the chips; a Claim is paid to the player's MUSD ATA.
//! LIMITATION: a chip entry is paid only while its ChipStack is on Solana (not
//! delegated), so a queued chip win waits, and holds the queue, until then. UNVERIFIED on devnet.
//!
//! The queue lives on Solana (base layer). It is a single zero-copy account of
//! about 4 KB, under the 10,240-byte ER commit cap.

use anchor_lang::prelude::*;

use crate::{
    error::ErrorCode,
    instructions::claim_redeem::{spl_transfer, token_balance},
    instructions::settle::save_account,
    state::{Claim, ChipStack, PayoutQueue, PoolState, QueueEntry},
    CAGE_VAULT_SEED, POOL_SEED, POOL_VAULT_SEED, QUEUE_CAPACITY, QUEUE_SEED, SPL_TOKEN_PROGRAM_ID,
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
    // then that claim's player MUSD ATA (writable). For a queued chip win the pair is
    // the ChipStack (writable), then the cage vault (writable).
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
        // Copy the packed fields out: references to them are not allowed.
        let (entry_claim, entry_amount) = (entry.claim, entry.amount);
        // Strict FIFO: if the head cannot be paid in full, stop here.
        if token_balance(&ctx.accounts.pool_vault)? < entry_amount {
            break;
        }
        let Some(pair) = pairs.next() else { break };
        let (claim_info, ata_info) = (&pair[0], &pair[1]);
        require_keys_eq!(claim_info.key(), entry_claim, ErrorCode::QueueOrderMismatch);

        // #110: a chip win. The entry's key is the ChipStack; pay the cage, credit the chips.
        let is_chips = {
            let data = claim_info.try_borrow_data()?;
            ChipStack::try_deserialize(&mut &data[..]).is_ok()
        };
        if is_chips {
            require_keys_eq!(*claim_info.owner, crate::ID, ErrorCode::QueueOrderMismatch);
            require!(claim_info.is_writable, ErrorCode::QueueOrderMismatch);
            let cage_expected = Pubkey::find_program_address(&[CAGE_VAULT_SEED], &crate::ID).0;
            require_keys_eq!(ata_info.key(), cage_expected, ErrorCode::QueueOrderMismatch);
            require!(ata_info.is_writable, ErrorCode::QueueOrderMismatch);
            let mut stack = {
                let data = claim_info.try_borrow_data()?;
                ChipStack::try_deserialize(&mut &data[..])?
            };
            let seeds: &[&[u8]] = &[POOL_SEED, &[pool.bump]];
            let (vault_info, pool_info) = (ctx.accounts.pool_vault.to_account_info(), pool.to_account_info());
            spl_transfer(&vault_info, ata_info, &pool_info, entry_amount, &[seeds])?;
            stack.balance = stack.balance.checked_add(entry_amount).ok_or(ErrorCode::MathOverflow)?;
            save_account(claim_info, &stack)?;
            queue.head += 1;
            msg!("Queued chip win of {} credited to chips; {} left in queue", entry_amount, queue.tail - queue.head);
            continue;
        }

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
        spl_transfer(&vault_info, &ata_owned, &pool_info, entry_amount, &[seeds])?;
        queue.head += 1;
        msg!("Queue paid {} base MUSD; {} left in queue", entry_amount, queue.tail - queue.head);
    }
    Ok(())
}
