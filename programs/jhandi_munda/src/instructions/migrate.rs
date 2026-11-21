//! `migrate_config` (#42 Blitz verify): moves the deployed config to the current
//! layout. The devnet config was written before `admin` existed, so its data is
//! 42 bytes; the current `ProgramConfig` is 74 bytes. The migration:
//!
//! - may only be signed by the program's upgrade authority, read from the
//!   upgradeable loader's ProgramData account (not by any wallet);
//! - reallocates the config to the current size and keeps the old fields;
//! - sets `admin` to the upgrade authority, which then can delegate the round (D40).
//!
//! It runs once: a config already at the current size is refused.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{bpf_loader_upgradeable, program::invoke, system_instruction};
use anchor_lang::Discriminator;

use crate::{error::ErrorCode, state::ProgramConfig, CONFIG_SEED};

/// The config before `admin` existed: 8 discriminator + musd_mint 32 + bump 1 + musd_bump 1.
const OLD_CONFIG_LEN: usize = 8 + 32 + 1 + 1;

#[derive(Accounts)]
pub struct MigrateConfig<'info> {
    /// CHECK: the config PDA. Its data is in the old layout; this instruction resizes
    /// and rewrites it. Seeds pin the address.
    #[account(mut, seeds = [CONFIG_SEED], bump)]
    pub config: UncheckedAccount<'info>,

    /// The program's upgrade authority: the only signer that can migrate (checked
    /// against the ProgramData account in the handler).
    #[account(mut)]
    pub upgrade_authority: Signer<'info>,

    /// CHECK: this program's ProgramData account (upgradeable loader). Checked in the handler.
    pub program_data: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_migrate_config<'info>(ctx: Context<'info, MigrateConfig<'info>>) -> Result<()> {
    let authority = ctx.accounts.upgrade_authority.key();

    // 1. The ProgramData account of this program, and its upgrade authority.
    let loader = bpf_loader_upgradeable::id();
    let (program_data_key, _) = Pubkey::find_program_address(&[crate::ID.as_ref()], &loader);
    require_keys_eq!(ctx.accounts.program_data.key(), program_data_key, ErrorCode::NotUpgradeAuthority);
    require_keys_eq!(*ctx.accounts.program_data.owner, loader, ErrorCode::NotUpgradeAuthority);
    {
        let data = ctx.accounts.program_data.try_borrow_data()?;
        // UpgradeableLoaderState::ProgramData: tag 3 (u32), slot (u64), Option<Pubkey>.
        require!(data.len() >= 45, ErrorCode::NotUpgradeAuthority);
        require!(data[0..4] == [3, 0, 0, 0], ErrorCode::NotUpgradeAuthority);
        require!(data[12] == 1, ErrorCode::NotUpgradeAuthority);
        require!(data[13..45] == authority.to_bytes(), ErrorCode::NotUpgradeAuthority);
    }

    // 2. The config must still be in the old layout (migrate once).
    let config_info = ctx.accounts.config.to_account_info();
    let (musd_mint, bump, musd_bump) = {
        let data = config_info.try_borrow_data()?;
        require!(data.len() == OLD_CONFIG_LEN, ErrorCode::ConfigAlreadyMigrated);
        require!(data[0..8] == *ProgramConfig::DISCRIMINATOR, ErrorCode::BetAccountMismatch);
        (
            Pubkey::from(<[u8; 32]>::try_from(&data[8..40]).unwrap()),
            data[40],
            data[41],
        )
    };

    // 3. Grow the account to the current size, paying the extra rent from the authority.
    let new_len = 8 + ProgramConfig::INIT_SPACE;
    let needed = Rent::get()?.minimum_balance(new_len);
    let current = config_info.lamports();
    if needed > current {
        invoke(
            &system_instruction::transfer(&authority, &config_info.key(), needed - current),
            &[
                ctx.accounts.upgrade_authority.to_account_info(),
                config_info.clone(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
    }
    config_info.resize(new_len)?;

    // 4. Write the current layout: discriminator, admin, musd_mint, bump, musd_bump.
    let mut data = config_info.try_borrow_mut_data()?;
    data[0..8].copy_from_slice(ProgramConfig::DISCRIMINATOR);
    data[8..40].copy_from_slice(authority.as_ref());
    data[40..72].copy_from_slice(musd_mint.as_ref());
    data[72] = bump;
    data[73] = musd_bump;

    msg!("Config migrated to the current layout; admin is the upgrade authority");
    Ok(())
}
