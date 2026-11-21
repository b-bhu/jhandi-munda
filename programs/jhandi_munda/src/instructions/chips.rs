//! Chips at the cage (D39, D30; #43).
//!
//! Tokens cannot move on the rollup (docs/er-design.md). So a player buys chips
//! on Solana: their MUSD moves into the pool vault and a `ChipStack` records the
//! balance. A chip bet on the rollup only subtracts from that balance. Cashing
//! out goes the other way and is only possible on Solana: a delegated `ChipStack`
//! is owned by the delegation program, so the typed account check refuses it.
//!
//! The base-layer `place_bet` (token transfer) is unchanged and still works.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke,
};
use ephemeral_rollups_sdk::{
    cpi::{delegate_account, DelegateAccounts, DelegateConfig},
    pda::{DELEGATE_BUFFER_TAG, DELEGATION_METADATA_TAG, DELEGATION_RECORD_TAG},
};

use crate::{
    error::ErrorCode,
    instructions::{
        claim_redeem::{spl_transfer, token_balance},
        place_bet::{check_bet, record_bet},
    },
    state::{Bet, ChipStack, PayoutQueue, PoolState, ProgramConfig, Round, TableConfig},
    BET_SEED, CAGE_VAULT_SEED, CHIPS_SEED, CONFIG_SEED, ER_VALIDATOR_ASIA, MUSD_MINT_SEED, POOL_SEED,
    QUEUE_SEED, ROUND_SEED, SPL_TOKEN_PROGRAM_ID, TABLE_SEED, TOKEN_ACCOUNT_SPACE,
};

// --- buy_chips (Solana only) ---

#[derive(Accounts)]
pub struct BuyChips<'info> {
    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the cage vault (chip money, D39), seeds pin it. The bought MUSD lands
    /// here, not in the pool vault: chip money is not pool money.
    #[account(mut, seeds = [CAGE_VAULT_SEED], bump)]
    pub cage_vault: UncheckedAccount<'info>,

    /// The buyer's chip stack, created on the first purchase.
    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + ChipStack::INIT_SPACE,
        seeds = [CHIPS_SEED, owner.key().as_ref()],
        bump
    )]
    pub chips: Account<'info, ChipStack>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: the buyer's canonical MUSD ATA, checked in the handler.
    #[account(mut)]
    pub owner_musd_ata: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_buy_chips(ctx: Context<BuyChips>, amount: u64) -> Result<()> {
    require_keys_eq!(ctx.accounts.token_program.key(), SPL_TOKEN_PROGRAM_ID, ErrorCode::InvalidTokenProgram);
    require!(amount > 0, ErrorCode::ZeroChips);
    let owner = ctx.accounts.owner.key();
    require_keys_eq!(
        ctx.accounts.owner_musd_ata.key(),
        crate::musd_ata(&owner, &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );
    spl_transfer(
        &ctx.accounts.owner_musd_ata.to_account_info(),
        &ctx.accounts.cage_vault.to_account_info(),
        &ctx.accounts.owner.to_account_info(),
        amount,
        &[],
    )?;
    let chips = &mut ctx.accounts.chips;
    if chips.owner == Pubkey::default() {
        chips.owner = owner;
        chips.bump = ctx.bumps.chips;
    }
    chips.balance = chips.balance.checked_add(amount).ok_or(ErrorCode::MathOverflow)?;
    msg!("Bought {} base chips", amount);
    Ok(())
}

// --- cash_out (Solana only, queue must be empty: D30) ---

#[derive(Accounts)]
pub struct CashOut<'info> {
    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the cage vault (chip money), seeds pin it. Cash-outs pay from here.
    #[account(mut, seeds = [CAGE_VAULT_SEED], bump)]
    pub cage_vault: UncheckedAccount<'info>,

    /// The payout queue: cash-outs wait until every queued winner is paid (D30).
    #[account(seeds = [QUEUE_SEED], bump)]
    pub queue: AccountLoader<'info, PayoutQueue>,

    /// The chip stack. A delegated stack is owned by the delegation program, so
    /// this typed account refuses it: cash-out needs Solana (D39).
    #[account(mut, seeds = [CHIPS_SEED, owner.key().as_ref()], bump = chips.bump)]
    pub chips: Account<'info, ChipStack>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: the owner's canonical MUSD ATA, checked in the handler.
    #[account(mut)]
    pub owner_musd_ata: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,
}

pub fn handle_cash_out(ctx: Context<CashOut>, amount: u64) -> Result<()> {
    require_keys_eq!(ctx.accounts.token_program.key(), SPL_TOKEN_PROGRAM_ID, ErrorCode::InvalidTokenProgram);
    require!(amount > 0, ErrorCode::ZeroChips);
    {
        let queue = ctx.accounts.queue.load()?;
        require!(queue.head == queue.tail, ErrorCode::QueueNotEmpty);
    }
    require!(ctx.accounts.chips.balance >= amount, ErrorCode::InsufficientChips);
    let owner = ctx.accounts.owner.key();
    require_keys_eq!(
        ctx.accounts.owner_musd_ata.key(),
        crate::musd_ata(&owner, &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );
    require!(
        token_balance(&ctx.accounts.cage_vault.to_account_info())? >= amount,
        ErrorCode::InsufficientChips
    );
    ctx.accounts.chips.balance -= amount;
    let seeds: &[&[u8]] = &[POOL_SEED, &[ctx.accounts.pool.bump]];
    spl_transfer(
        &ctx.accounts.cage_vault.to_account_info(),
        &ctx.accounts.owner_musd_ata.to_account_info(),
        &ctx.accounts.pool.to_account_info(),
        amount,
        &[seeds],
    )?;
    msg!("Cashed out {} base chips to MUSD", amount);
    Ok(())
}

// --- place_bet_chips (runs on the rollup: no token transfer) ---

#[derive(Accounts)]
#[instruction(symbol: u8, amount: u64)]
pub struct PlaceBetChips<'info> {
    #[account(seeds = [TABLE_SEED], bump)]
    pub table: Account<'info, TableConfig>,

    /// The live round (delegated to the rollup in production).
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: Account<'info, Round>,

    /// The bettor's bet slip for this round and face, opened on Solana by `open_chip_bet`
    /// (D42). This instruction only fills it in; it never creates an account, because the
    /// rollup refuses to change a non-delegated fee payer's balance (InvalidAccountForFee).
    #[account(
        mut,
        seeds = [BET_SEED, round.round_id.to_le_bytes().as_ref(), bettor.key().as_ref(), &[symbol]],
        bump = bet.bump
    )]
    pub bet: Account<'info, Bet>,

    /// The bettor's chips: a subtraction here, not a token transfer (D39).
    #[account(mut, seeds = [CHIPS_SEED, bettor.key().as_ref()], bump = chips.bump)]
    pub chips: Account<'info, ChipStack>,

    #[account(mut)]
    pub bettor: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_place_bet_chips(ctx: Context<PlaceBetChips>, symbol: u8, amount: u64) -> Result<()> {
    let bettor = ctx.accounts.bettor.key();
    // The slip is opened empty (amount 0). A filled slip cannot be filled again (D42).
    require!(
        ctx.accounts.bet.amount == 0 && !ctx.accounts.bet.settled,
        ErrorCode::BetSlipAlreadyFilled
    );
    let now = Clock::get()?.unix_timestamp;
    let is_new_player = check_bet(&ctx.accounts.round, &ctx.accounts.table, &bettor, symbol, amount, now)?;
    require!(ctx.accounts.chips.balance >= amount, ErrorCode::InsufficientChips);
    ctx.accounts.chips.balance -= amount;
    let bump = ctx.accounts.bet.bump;
    record_bet(
        &mut ctx.accounts.round,
        &mut ctx.accounts.bet,
        bettor,
        symbol,
        amount,
        bump,
        is_new_player,
        now,
    );
    // A chip bet: its stake is in the cage until settle_chip_bets moves it (D39).
    ctx.accounts.bet.paid_with_chips = true;
    msg!("Chip bet of {} on face {}", amount, symbol);
    Ok(())
}

// --- delegate_chip_stack (the owner delegates their chips with the round) ---

#[derive(Accounts)]
pub struct DelegateChipStack<'info> {
    /// The owner: pays the delegation rent and must own the chips.
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: the chip stack to delegate. Validated as a ChipStack in the handler.
    #[account(mut, seeds = [CHIPS_SEED, owner.key().as_ref()], bump)]
    pub chips: UncheckedAccount<'info>,

    /// CHECK: the delegation buffer, created by the SDK. Seeds pin its address.
    #[account(
        mut,
        seeds = [DELEGATE_BUFFER_TAG, chips.key().as_ref()],
        bump,
        seeds::program = crate::ID
    )]
    pub buffer_chips: UncheckedAccount<'info>,

    /// CHECK: the delegation record, written by the delegation program.
    #[account(
        mut,
        seeds = [DELEGATION_RECORD_TAG, chips.key().as_ref()],
        bump,
        seeds::program = delegation_program.key()
    )]
    pub delegation_record_chips: UncheckedAccount<'info>,

    /// CHECK: the delegation metadata, written by the delegation program.
    #[account(
        mut,
        seeds = [DELEGATION_METADATA_TAG, chips.key().as_ref()],
        bump,
        seeds::program = delegation_program.key()
    )]
    pub delegation_metadata_chips: UncheckedAccount<'info>,

    /// CHECK: this program, the chips' owner before delegation.
    #[account(address = crate::ID)]
    pub owner_program: UncheckedAccount<'info>,

    /// CHECK: the delegation program, pinned to the SDK's id.
    #[account(address = ephemeral_rollups_sdk::id())]
    pub delegation_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_delegate_chip_stack<'info>(ctx: Context<'info, DelegateChipStack<'info>>) -> Result<()> {
    let owner = ctx.accounts.owner.key();
    {
        let chips_info = ctx.accounts.chips.to_account_info();
        let data = chips_info.try_borrow_data()?;
        let stack = ChipStack::try_deserialize(&mut &data[..])?;
        require_keys_eq!(stack.owner, owner, ErrorCode::NotChipOwner);
    }
    let config = DelegateConfig {
        validator: Some(ER_VALIDATOR_ASIA),
        ..Default::default()
    };
    // The SDK appends the bump itself, so pass the seeds without it.
    let seeds: &[&[u8]] = &[CHIPS_SEED, owner.as_ref()];
    let accounts = DelegateAccounts {
        payer: &ctx.accounts.owner.to_account_info(),
        pda: &ctx.accounts.chips.to_account_info(),
        owner_program: &ctx.accounts.owner_program.to_account_info(),
        buffer: &ctx.accounts.buffer_chips.to_account_info(),
        delegation_record: &ctx.accounts.delegation_record_chips.to_account_info(),
        delegation_metadata: &ctx.accounts.delegation_metadata_chips.to_account_info(),
        delegation_program: &ctx.accounts.delegation_program.to_account_info(),
        system_program: &ctx.accounts.system_program.to_account_info(),
    };
    delegate_account(accounts, seeds, config)?;
    msg!("Chip stack delegated to the Asia Ephemeral Rollup");
    Ok(())
}

// --- init_cage (once, after initialize_pool) ---

#[derive(Accounts)]
pub struct InitCage<'info> {
    #[account(seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, ProgramConfig>,

    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the cage vault, created here: a MUSD token account owned by the pool PDA.
    #[account(
        init,
        payer = payer,
        space = TOKEN_ACCOUNT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [CAGE_VAULT_SEED],
        bump
    )]
    pub cage_vault: UncheckedAccount<'info>,

    /// CHECK: the Step 2 MUSD mint, pinned by the config.
    #[account(
        seeds = [MUSD_MINT_SEED],
        bump,
        owner = SPL_TOKEN_PROGRAM_ID,
        constraint = config.musd_mint == musd_mint.key() @ ErrorCode::MusdMintMismatch
    )]
    pub musd_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_init_cage(ctx: Context<InitCage>) -> Result<()> {
    require_keys_eq!(ctx.accounts.token_program.key(), SPL_TOKEN_PROGRAM_ID, ErrorCode::InvalidTokenProgram);
    // The pool PDA owns the cage vault, the same way it owns the pool vault.
    let mut data = Vec::with_capacity(33);
    data.push(18u8); // InitializeAccount3
    data.extend_from_slice(ctx.accounts.pool.key().as_ref());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.cage_vault.key(), false),
            AccountMeta::new_readonly(ctx.accounts.musd_mint.key(), false),
        ],
        data,
    };
    invoke(
        &ix,
        &[ctx.accounts.cage_vault.to_account_info(), ctx.accounts.musd_mint.to_account_info()],
    )?;
    msg!("Cage vault initialized");
    Ok(())
}

// --- open_chip_bet and delegate_bet (D42): the slip is opened and delegated on Solana ---

/// The bettor opens an empty bet slip for `round_id` and `symbol` on Solana. The bettor
/// pays the rent here (Solana, not the rollup). `place_bet_chips` fills it on the rollup.
#[derive(Accounts)]
#[instruction(round_id: u64, symbol: u8)]
pub struct OpenChipBet<'info> {
    /// CHECK: the round account, pinned by its seeds. Its owner is this program or, while
    /// it is delegated, the delegation program (the Solana copy). The handler reads round_id
    /// from it, so a typed `Account<Round>` (which needs owner = this program) cannot be used.
    #[account(seeds = [ROUND_SEED], bump)]
    pub round: UncheckedAccount<'info>,

    /// The empty slip. Its seeds include the round id, the bettor and the face (D42).
    #[account(
        init,
        payer = bettor,
        space = 8 + Bet::INIT_SPACE,
        seeds = [BET_SEED, round_id.to_le_bytes().as_ref(), bettor.key().as_ref(), &[symbol]],
        bump
    )]
    pub bet: Account<'info, Bet>,

    /// The bettor: signs, and pays the slip's rent on Solana.
    #[account(mut)]
    pub bettor: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_open_chip_bet(ctx: Context<OpenChipBet>, round_id: u64, symbol: u8) -> Result<()> {
    require!((1..=6).contains(&symbol), ErrorCode::InvalidSymbol);
    let round_info = ctx.accounts.round.to_account_info();
    require!(
        *round_info.owner == crate::ID || *round_info.owner == crate::undelegate::delegation_program_id(),
        ErrorCode::AccountNotProgramOwned
    );
    let current = {
        let data = round_info.try_borrow_data()?;
        Round::try_deserialize(&mut &data[..])?.round_id
    };
    require!(current == round_id, ErrorCode::RoundIdMismatch);

    let bet = &mut ctx.accounts.bet;
    bet.player = ctx.accounts.bettor.key();
    bet.amount = 0; // empty until place_bet_chips fills it
    bet.symbol = symbol;
    bet.round_id = round_id;
    bet.owed = 0;
    bet.settled = false;
    bet.settled_at = 0;
    bet.paid_with_chips = true; // a chip slip: its stake is in the cage (D39)
    bet.bump = ctx.bumps.bet;
    msg!("Bet slip opened for round {} on face {}", round_id, symbol);
    Ok(())
}

/// Delegates the bettor's own slip to the Asia rollup, with the round. Only the slip's owner.
#[derive(Accounts)]
#[instruction(round_id: u64, symbol: u8)]
pub struct DelegateBet<'info> {
    /// The bettor: must own the slip; pays the delegation rent.
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: the slip to delegate. Validated as this bettor's Bet in the handler.
    #[account(mut, seeds = [BET_SEED, round_id.to_le_bytes().as_ref(), owner.key().as_ref(), &[symbol]], bump)]
    pub bet: UncheckedAccount<'info>,

    /// CHECK: the delegation buffer, created by the SDK. Seeds pin its address.
    #[account(
        mut,
        seeds = [DELEGATE_BUFFER_TAG, bet.key().as_ref()],
        bump,
        seeds::program = crate::ID
    )]
    pub buffer_bet: UncheckedAccount<'info>,

    /// CHECK: the delegation record, written by the delegation program.
    #[account(
        mut,
        seeds = [DELEGATION_RECORD_TAG, bet.key().as_ref()],
        bump,
        seeds::program = delegation_program.key()
    )]
    pub delegation_record_bet: UncheckedAccount<'info>,

    /// CHECK: the delegation metadata, written by the delegation program.
    #[account(
        mut,
        seeds = [DELEGATION_METADATA_TAG, bet.key().as_ref()],
        bump,
        seeds::program = delegation_program.key()
    )]
    pub delegation_metadata_bet: UncheckedAccount<'info>,

    /// CHECK: this program, the slip's owner before delegation.
    #[account(address = crate::ID)]
    pub owner_program: UncheckedAccount<'info>,

    /// CHECK: the delegation program, pinned to the SDK's id.
    #[account(address = ephemeral_rollups_sdk::id())]
    pub delegation_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_delegate_bet<'info>(ctx: Context<'info, DelegateBet<'info>>, round_id: u64, symbol: u8) -> Result<()> {
    let owner = ctx.accounts.owner.key();
    {
        let bet_info = ctx.accounts.bet.to_account_info();
        let data = bet_info.try_borrow_data()?;
        let bet = Bet::try_deserialize(&mut &data[..])?;
        require_keys_eq!(bet.player, owner, ErrorCode::NotBetOwner);
        require!(bet.round_id == round_id && bet.symbol == symbol, ErrorCode::BetAccountMismatch);
        require!(!bet.settled, ErrorCode::BetAlreadySettled);
    }
    let config = DelegateConfig {
        validator: Some(ER_VALIDATOR_ASIA),
        ..Default::default()
    };
    // The SDK appends the bump itself, so pass the seeds without it.
    let round_bytes = round_id.to_le_bytes();
    let symbol_bytes = [symbol];
    let seeds: &[&[u8]] = &[BET_SEED, &round_bytes, owner.as_ref(), &symbol_bytes];
    let accounts = DelegateAccounts {
        payer: &ctx.accounts.owner.to_account_info(),
        pda: &ctx.accounts.bet.to_account_info(),
        owner_program: &ctx.accounts.owner_program.to_account_info(),
        buffer: &ctx.accounts.buffer_bet.to_account_info(),
        delegation_record: &ctx.accounts.delegation_record_bet.to_account_info(),
        delegation_metadata: &ctx.accounts.delegation_metadata_bet.to_account_info(),
        delegation_program: &ctx.accounts.delegation_program.to_account_info(),
        system_program: &ctx.accounts.system_program.to_account_info(),
    };
    delegate_account(accounts, seeds, config)?;
    msg!("Bet slip delegated to the Asia Ephemeral Rollup");
    Ok(())
}
