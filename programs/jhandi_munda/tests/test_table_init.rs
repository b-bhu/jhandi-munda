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

    let (table, table_bump) = Pubkey::find_program_address(&[TABLE_SEED], &program_id);
    let (round, round_bump) = Pubkey::find_program_address(&[ROUND_SEED], &program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &jhandi_munda::instruction::InitializeTable {}.data(),
        jhandi_munda::accounts::InitializeTable {
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
