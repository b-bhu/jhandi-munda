//! Step 3.4 NAV test (issue #9): a mocked pool profit moves the NAV above
//! $1.00 and reprices every deposit and withdraw, because both price off the
//! raw vault / share-supply token accounts (D27). The profit is mocked the
//! plain way — an ordinary MUSD transfer straight into the vault — so no
//! privileged "print profit" instruction is needed.
//!
//! Integer math (Done-when): deposits mint `amount × supply / vault` and
//! withdraws pay `shares × vault / supply`, both computed in u128 with the
//! floor left in the pool's favour — any rounding residue stays vault-side,
//! raising the NAV of the remaining shares instead of vanishing, and the
//! final full withdraw (shares == supply) prices the whole remaining vault.
//! The 25 MUSD profit on the 100 MUSD / 100-share pool is chosen so every
//! division in this story lands exactly (`100 × 100 / 125 = 80` etc.), so
//! the assertions below are equalities with no residue to explain.

use {
    anchor_lang::{
        prelude::*,
        solana_program::instruction::{AccountMeta, Instruction},
        InstructionData, ToAccountMetas,
    },
    jhandi_munda::{
        CONFIG_SEED, FAUCET_AMOUNT, FAUCET_COOLDOWN_SECS, FAUCET_RECORD_SEED,
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
// can never pass, so the faucet calls need this timestamp stamped first.
const BASE_TIME: i64 = 1_700_000_000;

// The mocked house profit is a plain SPL `Transfer` (opcode 3, data
// [3][u64 LE]) signed by the token owner, like the program's own CPI data.
const TOKEN_IX_TRANSFER: u8 = 3;

// The story's numbers in MUSD base units (6 decimals): the same 100 MUSD
// deposit both depositors make, the 25 MUSD profit moved straight into the
// vault, the NAV readback before ($1.00) and after ($1.25) that profit, and
// what the second 100 MUSD buys at NAV $1.25 (100 × 100 / 125 = 80).
const DEPOSIT_AMOUNT: u64 = 100_000_000;
const MOCK_PROFIT: u64 = 25_000_000;
const NAV_BEFORE: u64 = 1_000_000;
const NAV_AFTER: u64 = 1_250_000;
const SHARES_SECOND: u64 = 80_000_000;

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
// the NAV tests need.
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

// Gives one wallet 100 MUSD the demo way: the faucet. The claim's timestamp
// is passed by the caller: the first claim of a wallet stamps its clock time,
// and a later claim stamps `now + FAUCET_COOLDOWN_SECS` — the check is >=, so
// the full-hour boundary itself allows the next claim (same pattern as the
// faucet tests, D33).
fn faucet(svm: &mut LiteSVM, wallet: &Keypair, now: i64, config: &Pubkey, musd_mint: &Pubkey,
          musd_mint_authority: &Pubkey, ata: &Pubkey, record: &Pubkey) {
    set_clock(svm, now);
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
// assert success or failure. The MUSD source and share destination are the
// wallet's canonical ATAs, derived the same way the program checks them.
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
            share_mint: *share_mint,
            vault: *vault,
            depositor,
            depositor_ata,
            depositor_shares,
            token_program: SPL_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
            associated_token_program: jhandi_munda::ATA_PROGRAM_ID,
        }
        .to_account_metas(None),
    );
    send(svm, wallet, &instruction)
}

// Calls `withdraw` for one wallet and returns the raw result. The burn
// source and MUSD destination are the wallet's canonical ATAs, derived the
// same way the program checks them.
fn try_withdraw(svm: &mut LiteSVM, wallet: &Keypair, pool: &Pubkey, musd_mint: &Pubkey,
                vault: &Pubkey, share_mint: &Pubkey, amount: u64) -> TransactionResult {
    let withdrawer = wallet.pubkey();
    let withdrawer_shares = jhandi_munda::share_ata(&withdrawer, share_mint);
    let withdrawer_ata = jhandi_munda::musd_ata(&withdrawer, musd_mint);

    let instruction = Instruction::new_with_bytes(
        jhandi_munda::id(),
        &jhandi_munda::instruction::Withdraw { amount }.data(),
        jhandi_munda::accounts::Withdraw {
            pool: *pool,
            share_mint: *share_mint,
            vault: *vault,
            withdrawer,
            withdrawer_shares,
            withdrawer_ata,
            token_program: SPL_TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
    );
    send(svm, wallet, &instruction)
}

// The mocked pool profit (issue What, D27): an ordinary SPL `Transfer` signed
// by the MUSD owner, moving extra MUSD straight into the vault. Nothing
// program-side is involved, so no new shares can be minted: the vault grows
// while the share supply stays put, which is exactly what a higher NAV read
// is made of. No privileged "print profit" instruction exists.
fn transfer_to_vault(svm: &mut LiteSVM, source: &Keypair, source_ata: &Pubkey,
                     vault: &Pubkey, amount: u64) -> TransactionResult {
    let mut data = Vec::with_capacity(9);
    data.push(TOKEN_IX_TRANSFER);
    data.extend_from_slice(&amount.to_le_bytes());
    let transfer = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*source_ata, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(source.pubkey(), true),
        ],
        data,
    };
    send(svm, source, &transfer)
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

// The two-depositor story (issue Done-when, D27): depositor A puts 100 MUSD
// in at the empty-pool $1.00 NAV for 100 shares; 25 MUSD is transferred
// straight into the vault to mock a house profit; the NAV read moves to
// $1.25 without any new deposit; depositor B's same 100 MUSD then buys only
// 80 shares; and both withdraws pay exactly `shares × NAV` — 125 MUSD for A,
// 100 MUSD for B — leaving the vault and the share supply at zero.
#[test]
fn test_pool_nav_after_profit() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority, pool, vault, share_mint) =
        setup();
    let payer_ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let payer_shares = jhandi_munda::share_ata(&payer.pubkey(), &share_mint);
    let payer_record = faucet_record_pda(&payer.pubkey());

    // The second depositor needs SOL for their share-ATA rent and fees, the
    // same way `setup` funded the first wallet.
    let payer_b = Keypair::new();
    svm.airdrop(&payer_b.pubkey(), 1_000_000_000).unwrap();
    let payer_b_ata = jhandi_munda::musd_ata(&payer_b.pubkey(), &musd_mint);
    let payer_b_shares = jhandi_munda::share_ata(&payer_b.pubkey(), &share_mint);
    let payer_b_record = faucet_record_pda(&payer_b.pubkey());

    // ---- Depositor A: 100 MUSD into the empty pool -> 100 shares 1:1.
    faucet(
        &mut svm, &payer, BASE_TIME, &config, &musd_mint, &musd_mint_authority,
        &payer_ata, &payer_record,
    );
    let res = try_deposit(&mut svm, &payer, &pool, &musd_mint, &vault, &share_mint, DEPOSIT_AMOUNT);
    assert!(res.is_ok(), "the first deposit failed: {:?}", res.err());

    let (vault_amount, vault_mint, vault_owner) = token_account(&svm, &vault, "pool vault");
    assert_eq!(vault_amount, DEPOSIT_AMOUNT, "the vault must hold the deposited 100 MUSD");
    assert_eq!(vault_mint, musd_mint, "the vault must stay a MUSD account");
    assert_eq!(vault_owner, pool, "the vault must stay owned by the pool PDA");
    let (shares_a, _, _) = token_account(&svm, &payer_shares, "first depositor share account");
    assert_eq!(shares_a, DEPOSIT_AMOUNT, "100 MUSD at NAV $1.00 must mint 100 shares");
    // The NAV readback (D27): `vault × 1_000_000 / supply`, 1_000_000 = $1.00.
    assert_eq!(
        jhandi_munda::nav_per_share(vault_amount, mint_supply(&svm, &share_mint)),
        NAV_BEFORE,
        "the pool must price at $1.00 before any profit"
    );

    // ---- The mocked house profit: 25 MUSD straight into the vault. A's
    // second faucet claim needs the one-hour warp (D33); the transfer itself
    // is a plain token move, so no deposit runs and no share mints.
    faucet(
        &mut svm, &payer, BASE_TIME + FAUCET_COOLDOWN_SECS, &config, &musd_mint,
        &musd_mint_authority, &payer_ata, &payer_record,
    );
    let res = transfer_to_vault(&mut svm, &payer, &payer_ata, &vault, MOCK_PROFIT);
    assert!(res.is_ok(), "the profit transfer failed: {:?}", res.err());

    let (vault_amount, vault_mint, vault_owner) = token_account(&svm, &vault, "pool vault");
    assert_eq!(
        vault_amount,
        DEPOSIT_AMOUNT + MOCK_PROFIT,
        "the vault must hold deposit + mocked profit"
    );
    assert_eq!(vault_mint, musd_mint, "the profit must land as MUSD");
    assert_eq!(vault_owner, pool, "the profit must land in the pool's own vault");
    assert_eq!(
        mint_supply(&svm, &share_mint),
        DEPOSIT_AMOUNT,
        "a straight vault transfer adds NAV without minting a single share"
    );
    let nav = jhandi_munda::nav_per_share(vault_amount, mint_supply(&svm, &share_mint));
    assert_eq!(nav, NAV_AFTER, "125_000_000 in the vault over 100_000_000 shares is $1.25");
    assert!(nav > NAV_BEFORE, "the mocked profit must move the NAV above $1.00");

    // ---- Depositor B: the SAME 100 MUSD now buys fewer shares. Deposit
    // pricing (u128, floored in the pool's favour): 100_000_000 × 100_000_000
    // / 125_000_000 = 80_000_000 exactly — the 25 MUSD profit makes every
    // division in this story integer-exact, so no floor residue exists here;
    // a residue would stay vault-side (remaining holders gain, never lose).
    faucet(
        &mut svm, &payer_b, BASE_TIME + FAUCET_COOLDOWN_SECS, &config, &musd_mint,
        &musd_mint_authority, &payer_b_ata, &payer_b_record,
    );
    let res = try_deposit(&mut svm, &payer_b, &pool, &musd_mint, &vault, &share_mint, DEPOSIT_AMOUNT);
    assert!(res.is_ok(), "the second deposit failed: {:?}", res.err());

    let (shares_b, _, _) = token_account(&svm, &payer_b_shares, "second depositor share account");
    assert_eq!(shares_b, SHARES_SECOND, "100 MUSD at NAV $1.25 must mint 80 shares, not 100");
    assert!(
        shares_b < shares_a,
        "the same 100 MUSD must buy fewer shares once NAV is above $1.00"
    );
    assert_eq!(
        mint_supply(&svm, &share_mint),
        DEPOSIT_AMOUNT + SHARES_SECOND,
        "the share supply must only grow by B's 80 shares"
    );
    let (vault_amount, _, _) = token_account(&svm, &vault, "pool vault");
    assert_eq!(
        vault_amount,
        DEPOSIT_AMOUNT + MOCK_PROFIT + DEPOSIT_AMOUNT,
        "the vault must also hold B's deposit"
    );
    // B deposited exactly at the current NAV, so the pool dilutes nobody:
    let nav = jhandi_munda::nav_per_share(vault_amount, mint_supply(&svm, &share_mint));
    assert_eq!(nav, NAV_AFTER, "a deposit priced at the current NAV must not move it");

    // ---- Depositor A withdraws all 100 shares. Withdraw pricing (u128,
    // floored in the pool's favour): 100_000_000 × 225_000_000 / 180_000_000
    // = 125_000_000 exactly — `shares × NAV`, i.e. 100_000_000 shares ×
    // 1_250_000 scaled / 1_000_000 = the 125 MUSD the pool now owes.
    // Read the balance BEFORE the withdraw so the delta is what the pool paid.
    let (musd_before, _, _) = token_account(&svm, &payer_ata, "first depositor MUSD ATA");
    let res = try_withdraw(&mut svm, &payer, &pool, &musd_mint, &vault, &share_mint, DEPOSIT_AMOUNT);
    assert!(res.is_ok(), "the first depositor's withdraw failed: {:?}", res.err());

    // The payout readback must be a before/after delta, not the whole ATA
    // balance: A's ATA still holds 75 MUSD here — what stayed of the second
    // faucet claim after A funded the 25 MUSD mock out of it (200 after =
    // 75 before + 125 paid).
    let expected = DEPOSIT_AMOUNT * NAV_AFTER / 1_000_000;
    assert_eq!(expected, 125_000_000, "sanity: 100 shares × $1.25 = 125 MUSD");
    let (musd_back, _, _) = token_account(&svm, &payer_ata, "first depositor MUSD ATA");
    assert_eq!(
        musd_back - musd_before, expected,
        "100 shares at NAV $1.25 must pay exactly 125 MUSD (shares × NAV): after − before"
    );
    let (shares_left, _, _) = token_account(&svm, &payer_shares, "first depositor share account");
    assert_eq!(shares_left, 0, "the withdraw must burn all of A's shares");
    let (vault_amount, _, _) = token_account(&svm, &vault, "pool vault");
    assert_eq!(vault_amount, DEPOSIT_AMOUNT, "the vault must only owe B's 100 now");
    assert_eq!(
        mint_supply(&svm, &share_mint),
        SHARES_SECOND,
        "only B's 80 shares must remain"
    );
    assert_eq!(
        jhandi_munda::nav_per_share(vault_amount, mint_supply(&svm, &share_mint)),
        NAV_AFTER,
        "the remaining shares must still price at $1.25"
    );

    // ---- Depositor B withdraws all 80 shares. Pricing: 80_000_000 ×
    // 100_000_000 / 80_000_000 = 100_000_000 exactly — `shares × NAV`, 80
    // shares × $1.25. A full withdraw of the last shares (shares == supply)
    // prices the whole remaining vault, so no MUSD can be stranded.
    // Read the balance BEFORE the withdraw so the delta is what the pool paid.
    let (musd_before, _, _) = token_account(&svm, &payer_b_ata, "second depositor MUSD ATA");
    let res = try_withdraw(&mut svm, &payer_b, &pool, &musd_mint, &vault, &share_mint, SHARES_SECOND);
    assert!(res.is_ok(), "the second depositor's withdraw failed: {:?}", res.err());

    // Same delta readback as A's, so the pay check never depends on leftover
    // faucet MUSD sitting in the ATA (this story happens to start B at 0).
    let expected = SHARES_SECOND * NAV_AFTER / 1_000_000;
    assert_eq!(expected, 100_000_000, "sanity: 80 shares × $1.25 = 100 MUSD");
    let (musd_back, _, _) = token_account(&svm, &payer_b_ata, "second depositor MUSD ATA");
    assert_eq!(
        musd_back - musd_before, expected,
        "80 shares at NAV $1.25 must pay exactly 100 MUSD (shares × NAV): after − before"
    );
    let (vault_amount, _, vault_owner) = token_account(&svm, &vault, "pool vault");
    assert_eq!(vault_amount, 0, "both withdraws must empty the vault: no MUSD stranded");
    assert_eq!(vault_owner, pool, "the emptied vault must stay owned by the pool PDA");
    assert_eq!(mint_supply(&svm, &share_mint), 0, "both withdraws must burn every share");
    let (shares_left, _, _) = token_account(&svm, &payer_b_shares, "second depositor share account");
    assert_eq!(shares_left, 0, "the withdraw must empty B's shares");

    // ---- Conservation readback. A funded the 25 MUSD mock (the transfer
    // above signs with A's key, out of the second faucet claim) and got the
    // profit back inside the 125 payout, D27: 2 × 100 (both claims) − 100
    // (deposit) − 25 (mock) + 125 (payout) = 200 = 2 × FAUCET_AMOUNT. B broke
    // even — no mock, no profit: 100 (claim) − 100 (deposit) + 100 (payout)
    // = 100 = FAUCET_AMOUNT.
    let (musd_a, _, _) = token_account(&svm, &payer_ata, "first depositor MUSD ATA");
    assert_eq!(musd_a, 2 * FAUCET_AMOUNT, "A's MUSD must balance to both faucet claims");
    let (musd_b, _, _) = token_account(&svm, &payer_b_ata, "second depositor MUSD ATA");
    assert_eq!(musd_b, FAUCET_AMOUNT, "B must end with the deposit they put in");
}
