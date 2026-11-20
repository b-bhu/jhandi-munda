use {
    anchor_lang::prelude::*,
    anchor_lang::{solana_program::instruction::Instruction, InstructionData, ToAccountMetas},
    jhandi_munda::{
        CONFIG_SEED, FAUCET_AMOUNT, MUSD_MINT_AUTHORITY_SEED, MUSD_MINT_SEED, SPL_TOKEN_PROGRAM_ID,
    },
    litesvm::LiteSVM,
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
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "initialize failed: {:?}", res.err());
    (svm, payer, config, musd_mint, musd_mint_authority)
}

// Calls the faucet for the payer wallet and asserts the result.
fn faucet(svm: &mut LiteSVM, setup: (&Keypair, &Pubkey, &Pubkey, &Pubkey, &Pubkey)) {
    let (payer, config, musd_mint, musd_mint_authority, destination) = setup;

    let instruction = Instruction::new_with_bytes(
        jhandi_munda::id(),
        &jhandi_munda::instruction::MusdFaucet {}.data(),
        jhandi_munda::accounts::MusdFaucet {
            config: *config,
            musd_mint: *musd_mint,
            musd_mint_authority: *musd_mint_authority,
            wallet: payer.pubkey(),
            destination: *destination,
            associated_token_program: jhandi_munda::ATA_PROGRAM_ID,
            token_program: SPL_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();

    let res = svm.send_transaction(tx);
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

#[test]
fn test_musd_faucet() {
    let (mut svm, payer, config, musd_mint, musd_mint_authority) = setup();
    let ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);

    // The test wallet starts at 0 MUSD: no token account yet.
    assert!(svm.get_account(&ata).is_none(), "wallet must start with 0 MUSD");

    // First faucet call: the wallet ends with exactly the documented amount.
    faucet(&mut svm, (&payer, &config, &musd_mint, &musd_mint_authority, &ata));
    let (amount, mint, owner) = ata_amount(&svm, &ata);
    assert_eq!(amount, FAUCET_AMOUNT, "first faucet call must mint 100 MUSD");
    assert_eq!(mint, musd_mint, "ATA must hold the MUSD mint");
    assert_eq!(owner, payer.pubkey(), "ATA must belong to the test wallet");
    assert_eq!(mint_supply(&svm, &musd_mint), FAUCET_AMOUNT, "mint supply must equal the minted total");

    // Second faucet call: the balance increases by the same amount again.
    faucet(&mut svm, (&payer, &config, &musd_mint, &musd_mint_authority, &ata));
    let (amount, _mint, _owner) = ata_amount(&svm, &ata);
    assert_eq!(amount, 2 * FAUCET_AMOUNT, "second faucet call must double the balance");
    assert_eq!(mint_supply(&svm, &musd_mint), 2 * FAUCET_AMOUNT, "mint supply must rise by the same total");
}
