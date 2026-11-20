//! Redeeming a win claim (D7, D13, #26).
//!
//! - **Wait**: after the claim unlocks (settle time + 90 s), pay 100% of the owed
//!   MUSD from the pool vault.
//! - **Take now**: pay 95% at once from the insurance vault. The claim then owes
//!   the full 100% back to the insurance fund, which `collect_insurance` takes
//!   from the pool once the claim unlocks. The insurance fund keeps the 5%.
//!
//! No FIFO queue here: if the pool cannot pay, the redeem is rejected (Step 8).
//! Token moves are hand-built SPL transfers, as in the earlier steps.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
};

use crate::{
    error::ErrorCode,
    payout::take_now_payout,
    state::{Claim, ClaimAsset, PoolState, ProgramConfig},
    CLAIM_SEED, CONFIG_SEED, INSURANCE_SEED, INSURANCE_VAULT_SEED, MUSD_MINT_SEED, POOL_SEED,
    POOL_VAULT_SEED, SPL_TOKEN_PROGRAM_ID, TOKEN_ACCOUNT_SPACE,
};

/// SPL token `Transfer` (opcode 3): data [3][amount u64 LE].
const TOKEN_IX_TRANSFER: u8 = 3;
/// SPL token `InitializeAccount3` (opcode 18): data [18][authority 32 bytes].
const TOKEN_IX_INITIALIZE_ACCOUNT3: u8 = 18;
/// A token account keeps its balance at bytes 64..72.
const TOKEN_AMOUNT_OFFSET: usize = 64;

fn token_balance(info: &AccountInfo) -> Result<u64> {
    let data = info.try_borrow_data()?;
    let bytes = data
        .get(TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8)
        .ok_or(ErrorCode::InvalidTokenAccount)?;
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| ErrorCode::InvalidTokenAccount)?))
}

/// SPL `Transfer` of `amount` from `from` to `to`, authorised by `authority`
/// (a wallet that signed the transaction, or a PDA via `signer_seeds`).
fn spl_transfer<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_TRANSFER);
    data.extend_from_slice(&amount.to_le_bytes());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(from.key(), false),
            AccountMeta::new(to.key(), false),
            AccountMeta::new_readonly(authority.key(), true),
        ],
        data,
    };
    invoke_signed(&ix, &[from.clone(), to.clone(), authority.clone()], signer_seeds)?;
    Ok(())
}

#[derive(Accounts)]
pub struct InitInsurance<'info> {
    /// Program config: pins which MUSD mint the insurance vault holds.
    #[account(seeds = [CONFIG_SEED], bump)]
    pub config: Account<'info, ProgramConfig>,

    /// CHECK: the insurance vault, a token account owned by the insurance PDA.
    /// Created here, then initialised by the handler.
    #[account(
        init,
        payer = payer,
        space = TOKEN_ACCOUNT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [INSURANCE_VAULT_SEED],
        bump
    )]
    pub insurance_vault: UncheckedAccount<'info>,

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

pub fn handle_init_insurance(ctx: Context<InitInsurance>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    // The insurance PDA is the vault's authority; only "Take now" signs as it.
    let authority = Pubkey::find_program_address(&[INSURANCE_SEED], &crate::ID).0;
    let mut data = Vec::with_capacity(33);
    data.push(TOKEN_IX_INITIALIZE_ACCOUNT3);
    data.extend_from_slice(authority.as_ref());
    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(ctx.accounts.insurance_vault.key(), false),
            AccountMeta::new_readonly(ctx.accounts.musd_mint.key(), false),
        ],
        data,
    };
    invoke(
        &ix,
        &[
            ctx.accounts.insurance_vault.to_account_info(),
            ctx.accounts.musd_mint.to_account_info(),
        ],
    )?;
    msg!("Insurance vault initialized");
    Ok(())
}

#[derive(Accounts)]
pub struct RedeemClaim<'info> {
    /// The bet the claim settles. The claim's seeds bind it.
    /// CHECK: only its key is used, to find the claim PDA.
    pub bet: UncheckedAccount<'info>,

    /// The claim being paid. Its player must sign.
    #[account(
        mut,
        seeds = [CLAIM_SEED, bet.key().as_ref()],
        bump = claim.bump,
        has_one = player
    )]
    pub claim: Account<'info, Claim>,

    /// The claim's wallet: signs, and receives the payout.
    pub player: Signer<'info>,

    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool MUSD vault, seeds pin it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump)]
    pub pool_vault: UncheckedAccount<'info>,

    /// CHECK: the insurance vault, seeds pin it.
    #[account(mut, seeds = [INSURANCE_VAULT_SEED], bump)]
    pub insurance_vault: UncheckedAccount<'info>,

    /// CHECK: the insurance authority PDA; signs "Take now" payouts.
    #[account(seeds = [INSURANCE_SEED], bump)]
    pub insurance_authority: UncheckedAccount<'info>,

    /// CHECK: the player's MUSD ATA; checked against the canonical address.
    #[account(mut)]
    pub player_ata: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,
}

fn check_player_ata(ctx: &Context<RedeemClaim>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    require_keys_eq!(
        ctx.accounts.player_ata.key(),
        crate::musd_ata(&ctx.accounts.player.key(), &ctx.accounts.pool.musd_mint),
        ErrorCode::InvalidAtaAddress
    );
    Ok(())
}

/// Wait: after the unlock time, pay 100% of the owed MUSD from the pool.
pub fn handle_redeem_wait(ctx: Context<RedeemClaim>) -> Result<()> {
    check_player_ata(&ctx)?;
    let claim = &ctx.accounts.claim;
    require!(claim.asset == ClaimAsset::Musd, ErrorCode::NoLossClaimYet);
    require!(!claim.redeemed, ErrorCode::ClaimAlreadyRedeemed);
    require!(Clock::get()?.unix_timestamp >= claim.unlock_at, ErrorCode::ClaimLocked);
    let amount = claim.amount;
    require!(token_balance(&ctx.accounts.pool_vault)? >= amount, ErrorCode::InsufficientPoolFunds);

    ctx.accounts.claim.redeemed = true;
    let seeds: &[&[u8]] = &[POOL_SEED, &[ctx.accounts.pool.bump]];
    spl_transfer(
        &ctx.accounts.pool_vault,
        &ctx.accounts.player_ata,
        &ctx.accounts.pool.to_account_info(),
        amount,
        &[seeds],
    )?;
    msg!("Wait: paid {} base MUSD from the pool", amount);
    Ok(())
}

/// Take now: pay 95% at once from the insurance fund. The claim then owes the
/// full 100% back to the insurance fund, collected from the pool later.
pub fn handle_redeem_take_now(ctx: Context<RedeemClaim>) -> Result<()> {
    check_player_ata(&ctx)?;
    let claim = &ctx.accounts.claim;
    require!(claim.asset == ClaimAsset::Musd, ErrorCode::NoLossClaimYet);
    require!(!claim.redeemed, ErrorCode::ClaimAlreadyRedeemed);
    let amount = claim.amount;
    let payout = take_now_payout(amount, claim.early_exit_bps);
    require!(
        token_balance(&ctx.accounts.insurance_vault)? >= payout,
        ErrorCode::InsufficientInsuranceFunds
    );

    ctx.accounts.claim.redeemed = true;
    ctx.accounts.claim.insurance_owed = amount;
    let seeds: &[&[u8]] = &[INSURANCE_SEED, &[ctx.bumps.insurance_authority]];
    spl_transfer(
        &ctx.accounts.insurance_vault,
        &ctx.accounts.player_ata,
        &ctx.accounts.insurance_authority.to_account_info(),
        payout,
        &[seeds],
    )?;
    msg!("Take now: paid {} base MUSD, insurance is owed {}", payout, amount);
    Ok(())
}

#[derive(Accounts)]
pub struct CollectInsurance<'info> {
    /// CHECK: the bet the claim settles; only its key is used.
    pub bet: UncheckedAccount<'info>,

    /// The claim whose "Take now" the insurance fund advanced.
    #[account(mut, seeds = [CLAIM_SEED, bet.key().as_ref()], bump = claim.bump)]
    pub claim: Account<'info, Claim>,

    #[account(seeds = [POOL_SEED], bump)]
    pub pool: Account<'info, PoolState>,

    /// CHECK: the pool MUSD vault, seeds pin it.
    #[account(mut, seeds = [POOL_VAULT_SEED], bump)]
    pub pool_vault: UncheckedAccount<'info>,

    /// CHECK: the insurance vault, seeds pin it.
    #[account(mut, seeds = [INSURANCE_VAULT_SEED], bump)]
    pub insurance_vault: UncheckedAccount<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler.
    pub token_program: UncheckedAccount<'info>,
}

/// After the claim unlocks, the pool repays the insurance fund the full 100% it
/// advanced. Anyone can call it; the insurance fund keeps the 5% it took.
pub fn handle_collect_insurance(ctx: Context<CollectInsurance>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );
    let claim = &ctx.accounts.claim;
    let owed = claim.insurance_owed;
    require!(owed > 0, ErrorCode::NothingToCollect);
    require!(Clock::get()?.unix_timestamp >= claim.unlock_at, ErrorCode::ClaimLocked);
    require!(token_balance(&ctx.accounts.pool_vault)? >= owed, ErrorCode::InsufficientPoolFunds);

    ctx.accounts.claim.insurance_owed = 0;
    let seeds: &[&[u8]] = &[POOL_SEED, &[ctx.accounts.pool.bump]];
    spl_transfer(
        &ctx.accounts.pool_vault,
        &ctx.accounts.insurance_vault,
        &ctx.accounts.pool.to_account_info(),
        owed,
        &[seeds],
    )?;
    msg!("Insurance collected {} base MUSD from the pool", owed);
    Ok(())
}
