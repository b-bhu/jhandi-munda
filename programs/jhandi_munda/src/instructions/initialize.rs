use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
};

use crate::{
    error::ErrorCode, state::ProgramConfig, CONFIG_SEED, MUSD_DECIMALS, MUSD_MINT_AUTHORITY_SEED,
    MUSD_MINT_SEED, MUSD_MINT_SPACE, SPL_TOKEN_PROGRAM_ID,
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Program config. One per program, seeds include no arguments.
    #[account(
        init,
        payer = payer,
        space = 8 + ProgramConfig::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, ProgramConfig>,

    /// CHECK: the MUSD mint. Created here as a PDA owned by the SPL token
    /// program (init + owner), then set up by the `initialize_mint2` CPI in
    /// the handler below.
    #[account(
        init,
        payer = payer,
        space = MUSD_MINT_SPACE,
        owner = SPL_TOKEN_PROGRAM_ID,
        seeds = [MUSD_MINT_SEED],
        bump
    )]
    pub musd_mint: UncheckedAccount<'info>,

    /// CHECK: the mint authority PDA. No keypair exists for it; only this
    /// program can sign for it, which is what makes the mint program-controlled.
    #[account(seeds = [MUSD_MINT_AUTHORITY_SEED], bump)]
    pub musd_mint_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: verified against SPL_TOKEN_PROGRAM_ID in the handler before the CPI.
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.token_program.key(),
        SPL_TOKEN_PROGRAM_ID,
        ErrorCode::InvalidTokenProgram
    );

    // SPL token `initialize_mint2` (opcode 20): [20][decimals u8]
    // [mint_authority: 32 bytes][freeze authority flag: 0 = None]. Signed by
    // the mint authority PDA, so only this program can ever mint MUSD.
    let mut data = Vec::with_capacity(35);
    data.push(20u8);
    data.push(MUSD_DECIMALS);
    data.extend_from_slice(ctx.accounts.musd_mint_authority.key().as_ref());
    data.push(0u8);

    let ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![AccountMeta::new(ctx.accounts.musd_mint.key(), false)],
        data,
    };

    invoke_signed(
        &ix,
        &[ctx.accounts.musd_mint.to_account_info()],
        &[&[MUSD_MINT_AUTHORITY_SEED, &[ctx.bumps.musd_mint_authority]]],
    )?;

    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.payer.key();
    config.musd_mint = ctx.accounts.musd_mint.key();
    config.bump = ctx.bumps.config;
    config.musd_bump = ctx.bumps.musd_mint_authority;

    msg!("Program config and MUSD mint initialized");
    Ok(())
}
