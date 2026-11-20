use {
    anchor_lang::prelude::*,
    anchor_lang::{solana_program::instruction::Instruction, InstructionData, ToAccountMetas},
    jhandi_munda::{
        state::ProgramConfig, CONFIG_SEED, MUSD_DECIMALS, MUSD_MINT_AUTHORITY_SEED,
        MUSD_MINT_SEED, SPL_TOKEN_PROGRAM_ID,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

// SPL mint account layout (Mint::LEN): 0..4 mint authority COption flag,
// 4..36 mint authority key, 36..44 supply, 44 decimals, 46..50 freeze flag.
const MINT_SPACE: usize = 82;

#[test]
fn test_musd_mint_init() {
    let program_id = jhandi_munda::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/jhandi_munda.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let (config, config_bump) = Pubkey::find_program_address(&[CONFIG_SEED], &program_id);
    let (musd_mint, _) = Pubkey::find_program_address(&[MUSD_MINT_SEED], &program_id);
    let (musd_authority, authority_bump) =
        Pubkey::find_program_address(&[MUSD_MINT_AUTHORITY_SEED], &program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &jhandi_munda::instruction::Initialize {}.data(),
        jhandi_munda::accounts::Initialize {
            config,
            musd_mint,
            musd_mint_authority: musd_authority,
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

    // The MUSD mint exists, is owned by the SPL token program, has 6
    // decimals (= USDC, so 1 MUSD = 1_000_000 units, our $1 stand-in).
    let mint_acc = svm.get_account(&musd_mint).expect("MUSD mint must exist");
    assert_eq!(mint_acc.owner, SPL_TOKEN_PROGRAM_ID);
    assert_eq!(mint_acc.data.len(), MINT_SPACE);
    assert_eq!(mint_acc.data[44], MUSD_DECIMALS, "MUSD must have 6 decimals");

    // Mint authority is the expected program PDA, no keypair involved.
    assert_eq!(&mint_acc.data[0..4], &[1, 0, 0, 0], "mint authority not set");
    let authority_bytes: [u8; 32] = mint_acc.data[4..36].try_into().unwrap();
    assert_eq!(Pubkey::from(authority_bytes), musd_authority);

    // The config account stores the mint address (plus the PDA bumps).
    let config_acc = svm.get_account(&config).expect("config must exist");
    let cfg = ProgramConfig::try_from_slice(&config_acc.data[8..]).unwrap();
    assert_eq!(cfg.musd_mint, musd_mint);
    assert_eq!(cfg.bump, config_bump);
    assert_eq!(cfg.musd_bump, authority_bump);
}
