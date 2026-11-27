//! Step 3.3 withdraw tests (issue #8): after depositing 100 MUSD, a withdraw
//! of 40 shares pays 40 MUSD and leaves 60 shares; the later full withdraw
//! pays the last 60 MUSD and empties the user's shares; an over-withdraw is
//! rejected with nothing moved. NAV is the $1.00 happy path (no profit mock).

use {
    anchor_lang::{
        prelude::*,
        solana_program::instruction::Instruction,
        InstructionData, ToAccountMetas,
    },
    jhandi_munda::{
        error::ErrorCode, CONFIG_SEED, FAUCET_AMOUNT, FAUCET_RECORD_SEED,
        MUSD_MINT_AUTHORITY_SEED, MUSD_MINT_SEED, POOL_SEED, POOL_VAULT_SEED, QUEUE_SEED, SHARE_MINT_SEED,
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

// The issue's numbers in MUSD base units (6 decimals): 100 MUSD deposited,
// the 40-share partial withdraw, then the full 60-share withdraw.
const DEPOSIT_AMOUNT: u64 = 100_000_000;
const PART_SHARES: u64 = 40_000_000;

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
// the withdraw tests need.
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

    // The pool init: creates the pool state, vault, and share mint.
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

    // The payout queue (Step 8): an LP withdraw is refused while winners queue.
    let queue = Pubkey::find_program_address(&[QUEUE_SEED], &jhandi_munda::id()).0;
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

// Calls `deposit` for one wallet (Step 3.2, sets up the shares withdrawn in
// these tests). Returns the raw result so tests can assert success.
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

// Calls `withdraw` for one wallet and returns the raw result so tests can
// assert both success and failure. The burn source and MUSD destination are
// the wallet's canonical ATAs, derived the same way the program checks them.
fn try_withdraw(svm: &mut LiteSVM, wallet: &Keypair, pool: &Pubkey, vault: &Pubkey,
                share_mint: &Pubkey, musd_mint: &Pubkey, amount: u64) -> TransactionResult {
    let withdrawer = wallet.pubkey();
    let withdrawer_shares = jhandi_munda::share_ata(&withdrawer, share_mint);
    let withdrawer_ata = jhandi_munda::musd_ata(&withdrawer, musd_mint);

    let instruction = Instruction::new_with_bytes(
        jhandi_munda::id(),
        &jhandi_munda::instruction::Withdraw { amount }.data(),
        jhandi_munda::accounts::Withdraw {
            pool: *pool,
            config: Pubkey::find_program_address(&[CONFIG_SEED], &jhandi_munda::id()).0,
            share_mint: *share_mint,
            vault: *vault,
            withdrawer,
            withdrawer_shares,
            withdrawer_ata,
            queue: Pubkey::find_program_address(&[QUEUE_SEED], &jhandi_munda::id()).0,
            token_program: SPL_TOKEN_PROGRAM_ID,
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

// The Done-when: deposit 100 MUSD, withdraw 40 shares and receive 40 MUSD
// with 60 shares left; the later full withdraw of the remaining 60 shares
// empties the user's share balance and returns 60 MUSD (the full 100 the
// faucet minted). NAV is $1.00 throughout.
#[test]
fn test_pool_withdraw() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority, pool, vault, share_mint) =
        setup();
    let payer_ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let payer_shares = jhandi_munda::share_ata(&payer.pubkey(), &share_mint);
    let record = faucet_record_pda(&payer.pubkey());

    // The withdrawer funds up with the faucet and deposits the full 100 MUSD
    // (Step 3.2, already tested; here it only sets up the shares withdrawn).
    faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &payer_ata, &record);
    let res = try_deposit(&mut svm, &payer, &pool, &musd_mint, &vault, &share_mint, DEPOSIT_AMOUNT);
    assert!(res.is_ok(), "deposit failed: {:?}", res.err());

    // The partial withdraw: 40 of the held 100 shares.
    let res = try_withdraw(&mut svm, &payer, &pool, &vault, &share_mint, &musd_mint, PART_SHARES);
    assert!(res.is_ok(), "withdraw of 40 shares failed: {:?}", res.err());

    // 40 shares burnt returned exactly 40 MUSD at the $1.00 NAV.
    let (musd_back, musd_mint_read, musd_owner) =
        token_account(&svm, &payer_ata, "withdrawer MUSD ATA");
    assert_eq!(musd_back, PART_SHARES, "40 shares must return 40 MUSD at NAV $1.00");
    assert_eq!(musd_mint_read, musd_mint, "the payout must be MUSD, not shares");
    assert_eq!(musd_owner, payer.pubkey(), "the payout must land in the withdrawer's MUSD ATA");

    // The burnt 40 shares are gone from both the holder and the supply, and
    // the vault paid them out of its 100.
    let (shares_left, share_mint_read, share_owner) =
        token_account(&svm, &payer_shares, "withdrawer share account");
    assert_eq!(shares_left, DEPOSIT_AMOUNT - PART_SHARES, "60 shares must remain after the partial withdraw");
    assert_eq!(share_mint_read, share_mint, "shares must be the pool's LP mint");
    assert_eq!(share_owner, payer.pubkey(), "shares must stay with the withdrawer");
    assert_eq!(
        mint_supply(&svm, &share_mint),
        DEPOSIT_AMOUNT - PART_SHARES,
        "burning 40 shares must shrink the supply to 60"
    );
    let (vault_amount, vault_mint, vault_owner) = token_account(&svm, &vault, "pool vault");
    assert_eq!(vault_amount, DEPOSIT_AMOUNT - PART_SHARES, "the vault must pay out exactly 40 MUSD");
    assert_eq!(vault_mint, musd_mint, "the vault must stay a MUSD account");
    assert_eq!(vault_owner, pool, "the vault must stay owned by the pool PDA");

    // The later full withdraw: the remaining 60 shares empty the user's
    // share balance and return 60 MUSD (40 + 60 = the full 100 MUSD), taking
    // the vault and the share supply to zero.
    let res = try_withdraw(
        &mut svm,
        &payer,
        &pool,
        &vault,
        &share_mint,
        &musd_mint,
        DEPOSIT_AMOUNT - PART_SHARES,
    );
    assert!(res.is_ok(), "withdraw of the last 60 shares failed: {:?}", res.err());

    let (musd_back, _, _) = token_account(&svm, &payer_ata, "withdrawer MUSD ATA");
    assert_eq!(musd_back, FAUCET_AMOUNT, "the full withdraw must return all 100 MUSD the user deposited");
    let (shares_left, _, _) = token_account(&svm, &payer_shares, "withdrawer share account");
    assert_eq!(shares_left, 0, "the full withdraw must empty the user's shares");
    let (vault_amount, _, vault_owner) = token_account(&svm, &vault, "pool vault");
    assert_eq!(vault_amount, 0, "the full withdraw must empty the vault");
    assert_eq!(vault_owner, pool, "the vault must stay owned by the pool PDA");
    assert_eq!(
        mint_supply(&svm, &share_mint),
        0,
        "the full withdraw must burn every share it pays"
    );
}

// A withdraw of more shares than the holder owns is rejected (issue
// Done-when) and nothing moves: vault, supply and both balances stay put.
#[test]
fn test_pool_withdraw_rejects_over_withdraw() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority, pool, vault, share_mint) =
        setup();
    let payer_ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let payer_shares = jhandi_munda::share_ata(&payer.pubkey(), &share_mint);
    let record = faucet_record_pda(&payer.pubkey());
    faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &payer_ata, &record);
    let res = try_deposit(&mut svm, &payer, &pool, &musd_mint, &vault, &share_mint, DEPOSIT_AMOUNT);
    assert!(res.is_ok(), "deposit failed: {:?}", res.err());

    // One share more than the holder owns. The share source and MUSD
    // destination are the wallet's canonical ATAs, so the canonical-address
    // checks pass and the balance check is what rejects.
    let res = try_withdraw(
        &mut svm,
        &payer,
        &pool,
        &vault,
        &share_mint,
        &musd_mint,
        DEPOSIT_AMOUNT + 1,
    );
    let err = res.expect_err("an over-withdraw must be rejected");
    let expected = format!("Custom({})", error_code(ErrorCode::InsufficientShares));
    assert!(
        format!("{:?}", err.err).contains(&expected),
        "expected InsufficientShares ({}), got {:?}",
        expected,
        err.err
    );

    // The rejected call changed nothing.
    let (vault_amount, _, _) = token_account(&svm, &vault, "pool vault");
    assert_eq!(vault_amount, DEPOSIT_AMOUNT, "a rejected withdraw must not pay from the vault");
    assert_eq!(
        mint_supply(&svm, &share_mint),
        DEPOSIT_AMOUNT,
        "a rejected withdraw must not burn shares"
    );
    let (shares_left, _, _) = token_account(&svm, &payer_shares, "withdrawer share account");
    assert_eq!(shares_left, DEPOSIT_AMOUNT, "a rejected withdraw must not burn the holder's shares");
    let (musd_left, _, _) = token_account(&svm, &payer_ata, "withdrawer MUSD ATA");
    assert_eq!(musd_left, 0, "a rejected withdraw must not pay MUSD");
}
