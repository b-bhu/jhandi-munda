//! Layout migrations for accounts deployed before their current layout (#42, #109).
//!
//! Each migration:
//! - may only be signed by the program's upgrade authority, read from the
//!   upgradeable loader's ProgramData account (not by any wallet);
//! - reads the old fields through a legacy struct, then writes the current struct,
//!   so the new size comes from the struct itself (`INIT_SPACE`), not from a number;
//! - runs once: an account that is not in the expected old length is refused.
//!
//! - `migrate_config`: the config (42 or 74 -> current). From 42 it sets `admin` to the authority
//!   (D40); from 74 (the layout before `transfers_enabled` and the overflow fields, D46, D48) it keeps
//!   `admin` and every other field, and the new fields start at their defaults.
//! - `migrate_table`: the table (34 -> current). New fee pots are zero.
//! - `migrate_round`: the round (388 -> current). It only grows the account and
//!   sets `bet_count` and `settled_count` to zero. It does NOT change the status or
//!   any other field. A live round (Open or Rolling) is refused. It must be owned
//!   by this program, so a delegated round is refused on Solana. Whether the
//!   rollup lets this run on the delegated copy is UNVERIFIED (see docs/er-design.md).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{bpf_loader_upgradeable, program::invoke, system_instruction};
use anchor_lang::Discriminator;

use crate::{
    error::ErrorCode,
    state::{ProgramConfig, Round, RoundStatus, TableConfig},
    CONFIG_SEED, MAX_PLAYERS, ROUND_V1_SEED, TABLE_SEED,
};

/// The config before `admin`: 8 discriminator + musd_mint 32 + bump 1 + musd_bump 1.
const OLD_CONFIG_LEN: usize = 8 + 32 + 1 + 1;
/// The config before `transfers_enabled`, `overflow_target` and `overflow_mark` (the devnet
/// layout): 8 discriminator + admin 32 + musd_mint 32 + bump 1 + musd_bump 1.
const CONFIG_V74_LEN: usize = 8 + 32 + 32 + 1 + 1;
/// The table before the fee pots: 8 discriminator + 34 - 8 = 26 bytes of fields.
const OLD_TABLE_LEN: usize = 34;
/// The round before bet_count and settled_count: 8 discriminator + 380 bytes.
const OLD_ROUND_LEN: usize = 388;

/// The table as it was deployed (no fee pots).
#[derive(AnchorDeserialize)]
struct TableV34 {
    round_length_secs: i64,
    min_bet: u64,
    max_bet: u64,
    max_bet_enabled: bool,
    bump: u8,
}

/// The round as it was deployed (no bet_count or settled_count).
struct RoundV388 {
    status: RoundStatus,
    opened_at: i64,
    round_id: u64,
    players: Vec<Pubkey>,
    request_id: [u8; 32],
    faces: [u8; 6],
    bump: u8,
}

/// Reads the deployed round by offsets. Its players list is stored with room for
/// MAX_PLAYERS entries, so the unused slots (zeros) must be skipped by position:
/// a Borsh Vec read would leave them in the stream and shift every later field.
fn read_round_v388(data: &[u8]) -> Result<RoundV388> {
    let d = &data[8..];
    let status = match d[0] {
        0 => RoundStatus::Idle,
        1 => RoundStatus::Open,
        2 => RoundStatus::Rolling,
        3 => RoundStatus::Rolled,
        _ => return err!(ErrorCode::WrongLayoutForMigration),
    };
    let opened_at = i64::from_le_bytes(d[1..9].try_into().unwrap());
    let round_id = u64::from_le_bytes(d[9..17].try_into().unwrap());
    let n = u32::from_le_bytes(d[17..21].try_into().unwrap()) as usize;
    require!(n <= MAX_PLAYERS, ErrorCode::WrongLayoutForMigration);
    let players = (0..n)
        .map(|i| Pubkey::from(<[u8; 32]>::try_from(&d[21 + i * 32..21 + (i + 1) * 32]).unwrap()))
        .collect();
    // Skip the unused player slots: the list has room for MAX_PLAYERS entries.
    let tail = 21 + MAX_PLAYERS * 32;
    let request_id = <[u8; 32]>::try_from(&d[tail..tail + 32]).unwrap();
    let faces = <[u8; 6]>::try_from(&d[tail + 32..tail + 38]).unwrap();
    let bump = d[tail + 38];
    Ok(RoundV388 { status, opened_at, round_id, players, request_id, faces, bump })
}

/// Checks that `authority` is the program's upgrade authority, per the ProgramData account.
fn require_upgrade_authority(program_data: &AccountInfo, authority: &Pubkey) -> Result<()> {
    let loader = bpf_loader_upgradeable::id();
    let (key, _) = Pubkey::find_program_address(&[crate::ID.as_ref()], &loader);
    require_keys_eq!(*program_data.key, key, ErrorCode::NotUpgradeAuthority);
    require_keys_eq!(*program_data.owner, loader, ErrorCode::NotUpgradeAuthority);
    let data = program_data.try_borrow_data()?;
    // UpgradeableLoaderState::ProgramData: tag 3 (u32), slot (u64), Option<Pubkey>.
    require!(data.len() >= 45, ErrorCode::NotUpgradeAuthority);
    require!(data[0..4] == [3, 0, 0, 0], ErrorCode::NotUpgradeAuthority);
    require!(data[12] == 1, ErrorCode::NotUpgradeAuthority);
    require!(data[13..45] == authority.to_bytes(), ErrorCode::NotUpgradeAuthority);
    Ok(())
}

/// Grows `info` to `new_len`, paying the extra rent from `payer`.
fn grow<'info>(
    info: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    new_len: usize,
) -> Result<()> {
    let needed = Rent::get()?.minimum_balance(new_len);
    let current = info.lamports();
    if needed > current {
        invoke(
            &system_instruction::transfer(payer.key, info.key, needed - current),
            &[payer.clone(), info.clone(), system_program.clone()],
        )?;
    }
    info.resize(new_len)?;
    Ok(())
}

/// Writes a serialized account over `info`, zero-padded to its new length.
fn write_account<T: AccountSerialize>(info: &AccountInfo, value: &T, new_len: usize) -> Result<()> {
    let mut bytes = Vec::with_capacity(new_len);
    value.try_serialize(&mut bytes)?;
    require!(bytes.len() <= new_len, ErrorCode::MathOverflow);
    bytes.resize(new_len, 0);
    info.try_borrow_mut_data()?.copy_from_slice(&bytes);
    Ok(())
}

// --- migrate_config ---

#[derive(Accounts)]
pub struct MigrateConfig<'info> {
    /// CHECK: the config PDA. Its data is in the old layout; this instruction resizes
    /// and rewrites it. Seeds pin the address.
    #[account(mut, seeds = [CONFIG_SEED], bump)]
    pub config: UncheckedAccount<'info>,

    /// The program's upgrade authority: the only signer that can migrate.
    #[account(mut)]
    pub upgrade_authority: Signer<'info>,

    /// CHECK: this program's ProgramData account (upgradeable loader). Checked in the handler.
    pub program_data: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_migrate_config<'info>(ctx: Context<'info, MigrateConfig<'info>>) -> Result<()> {
    let authority = ctx.accounts.upgrade_authority.key();
    require_upgrade_authority(&ctx.accounts.program_data.to_account_info(), &authority)?;

    let config_info = ctx.accounts.config.to_account_info();
    let (admin, musd_mint, bump, musd_bump) = {
        let data = config_info.try_borrow_data()?;
        require!(
            data.len() == OLD_CONFIG_LEN || data.len() == CONFIG_V74_LEN,
            ErrorCode::ConfigAlreadyMigrated
        );
        require!(data[0..8] == *ProgramConfig::DISCRIMINATOR, ErrorCode::BetAccountMismatch);
        let key = |from: usize| Pubkey::from(<[u8; 32]>::try_from(&data[from..from + 32]).unwrap());
        if data.len() == OLD_CONFIG_LEN {
            // 42 bytes: no admin yet. The upgrade authority becomes the admin (D40).
            (authority, key(8), data[40], data[41])
        } else {
            // 74 bytes: the admin was set at `initialize`; keep it.
            (key(8), key(40), data[72], data[73])
        }
    };
    let new_len = 8 + ProgramConfig::INIT_SPACE;
    grow(&config_info, &ctx.accounts.upgrade_authority.to_account_info(), &ctx.accounts.system_program.to_account_info(), new_len)?;
    let config = ProgramConfig {
        admin,
        musd_mint,
        bump,
        musd_bump,
        transfers_enabled: false,
        overflow_target: crate::OVERFLOW_TARGET,
        overflow_mark: 0,
    };
    write_account(&config_info, &config, new_len)?;
    msg!("Config migrated to the current layout; admin is {}", admin);
    Ok(())
}

// --- migrate_table ---

#[derive(Accounts)]
pub struct MigrateTable<'info> {
    /// CHECK: the table config PDA in the old layout; resized and rewritten here.
    #[account(mut, seeds = [TABLE_SEED], bump)]
    pub table: UncheckedAccount<'info>,

    #[account(mut)]
    pub upgrade_authority: Signer<'info>,

    /// CHECK: this program's ProgramData account. Checked in the handler.
    pub program_data: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_migrate_table<'info>(ctx: Context<'info, MigrateTable<'info>>) -> Result<()> {
    require_upgrade_authority(&ctx.accounts.program_data.to_account_info(), &ctx.accounts.upgrade_authority.key())?;
    let info = ctx.accounts.table.to_account_info();
    require_keys_eq!(*info.owner, crate::ID, ErrorCode::AccountNotProgramOwned);
    let old = {
        let data = info.try_borrow_data()?;
        require!(data.len() == OLD_TABLE_LEN, ErrorCode::WrongLayoutForMigration);
        require!(data[0..8] == *TableConfig::DISCRIMINATOR, ErrorCode::WrongLayoutForMigration);
        TableV34::deserialize(&mut &data[8..])?
    };
    // New fee pots start at zero; the old fields are kept.
    let table = TableConfig {
        round_length_secs: old.round_length_secs,
        min_bet: old.min_bet,
        max_bet: old.max_bet,
        max_bet_enabled: old.max_bet_enabled,
        fee_treasury: 0,
        fee_devs: 0,
        fee_stakers: 0,
        bump: old.bump,
    };
    let new_len = 8 + TableConfig::INIT_SPACE;
    grow(&info, &ctx.accounts.upgrade_authority.to_account_info(), &ctx.accounts.system_program.to_account_info(), new_len)?;
    write_account(&info, &table, new_len)?;
    msg!("Table migrated to the current layout; fee pots are zero");
    Ok(())
}

// --- migrate_round ---

#[derive(Accounts)]
pub struct MigrateRound<'info> {
    /// CHECK: the v1 round PDA in the old layout (D41: only v1 is migrated; round_v2 is
    /// always the current size); resized and rewritten here. Must be owned by this program
    /// (a delegated round is refused on Solana).
    #[account(mut, seeds = [ROUND_V1_SEED], bump)]
    pub round: UncheckedAccount<'info>,

    #[account(mut)]
    pub upgrade_authority: Signer<'info>,

    /// CHECK: this program's ProgramData account. Checked in the handler.
    pub program_data: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_migrate_round<'info>(ctx: Context<'info, MigrateRound<'info>>) -> Result<()> {
    require_upgrade_authority(&ctx.accounts.program_data.to_account_info(), &ctx.accounts.upgrade_authority.key())?;
    let info = ctx.accounts.round.to_account_info();
    require_keys_eq!(*info.owner, crate::ID, ErrorCode::AccountNotProgramOwned);
    let old = {
        let data = info.try_borrow_data()?;
        require!(data.len() == OLD_ROUND_LEN, ErrorCode::WrongLayoutForMigration);
        require!(data[0..8] == *Round::DISCRIMINATOR, ErrorCode::WrongLayoutForMigration);
        read_round_v388(&data)?
    };
    // A live round (Open or Rolling) is never migrated: its bets are still in play.
    // Any other status is kept as it is: this only adds the two counters.
    require!(
        old.status == RoundStatus::Idle || old.status == RoundStatus::Rolled,
        ErrorCode::RoundIsLive
    );
    let round = Round {
        status: old.status,
        opened_at: old.opened_at,
        round_id: old.round_id,
        players: old.players,
        request_id: old.request_id,
        faces: old.faces,
        bet_count: 0,
        settled_count: 0,
        bump: old.bump,
    };
    let new_len = 8 + Round::INIT_SPACE;
    grow(&info, &ctx.accounts.upgrade_authority.to_account_info(), &ctx.accounts.system_program.to_account_info(), new_len)?;
    write_account(&info, &round, new_len)?;
    msg!("Round migrated to the current layout; counts start at zero");
    Ok(())
}
