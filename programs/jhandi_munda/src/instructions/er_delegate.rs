//! Delegating the live round to an Ephemeral Rollup (D20, #41).
//!
//! `delegate_round` hands the round account to a MagicBlock validator, so the
//! round's state runs on the rollup. The token-movement rule is in
//! docs/er-design.md: MUSD stays on Solana, and only round and dice state moves.
//! Nothing here commits or undelegates; those are later steps.
//!
//! The round is the only delegated account in this step. Its size is checked by
//! the tests against the 10,240-byte ER commit cap.
//!
//! This calls the SDK's `delegate_account` directly, with the PDA constraints the
//! `#[delegate]` attribute would generate written out here, so the account
//! mapping is explicit.

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::{
    cpi::{delegate_account, DelegateAccounts, DelegateConfig},
    pda::{DELEGATE_BUFFER_TAG, DELEGATION_METADATA_TAG, DELEGATION_RECORD_TAG},
};

use crate::{state::Round, ER_VALIDATOR_ASIA, ROUND_SEED};

#[derive(Accounts)]
pub struct DelegateRound<'info> {
    /// Pays the rent for the delegation buffer and record.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The live round, delegated to the Ephemeral Rollup. Untyped on purpose: it
    /// is owned by the delegation program after this call.
    /// CHECK: validated as a `Round` in the handler before delegation.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: UncheckedAccount<'info>,

    /// CHECK: the delegation buffer, created by the SDK. Seeds pin its address.
    #[account(
        mut,
        seeds = [DELEGATE_BUFFER_TAG, round.key().as_ref()],
        bump,
        seeds::program = crate::ID
    )]
    pub buffer_round: UncheckedAccount<'info>,

    /// CHECK: the delegation record, written by the delegation program.
    #[account(
        mut,
        seeds = [DELEGATION_RECORD_TAG, round.key().as_ref()],
        bump,
        seeds::program = delegation_program.key()
    )]
    pub delegation_record_round: UncheckedAccount<'info>,

    /// CHECK: the delegation metadata, written by the delegation program.
    #[account(
        mut,
        seeds = [DELEGATION_METADATA_TAG, round.key().as_ref()],
        bump,
        seeds::program = delegation_program.key()
    )]
    pub delegation_metadata_round: UncheckedAccount<'info>,

    /// CHECK: the program that owns the round before delegation (this program).
    #[account(address = crate::ID)]
    pub owner_program: UncheckedAccount<'info>,

    /// CHECK: the delegation program, pinned to the SDK's id.
    #[account(address = ephemeral_rollups_sdk::id())]
    pub delegation_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_delegate_round<'info>(ctx: Context<'info, DelegateRound<'info>>) -> Result<()> {
    // Check the round holds a valid Round (discriminator and layout) before it
    // is handed to the rollup. Nothing is changed, so nothing is written back.
    {
        let round_info = ctx.accounts.round.to_account_info();
        let data = round_info.try_borrow_data()?;
        Round::try_deserialize(&mut &data[..])?;
    }

    let config = DelegateConfig {
        validator: Some(ER_VALIDATOR_ASIA),
        ..Default::default()
    };
    // The SDK derives the bump and appends it itself, so pass the seeds without it.
    let seeds: &[&[u8]] = &[ROUND_SEED];
    let accounts = DelegateAccounts {
        payer: &ctx.accounts.payer.to_account_info(),
        pda: &ctx.accounts.round.to_account_info(),
        owner_program: &ctx.accounts.owner_program.to_account_info(),
        buffer: &ctx.accounts.buffer_round.to_account_info(),
        delegation_record: &ctx.accounts.delegation_record_round.to_account_info(),
        delegation_metadata: &ctx.accounts.delegation_metadata_round.to_account_info(),
        delegation_program: &ctx.accounts.delegation_program.to_account_info(),
        system_program: &ctx.accounts.system_program.to_account_info(),
    };
    delegate_account(accounts, seeds, config)?;

    msg!("Round delegated to the Asia Ephemeral Rollup");
    Ok(())
}
