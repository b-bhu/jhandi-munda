use {
    anchor_lang::{
        prelude::*,
        solana_program::instruction::Instruction,
        InstructionData, ToAccountMetas,
    },
    jhandi_munda::{
        error::ErrorCode, CONFIG_SEED, FAUCET_AMOUNT, FAUCET_RECORD_SEED,
        MUSD_MINT_AUTHORITY_SEED, MUSD_MINT_SEED, POOL_SEED, POOL_VAULT_SEED, SHARE_MINT_SEED,
        SPL_TOKEN_PROGRAM_ID,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

// SPL token account layout (Account::LEN): 0..32 mint, 32..64 owner, 64..72
// amount. SPL mint layout: 36..44 supply.
const TOKEN_ACCOUNT_SPACE: usize = 165;
const TOKEN_AMOUNT_OFFSET: usize = 64;
const MINT_SUPPLY_OFFSET: usize = 36;

// LiteSVM starts its Clock at unix 0, where the faucet's one-hour cooldown
// can never pass, so the faucet call needs this timestamp stamped first.
const BASE_TIME: i64 = 1_700_000_000;

// The issue's deposit: 100 MUSD in MUSD base units (6 decimals).
const DEPOSIT_AMOUNT: u64 = 100_000_000;

// Sends one instruction signed only by `signer`, expiring the blockhash
// first (LiteSVM needs a fresh one per transaction), and returns the raw
// result so tests can assert success or failure.
fn send(svm: &mut LiteSVM, signer: &Keypair, instruction: &Instruction) -> TransactionResult {
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg =
        Message::new_with_blockhash(&[instruction.clone()], Some(&signer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[signer]).unwrap();
    svm.send_transaction(tx)
}

// Runs Step 2's `initialize` (config + MUSD mint) and Step 3.1's
// `initialize_pool` (empty pool PDA, vault, share mint), returning the PDAs
// the deposit tests need.
fn setup() -> (
    LiteSVM,
    Keypair,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
) {
    let program_id = jhandi_munda::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/jhandi_munda.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let (config, _) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);
    let (musd_mint, _) = Pubkey::find_program_address(&[MUSD_MINT_SEED], &program_id);
    let (musd_mint_authority, _) =
        Pubkey::find_program_address(&[MUSD_MINT_AUTHORITY_SEED], &program_id);
    let (pool, _) = Pubkey::find_program_address(&[POOL_SEED], &program_id);
    let (vault, _) = Pubkey::find_program_address(&[POOL_VAULT_SEED], &program_id);
    let (share_mint, _) = Pubkey::find_program_address(&[SHARE_MINT_SEED], &program_id);

    // Step 2 first: the config and the one dollar mint the pool reuses.
    let instruction = Instruction::new_with_bytes(
        program_id,
        &jhandi_munda::instruction::Initialize {}.data(),
        jhandi_munda::accounts::Initialize {
            config,
            musd_mint,
            musd_mint_authority,
            payer: payer.pubkey(),
            token_program: SPL_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );
    let res = send(&mut svm, &payer, &instruction);
    assert!(res.is_ok(), "initialize failed: {:?}", res.err());

    // The pool init: creates the empty pool state, vault, and share mint.
    let instruction = Instruction::new_with_bytes(
        program_id,
        &jhandi_munda::instruction::InitializePool {}.data(),
        jhandi_munda::accounts::InitializePool {
            config,
            pool,
            share_mint,
            vault,
            musd_mint,
            payer: payer.pubkey(),
            token_program: SPL_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );
    let res = send(&mut svm, &payer, &instruction);
    assert!(res.is_ok(), "initialize_pool failed: {:?}", res.err());

    // deposit reads the payout queue (#110), so it must exist.
    let queue = Pubkey::find_program_address(&[jhandi_munda::QUEUE_SEED], &jhandi_munda::id()).0;
    let instruction = Instruction::new_with_bytes(
        jhandi_munda::id(),
        &jhandi_munda::instruction::InitQueue {}.data(),
        jhandi_munda::accounts::InitQueue {
            queue,
            payer: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );
    let res = send(&mut svm, &payer, &instruction);
    assert!(res.is_ok(), "init_queue failed: {:?}", res.err());

    (svm, payer, config, musd_mint, musd_mint_authority, pool, vault, share_mint)
}

// Rewrites the Clock sysvar the program reads with `Clock::get()`.
fn set_clock(svm: &mut LiteSVM, unix_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    svm.set_sysvar::<Clock>(&clock);
}

// The wallet's faucet-record PDA (D33): seeds `["faucet", wallet_pubkey]`.
fn faucet_record_pda(wallet: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[FAUCET_RECORD_SEED, wallet.as_ref()], &jhandi_munda::id()).0
}

// Gives one wallet its 100 MUSD the demo way: the faucet. Each test wallet
// claims at most once, so its fresh record passes the cooldown check.
fn faucet(svm: &mut LiteSVM, wallet: &Keypair, config: &Pubkey, musd_mint: &Pubkey,
          musd_mint_authority: &Pubkey, ata: &Pubkey, record: &Pubkey) {
    set_clock(svm, BASE_TIME);
    let instruction = Instruction::new_with_bytes(
        jhandi_munda::id(),
        &jhandi_munda::instruction::MusdFaucet {}.data(),
        jhandi_munda::accounts::MusdFaucet {
            config: *config,
            musd_mint: *musd_mint,
            musd_mint_authority: *musd_mint_authority,
            faucet_record: *record,
            wallet: wallet.pubkey(),
            destination: *ata,
            associated_token_program: jhandi_munda::ATA_PROGRAM_ID,
            token_program: SPL_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );
    let res = send(svm, wallet, &instruction);
    assert!(res.is_ok(), "musd_faucet failed: {:?}", res.err());
}

// Calls `deposit` for one wallet and returns the raw result so tests can
// assert both success and failure. The MUSD source and share destination are
// the wallet's canonical ATAs, derived the same way the program checks them.
fn try_deposit(svm: &mut LiteSVM, wallet: &Keypair, pool: &Pubkey, musd_mint: &Pubkey,
               vault: &Pubkey, share_mint: &Pubkey, amount: u64) -> TransactionResult {
    let depositor = wallet.pubkey();
    let depositor_ata = jhandi_munda::musd_ata(&depositor, musd_mint);
    let depositor_shares = jhandi_munda::share_ata(&depositor, share_mint);

    let instruction = Instruction::new_with_bytes(
        jhandi_munda::id(),
        &jhandi_munda::instruction::Deposit { amount }.data(),
        jhandi_munda::accounts::Deposit {
            pool: *pool,
            config: Pubkey::find_program_address(&[CONFIG_SEED], &jhandi_munda::id()).0,
            share_mint: *share_mint,
            vault: *vault,
            depositor,
            depositor_ata,
            depositor_shares,
            insurance_vault: Pubkey::find_program_address(&[jhandi_munda::INSURANCE_VAULT_SEED], &jhandi_munda::id()).0,
            queue: Pubkey::find_program_address(&[jhandi_munda::QUEUE_SEED], &jhandi_munda::id()).0,
            token_program: SPL_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
            associated_token_program: jhandi_munda::ATA_PROGRAM_ID,
        }
        .to_account_metas(None),
    );
    send(svm, wallet, &instruction)
}

// Reads a token account as (amount, mint, owner), asserting the layout.
fn token_account(svm: &LiteSVM, address: &Pubkey, label: &str) -> (u64, Pubkey, Pubkey) {
    let acc = svm
        .get_account(address)
        .unwrap_or_else(|| panic!("{} must exist", label));
    assert_eq!(acc.owner, SPL_TOKEN_PROGRAM_ID, "{} owner must be the SPL token program", label);
    assert_eq!(acc.data.len(), TOKEN_ACCOUNT_SPACE, "{} must be a plain SPL token account", label);
    // 0..32 mint, 32..64 owner, 64..72 amount.
    let mint = Pubkey::from(<[u8; 32]>::try_from(&acc.data[0..32]).unwrap());
    let owner = Pubkey::from(<[u8; 32]>::try_from(&acc.data[32..64]).unwrap());
    let amount = u64::from_le_bytes(
        <[u8; 8]>::try_from(&acc.data[TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8]).unwrap(),
    );
    (amount, mint, owner)
}

fn mint_supply(svm: &LiteSVM, mint: &Pubkey) -> u64 {
    u64::from_le_bytes(
        <[u8; 8]>::try_from(
            &svm.get_account(mint).unwrap().data[MINT_SUPPLY_OFFSET..MINT_SUPPLY_OFFSET + 8],
        )
        .unwrap(),
    )
}

// Custom errors reach the chain as InstructionError::Custom(6000 + variant),
// like FaucetCooldown in the faucet tests.
fn error_code(variant: ErrorCode) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + variant as u32
}

// The Done-when: faucet 100 MUSD, deposit it, see 100 shares at NAV $1.00
// (1_000_000 in the documented integer representation), vault holding 100.
#[test]
fn test_pool_deposit_100() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority, pool, vault, share_mint) =
        setup();
    let payer_ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let payer_shares = jhandi_munda::share_ata(&payer.pubkey(), &share_mint);
    let record = faucet_record_pda(&payer.pubkey());

    // The depositor's share ATA does not exist yet: the deposit must create
    // it. The faucet mints exactly the 100 MUSD being deposited.
    assert!(
        svm.get_account(&payer_shares).is_none(),
        "share ATA must not exist before the first deposit"
    );
    faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &payer_ata, &record);

    let res = try_deposit(&mut svm, &payer, &pool, &musd_mint, &vault, &share_mint, DEPOSIT_AMOUNT);
    assert!(res.is_ok(), "deposit failed: {:?}", res.err());

    // The vault received the whole deposit and stayed a pool-owned MUSD
    // account.
    let (vault_amount, vault_mint, vault_owner) = token_account(&svm, &vault, "pool vault");
    assert_eq!(vault_amount, DEPOSIT_AMOUNT, "the vault must hold the deposited 100 MUSD");
    assert_eq!(vault_mint, musd_mint, "the vault must hold the MUSD mint");
    assert_eq!(vault_owner, pool, "the vault must stay owned by the pool PDA");

    // 100 MUSD at the $1.00 empty-pool NAV minted exactly 100 shares.
    let (share_balance, share_mint_read, share_owner) =
        token_account(&svm, &payer_shares, "depositor share account");
    assert_eq!(
        share_balance, DEPOSIT_AMOUNT,
        "100 MUSD at NAV $1.00 must mint 100 shares (100_000_000 base units)"
    );
    assert_eq!(share_mint_read, share_mint, "shares must be the pool's LP mint");
    assert_eq!(share_owner, payer.pubkey(), "shares must belong to the depositor");
    assert_eq!(mint_supply(&svm, &share_mint), DEPOSIT_AMOUNT, "the share supply must equal the minted shares");
    let (musd_left, _, _) = token_account(&svm, &payer_ata, "depositor MUSD ATA");
    assert_eq!(musd_left, 0, "the full 100 MUSD deposit must leave the depositor");

    // NAV readback in the documented integer representation: $1.00 is
    // 1_000_000 (6 decimals).
    assert_eq!(
        jhandi_munda::nav_per_share(vault_amount, DEPOSIT_AMOUNT),
        1_000_000,
        "NAV after the first deposit must be $1.00 (1_000_000)"
    );
}

#[test]
fn test_pool_deposit_rejects_zero() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority, pool, vault, share_mint) =
        setup();
    let payer_ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let payer_shares = jhandi_munda::share_ata(&payer.pubkey(), &share_mint);
    let record = faucet_record_pda(&payer.pubkey());
    faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &payer_ata, &record);

    let res = try_deposit(&mut svm, &payer, &pool, &musd_mint, &vault, &share_mint, 0);
    let err = res.expect_err("a zero-amount deposit must be rejected");
    let expected = format!("Custom({})", error_code(ErrorCode::ZeroDeposit));
    assert!(
        format!("{:?}", err.err).contains(&expected),
        "expected ZeroDeposit ({}), got {:?}",
        expected,
        err.err
    );

    // The rejected call changed nothing: empty vault, no shares, MUSD kept,
    // no share ATA created.
    let (vault_amount, _, _) = token_account(&svm, &vault, "pool vault");
    assert_eq!(vault_amount, 0, "a rejected zero deposit must not fill the vault");
    assert_eq!(mint_supply(&svm, &share_mint), 0, "a rejected zero deposit must not mint shares");
    let (musd_left, _, _) = token_account(&svm, &payer_ata, "depositor MUSD ATA");
    assert_eq!(musd_left, FAUCET_AMOUNT, "a rejected zero deposit must not take MUSD");
    assert!(
        svm.get_account(&payer_shares).is_none(),
        "a rejected zero deposit must not create a share account"
    );
}
