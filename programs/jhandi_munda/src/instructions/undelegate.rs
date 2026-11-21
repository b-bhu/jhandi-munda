//! Commit and undelegate (Step 11.4, #44, D20, D39, D40).
//!
//! These run ON THE ROLLUP. The rollup holds the delegated copy of the round (or of
//! a chip stack) and is owned by this program there. `commit_and_undelegate` asks the
//! magic program to write the copy back to Solana and hand the account back to this
//! program. Only the table authority (the config admin) can ask (D40).
//!
//! UNVERIFIED (not run on a validator from here): the rollup's handling of the
//! magic-program CPI and the time it takes for Solana to own the account again. The
//! tests use a no-op stub at the magic program's address, so only the SDK's builder
//! call and our checks run.

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::{
    consts::{MAGIC_CONTEXT_ID, MAGIC_PROGRAM_ID},
    ephem::{FoldableIntentBuilder, MagicIntentBundleBuilder},
};

use crate::{
    error::ErrorCode,
    state::{ChipStack, ProgramConfig, Round},
    CHIPS_SEED, CONFIG_SEED, ROUND_SEED,
};

/// The rollup copy must be this program's account. If the delegation program owns it, the
/// rollup has already scheduled its undelegate (the account is "undelegating"), so say that
/// rather than the migration message (`AccountNotProgramOwned`).
fn require_undelegatable(info: &AccountInfo, already: ErrorCode) -> Result<()> {
    let dlp = Pubkey::new_from_array(ephemeral_rollups_sdk::id().to_bytes());
    if *info.owner == dlp {
        return Err(already.into());
    }
    require_keys_eq!(*info.owner, crate::ID, ErrorCode::AccountNotProgramOwned);
    Ok(())
}

#[derive(Accounts)]
pub struct UndelegateRound<'info> {
    /// The table authority (D40), checked against the config below.
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump,
        constraint = config.admin == payer.key() @ ErrorCode::NotTableAuthority
    )]
    pub config: Account<'info, ProgramConfig>,

    /// CHECK: the round's rollup copy. Owned by this program on the rollup; checked in the handler.
    #[account(mut, seeds = [ROUND_SEED], bump)]
    pub round: UncheckedAccount<'info>,

    /// CHECK: the magic context account, pinned to the SDK's id.
    #[account(mut, address = MAGIC_CONTEXT_ID)]
    pub magic_context: UncheckedAccount<'info>,

    /// CHECK: the magic program, pinned to the SDK's id.
    #[account(address = MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
}

pub fn handle_undelegate_round<'info>(ctx: Context<'info, UndelegateRound<'info>>) -> Result<()> {
    let round_info = ctx.accounts.round.to_account_info();
    require_undelegatable(&round_info, ErrorCode::RoundAlreadyUndelegating)?;
    {
        let data = round_info.try_borrow_data()?;
        Round::try_deserialize(&mut &data[..])?;
    }
    MagicIntentBundleBuilder::new(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.magic_context.to_account_info(),
        ctx.accounts.magic_program.to_account_info(),
    )
    .commit_and_undelegate(&[round_info])
    .build_and_invoke()?;
    msg!("Round committed and undelegated back to Solana");
    Ok(())
}

#[derive(Accounts)]
#[instruction(owner: Pubkey)]
pub struct UndelegateChips<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump,
        constraint = config.admin == payer.key() @ ErrorCode::NotTableAuthority
    )]
    pub config: Account<'info, ProgramConfig>,

    /// CHECK: the chip stack's rollup copy, for `owner`. Checked in the handler.
    #[account(mut, seeds = [CHIPS_SEED, owner.as_ref()], bump)]
    pub chips: UncheckedAccount<'info>,

    /// CHECK: the magic context account, pinned to the SDK's id.
    #[account(mut, address = MAGIC_CONTEXT_ID)]
    pub magic_context: UncheckedAccount<'info>,

    /// CHECK: the magic program, pinned to the SDK's id.
    #[account(address = MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
}

pub fn handle_undelegate_chips<'info>(
    ctx: Context<'info, UndelegateChips<'info>>,
    owner: Pubkey,
) -> Result<()> {
    let chips_info = ctx.accounts.chips.to_account_info();
    require_undelegatable(&chips_info, ErrorCode::ChipStackAlreadyUndelegating)?;
    {
        let data = chips_info.try_borrow_data()?;
        let stack = ChipStack::try_deserialize(&mut &data[..])?;
        require_keys_eq!(stack.owner, owner, ErrorCode::NotChipOwner);
    }
    MagicIntentBundleBuilder::new(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.magic_context.to_account_info(),
        ctx.accounts.magic_program.to_account_info(),
    )
    .commit_and_undelegate(&[chips_info])
    .build_and_invoke()?;
    msg!("Chip stack committed and undelegated back to Solana");
    Ok(())
}
