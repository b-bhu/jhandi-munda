use {
    anchor_lang::{
        prelude::*,
        solana_program::instruction::Instruction,
        InstructionData, ToAccountMetas,
    },
    jhandi_munda::{state::{Round, RoundStatus, TableConfig}, ROUND_SEED, TABLE_SEED},
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

#[test]
fn test_round_account_init() {
    let program_id = jhandi_munda::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/jhandi_munda.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    // The config first: its admin is the payer, who is the table authority (#111).
    init_config(&mut svm, &payer);

    let (table, table_bump) = Pubkey::find_program_address(&[TABLE_SEED], &program_id);
    let (round, round_bump) = Pubkey::find_program_address(&[ROUND_SEED], &program_id);
    let config = Pubkey::find_program_address(&[jhandi_munda::CONFIG_SEED], &program_id).0;

    let instruction = Instruction::new_with_bytes(
        program_id,
        &jhandi_munda::instruction::InitializeTable {}.data(),
        jhandi_munda::accounts::InitializeTable {
            config,
            table,
            round,
            payer: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();
    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "initialize_table failed: {:?}", res.err());

    // Table config: 90 s rounds (D8), $1 minimum bet (D3), max bet off (D6).
    let table_acc = svm.get_account(&table).expect("table PDA must exist");
    assert_eq!(table_acc.owner, program_id);
    let mut data: &[u8] = &table_acc.data;
    let cfg = TableConfig::try_deserialize(&mut data).expect("table config must decode");
    assert_eq!(cfg.round_length_secs, 90, "round length must be 90 seconds");
    assert_eq!(cfg.min_bet, 1_000_000, "min bet must be $1 MUSD");
    assert!(!cfg.max_bet_enabled, "max bet must be off by default on devnet");
    assert_eq!(cfg.bump, table_bump);

    // Round account: vacant and idle, no players, not yet opened (D5).
    let round_acc = svm.get_account(&round).expect("round PDA must exist");
    assert_eq!(round_acc.owner, program_id);
    let mut data: &[u8] = &round_acc.data;
    let r = Round::try_deserialize(&mut data).expect("round must decode");
    assert_eq!(r.status, RoundStatus::Idle, "a new round must start idle");
    assert_eq!(r.opened_at, 0, "an idle round has no open timestamp");
    assert!(r.players.is_empty(), "an idle round has no players");
    assert_eq!(r.bump, round_bump);
}

// Runs `initialize` (the config, its MUSD mint and authority) with `payer` as the admin.
fn init_config(svm: &mut LiteSVM, payer: &Keypair) {
    let program_id = jhandi_munda::id();
    let config = Pubkey::find_program_address(&[jhandi_munda::CONFIG_SEED], &program_id).0;
    let musd_mint = Pubkey::find_program_address(&[jhandi_munda::MUSD_MINT_SEED], &program_id).0;
    let musd_mint_authority = Pubkey::find_program_address(&[jhandi_munda::MUSD_MINT_AUTHORITY_SEED], &program_id).0;
    let instruction = Instruction::new_with_bytes(
        program_id,
        &jhandi_munda::instruction::Initialize {}.data(),
        jhandi_munda::accounts::Initialize {
            config,
            musd_mint,
            musd_mint_authority,
            payer: payer.pubkey(),
            token_program: jhandi_munda::SPL_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "initialize failed: {:?}", res.err());
}

// #111: only the table authority (the config admin, D40) can set up the table and the pool.
#[test]
fn test_only_admin_sets_up_table_and_pool() {
    let program_id = jhandi_munda::id();
    let admin = Keypair::new();
    let stranger = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/jhandi_munda.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&stranger.pubkey(), 10_000_000_000).unwrap();
    init_config(&mut svm, &admin);

    let pda = |seeds: &[&[u8]]| Pubkey::find_program_address(seeds, &program_id).0;
    let send_as = |svm: &mut LiteSVM, signer: &Keypair, ix: Instruction| {
        svm.expire_blockhash();
        let blockhash = svm.latest_blockhash();
        let msg = Message::new_with_blockhash(&[ix], Some(&signer.pubkey()), &blockhash);
        let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[signer]).unwrap();
        svm.send_transaction(tx)
    };
    let table_ix = |signer: &Keypair| Instruction::new_with_bytes(
        program_id,
        &jhandi_munda::instruction::InitializeTable {}.data(),
        jhandi_munda::accounts::InitializeTable {
            config: pda(&[jhandi_munda::CONFIG_SEED]),
            table: pda(&[TABLE_SEED]),
            round: pda(&[ROUND_SEED]),
            payer: signer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );
    let pool_ix = |signer: &Keypair| Instruction::new_with_bytes(
        program_id,
        &jhandi_munda::instruction::InitializePool {}.data(),
        jhandi_munda::accounts::InitializePool {
            config: pda(&[jhandi_munda::CONFIG_SEED]),
            pool: pda(&[jhandi_munda::POOL_SEED]),
            share_mint: pda(&[jhandi_munda::SHARE_MINT_SEED]),
            vault: pda(&[jhandi_munda::POOL_VAULT_SEED]),
            musd_mint: pda(&[jhandi_munda::MUSD_MINT_SEED]),
            payer: signer.pubkey(),
            token_program: jhandi_munda::SPL_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    // A stranger is refused for both; nothing is created.
    let res = send_as(&mut svm, &stranger, table_ix(&stranger));
    assert!(res.is_err(), "a stranger cannot set up the table");
    assert!(svm.get_account(&pda(&[TABLE_SEED])).is_none(), "the refused call created nothing");
    let res = send_as(&mut svm, &stranger, pool_ix(&stranger));
    assert!(res.is_err(), "a stranger cannot set up the pool");
    assert!(svm.get_account(&pda(&[jhandi_munda::POOL_SEED])).is_none());

    // The admin can.
    let res = send_as(&mut svm, &admin, table_ix(&admin));
    assert!(res.is_ok(), "the admin sets up the table: {:?}", res.err());
    let res = send_as(&mut svm, &admin, pool_ix(&admin));
    assert!(res.is_ok(), "the admin sets up the pool: {:?}", res.err());
}
