use {
    anchor_lang::{
        prelude::*,
        solana_program::instruction::Instruction,
        InstructionData, ToAccountMetas,
    },
    jhandi_munda::{
        state::FaucetRecord, CONFIG_SEED, FAUCET_AMOUNT, FAUCET_COOLDOWN_SECS,
        FAUCET_RECORD_SEED, MUSD_MINT_AUTHORITY_SEED, MUSD_MINT_SEED, SPL_TOKEN_PROGRAM_ID,
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

// Fixed base clock for the faucet tests. LiteSVM starts its Clock sysvar at
// unix 0, where an hour-long cooldown can never pass, so every test first
// sets the clock to this real-feeling timestamp (D33).
const BASE_TIME: i64 = 1_700_000_000;

// Sets up the program and runs `initialize`; returns (svm, payer, config,
// musd_mint, musd_mint_authority) so the wallet can start with 0 MUSD (no
// token accounts yet).
fn setup() -> (LiteSVM, Keypair, Pubkey, Pubkey, Pubkey) {
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

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "initialize failed: {:?}", res.err());
    (svm, payer, config, musd_mint, musd_mint_authority)
}

// Rewrites the Clock sysvar the program reads with `Clock::get()`. LiteSVM
// runs the program against exactly this sysvar, so this is the clock warp the
// tests use to jump past the faucet's one-hour cooldown (D33).
fn set_clock(svm: &mut LiteSVM, unix_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    svm.set_sysvar::<Clock>(&clock);
}

// The wallet's faucet-record PDA (D33): seeds `["faucet", wallet_pubkey]`.
fn faucet_record_pda(wallet: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[FAUCET_RECORD_SEED, wallet.as_ref()], &jhandi_munda::id()).0
}

// Reads and decodes a faucet record, asserting it exists and belongs to the
// program.
fn faucet_record(svm: &LiteSVM, record: &Pubkey) -> FaucetRecord {
    let acc = svm.get_account(record).expect("faucet record PDA must exist");
    assert_eq!(
        acc.owner,
        jhandi_munda::id(),
        "faucet record PDA must belong to the program"
    );
    let mut data: &[u8] = &acc.data;
    FaucetRecord::try_deserialize(&mut data).expect("faucet record must decode")
}

// Calls `musd_faucet` for one wallet and returns the raw LiteSVM result so
// tests can assert both success and failure.
fn try_faucet(
    svm: &mut LiteSVM,
    payer: &Keypair,
    config: &Pubkey,
    musd_mint: &Pubkey,
    musd_mint_authority: &Pubkey,
    destination: &Pubkey,
    faucet_record: &Pubkey,
) -> TransactionResult {
    let instruction = Instruction::new_with_bytes(
        jhandi_munda::id(),
        &jhandi_munda::instruction::MusdFaucet {}.data(),
        jhandi_munda::accounts::MusdFaucet {
            config: *config,
            musd_mint: *musd_mint,
            musd_mint_authority: *musd_mint_authority,
            faucet_record: *faucet_record,
            wallet: payer.pubkey(),
            destination: *destination,
            associated_token_program: jhandi_munda::ATA_PROGRAM_ID,
            token_program: SPL_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    // A fresh blockhash per call: two identical faucet transactions with the
    // same blockhash collide on signature and the second is AlreadyProcessed.
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();

    svm.send_transaction(tx)
}

// Calls the faucet for one wallet and asserts the transaction succeeded.
fn faucet(
    svm: &mut LiteSVM,
    payer: &Keypair,
    config: &Pubkey,
    musd_mint: &Pubkey,
    musd_mint_authority: &Pubkey,
    destination: &Pubkey,
    faucet_record: &Pubkey,
) {
    let res = try_faucet(svm, payer, config, musd_mint, musd_mint_authority, destination, faucet_record);
    assert!(res.is_ok(), "musd_faucet failed: {:?}", res.err());
}

// Reads the faucet's ATA: (amount, mint, owner), with layout asserts inline.
fn ata_amount(svm: &LiteSVM, ata: &Pubkey) -> (u64, Pubkey, Pubkey) {
    let acc = svm.get_account(ata).expect("MUSD ATA must exist");
    assert_eq!(acc.owner, SPL_TOKEN_PROGRAM_ID, "ATA owner must be the SPL token program");
    assert_eq!(acc.data.len(), TOKEN_ACCOUNT_SPACE, "ATA must be a plain SPL token account");
    // 0..32 mint, 32..64 owner.
    let mint = Pubkey::from(<[u8; 32]>::try_from(&acc.data[0..32]).unwrap());
    let owner = Pubkey::from(<[u8; 32]>::try_from(&acc.data[32..64]).unwrap());
    let amount = u64::from_le_bytes(
        <[u8; 8]>::try_from(&acc.data[TOKEN_AMOUNT_OFFSET..TOKEN_AMOUNT_OFFSET + 8]).unwrap(),
    );
    (amount, mint, owner)
}

fn mint_supply(svm: &LiteSVM, musd_mint: &Pubkey) -> u64 {
    u64::from_le_bytes(
        <[u8; 8]>::try_from(&svm.get_account(musd_mint).unwrap().data[MINT_SUPPLY_OFFSET..MINT_SUPPLY_OFFSET + 8])
            .unwrap(),
    )
}

// FaucetCooldown reaches the chain as InstructionError::Custom(6000 + variant).
fn faucet_cooldown_code() -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET
        + jhandi_munda::error::ErrorCode::FaucetCooldown as u32
}

#[test]
fn test_faucet_first_call_creates_record() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority) = setup();
    let ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let record = faucet_record_pda(&payer.pubkey());
    let wallet = payer.pubkey();

    // The record PDA does not exist yet: the first call must create it.
    assert!(svm.get_account(&record).is_none(), "record must not exist before the first call");

    // Deterministic clock: the record stores exactly this claim timestamp.
    set_clock(&mut svm, BASE_TIME);

    // First faucet call: creates the record PDA and mints 100 MUSD.
    faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &ata, &record);

    let rec = faucet_record(&svm, &record);
    assert_eq!(
        rec.last_claim_time, BASE_TIME,
        "the record must store the claim's unix_timestamp"
    );
    let (_, bump) = Pubkey::find_program_address(&[FAUCET_RECORD_SEED, wallet.as_ref()], &jhandi_munda::id());
    assert_eq!(rec.bump, bump, "the record must store its canonical PDA bump");
    assert_eq!(
        svm.get_account(&record).unwrap().data.len(),
        8 + FaucetRecord::INIT_SPACE,
        "record space must stay at the declared minimum"
    );

    let (amount, mint, owner) = ata_amount(&svm, &ata);
    assert_eq!(amount, FAUCET_AMOUNT, "first faucet call must mint 100 MUSD");
    assert_eq!(mint, musd_mint, "ATA must hold the MUSD mint");
    assert_eq!(owner, wallet, "ATA must belong to the test wallet");
    assert_eq!(mint_supply(&svm, &musd_mint), FAUCET_AMOUNT, "mint supply must equal the minted total");
}

#[test]
fn test_faucet_cooldown_fails_within_an_hour() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority) = setup();
    let ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let record = faucet_record_pda(&payer.pubkey());

    set_clock(&mut svm, BASE_TIME);
    faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &ata, &record);

    // No clock warp: the second call is inside the one-hour window and must
    // fail with the custom FaucetCooldown error.
    let res = try_faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &ata, &record);
    let err = res.expect_err("second call inside the cooldown window must fail");
    let expected = format!("Custom({})", faucet_cooldown_code());
    assert!(
        format!("{:?}", err.err).contains(&expected),
        "expected FaucetCooldown ({}), got {:?}",
        expected,
        err.err
    );

    // The failed call changed nothing: no mint, no supply bump, no record move.
    let (amount, _, _) = ata_amount(&svm, &ata);
    assert_eq!(amount, FAUCET_AMOUNT, "a cooldown-failed call must not mint");
    assert_eq!(
        mint_supply(&svm, &musd_mint),
        FAUCET_AMOUNT,
        "a cooldown-failed call must not raise the supply"
    );
    assert_eq!(
        faucet_record(&svm, &record).last_claim_time,
        BASE_TIME,
        "a cooldown-failed call must not move the record"
    );
}

#[test]
fn test_faucet_succeeds_after_one_hour() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority) = setup();
    let ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let record = faucet_record_pda(&payer.pubkey());
    let wallet = payer.pubkey();

    set_clock(&mut svm, BASE_TIME);
    faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &ata, &record);

    // Warp the clock exactly one hour: the check is >=, so the full-hour
    // boundary itself allows the next claim.
    set_clock(&mut svm, BASE_TIME + FAUCET_COOLDOWN_SECS);
    faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &ata, &record);

    // The second claim went through and the record moved to the new time.
    let (amount, _, owner) = ata_amount(&svm, &ata);
    assert_eq!(amount, 2 * FAUCET_AMOUNT, "the next claim after an hour must mint again");
    assert_eq!(owner, wallet, "the mint must still land in the same wallet's ATA");
    assert_eq!(
        mint_supply(&svm, &musd_mint),
        2 * FAUCET_AMOUNT,
        "mint supply must rise by the second claim"
    );
    assert_eq!(
        faucet_record(&svm, &record).last_claim_time,
        BASE_TIME + FAUCET_COOLDOWN_SECS,
        "the record must move to the second claim's timestamp"
    );
}

#[test]
fn test_faucet_cooldown_is_per_wallet() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority) = setup();
    let ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let record = faucet_record_pda(&payer.pubkey());

    set_clock(&mut svm, BASE_TIME);
    faucet(&mut svm, &payer, &config, &musd_mint, &musd_mint_authority, &ata, &record);

    // A different wallet is not affected by the first wallet's cooldown: it
    // gets its own record PDA and can claim right away.
    let other = Keypair::new();
    svm.airdrop(&other.pubkey(), 1_000_000_000).unwrap();
    let other_ata = jhandi_munda::musd_ata(&other.pubkey(), &musd_mint);
    let other_record = faucet_record_pda(&other.pubkey());
    faucet(&mut svm, &other, &config, &musd_mint, &musd_mint_authority, &other_ata, &other_record);

    let (other_amount, _, other_owner) = ata_amount(&svm, &other_ata);
    assert_eq!(other_amount, FAUCET_AMOUNT, "the other wallet must get its own claim");
    assert_eq!(other_owner, other.pubkey(), "the mint must land in the other wallet's ATA");
    assert_eq!(
        faucet_record(&svm, &other_record).last_claim_time,
        BASE_TIME,
        "the other wallet's record must hold its own claim time"
    );

    // The first wallet's state is untouched by the other wallet's claim.
    assert_eq!(
        faucet_record(&svm, &record).last_claim_time,
        BASE_TIME,
        "the first wallet's record must be unchanged"
    );
    let (payer_amount, _, _) = ata_amount(&svm, &ata);
    assert_eq!(payer_amount, FAUCET_AMOUNT, "the first wallet's balance must be unchanged");
}
