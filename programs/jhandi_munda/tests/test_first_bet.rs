use {
    anchor_lang::{
        prelude::*, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
    },
    jhandi_munda::{
        state::{Bet, Round, RoundStatus},
        BET_SEED, CONFIG_SEED, MUSD_MINT_AUTHORITY_SEED, MUSD_MINT_SEED, POOL_SEED,
        POOL_VAULT_SEED, ROUND_SEED, SHARE_MINT_SEED, SPL_TOKEN_PROGRAM_ID, TABLE_SEED,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const BASE_TIME: i64 = 1_700_000_000;

// Sends one instruction, signed by `payer`, with a fresh blockhash.
fn send(svm: &mut LiteSVM, payer: &Keypair, ix: Instruction) -> TransactionResult {
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    svm.send_transaction(tx)
}

fn ix<A: ToAccountMetas, D: InstructionData>(accounts: A, data: D) -> Instruction {
    Instruction::new_with_bytes(jhandi_munda::id(), &data.data(), accounts.to_account_metas(None))
}

fn token_amount(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let data = svm.get_account(account).unwrap().data;
    u64::from_le_bytes(data[64..72].try_into().unwrap())
}

#[test]
fn test_first_bet_opens_round() {
    let program_id = jhandi_munda::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/jhandi_munda.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = BASE_TIME;
    svm.set_sysvar::<Clock>(&clock);
    let sys = anchor_lang::solana_program::system_program::ID;
    let pda = |seeds: &[&[u8]]| Pubkey::find_program_address(seeds, &program_id).0;

    let config = pda(&[CONFIG_SEED]);
    let musd_mint = pda(&[MUSD_MINT_SEED]);
    let mint_auth = pda(&[MUSD_MINT_AUTHORITY_SEED]);
    let pool = pda(&[POOL_SEED]);
    let vault = pda(&[POOL_VAULT_SEED]);
    let share_mint = pda(&[SHARE_MINT_SEED]);
    let table = pda(&[TABLE_SEED]);
    let round = pda(&[ROUND_SEED]);
    let bet = pda(&[BET_SEED, payer.pubkey().as_ref()]);
    let wallet_ata = jhandi_munda::musd_ata(&payer.pubkey(), &musd_mint);
    let faucet_record = pda(&[jhandi_munda::FAUCET_RECORD_SEED, payer.pubkey().as_ref()]);

    // Setup: config + MUSD mint, empty pool, table and idle round, and a faucet
    // claim so the bettor holds MUSD (100 MUSD).
    let res = send(
        &mut svm,
        &payer,
        ix(
            jhandi_munda::accounts::Initialize {
                config,
                musd_mint,
                musd_mint_authority: mint_auth,
                payer: payer.pubkey(),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: sys,
            },
            jhandi_munda::instruction::Initialize {},
        ),
    );
    assert!(res.is_ok(), "initialize failed: {:?}", res.err());
    let res = send(
        &mut svm,
        &payer,
        ix(
            jhandi_munda::accounts::InitializePool {
                config,
                pool,
                share_mint,
                vault,
                musd_mint,
                payer: payer.pubkey(),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: sys,
            },
            jhandi_munda::instruction::InitializePool {},
        ),
    );
    assert!(res.is_ok(), "initialize_pool failed: {:?}", res.err());
    let res = send(
        &mut svm,
        &payer,
        ix(
            jhandi_munda::accounts::InitializeTable {
                table,
                round,
                payer: payer.pubkey(),
                system_program: sys,
            },
            jhandi_munda::instruction::InitializeTable {},
        ),
    );
    assert!(res.is_ok(), "initialize_table failed: {:?}", res.err());
    let res = send(
        &mut svm,
        &payer,
        ix(
            jhandi_munda::accounts::MusdFaucet {
                config,
                musd_mint,
                musd_mint_authority: mint_auth,
                faucet_record,
                wallet: payer.pubkey(),
                destination: wallet_ata,
                associated_token_program: jhandi_munda::ATA_PROGRAM_ID,
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: sys,
            },
            jhandi_munda::instruction::MusdFaucet {},
        ),
    );
    assert!(res.is_ok(), "faucet failed: {:?}", res.err());

    let place = |symbol: u8, amount: u64| {
        ix(
            jhandi_munda::accounts::PlaceBet {
                table,
                round,
                bet,
                pool,
                vault,
                bettor: payer.pubkey(),
                bettor_ata: wallet_ata,
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: sys,
            },
            jhandi_munda::instruction::PlaceBet { symbol, amount },
        )
    };

    // A $0.50 bet is rejected: below the $1 minimum (D3). Nothing moves.
    let res = send(&mut svm, &payer, place(1, 500_000));
    assert!(res.is_err(), "a $0.50 bet must be rejected");
    let r = Round::try_deserialize(&mut svm.get_account(&round).unwrap().data.as_slice()).unwrap();
    assert_eq!(r.status, RoundStatus::Idle, "a rejected bet must not open the round");

    // A $10 bet on die face 3 opens the round (D31) and starts the clock.
    let res = send(&mut svm, &payer, place(3, 10_000_000));
    assert!(res.is_ok(), "$10 bet failed: {:?}", res.err());

    let r = Round::try_deserialize(&mut svm.get_account(&round).unwrap().data.as_slice()).unwrap();
    assert_eq!(r.status, RoundStatus::Open, "the first bet must open the round");
    assert_eq!(r.opened_at, BASE_TIME, "opened_at must be the clock time");
    assert_eq!(r.player_count, 1);

    let b = Bet::try_deserialize(&mut svm.get_account(&bet).unwrap().data.as_slice()).unwrap();
    assert_eq!(b.player, payer.pubkey());
    assert_eq!(b.amount, 10_000_000, "bet must record $10");
    assert_eq!(b.symbol, 3);

    // The stake left the bettor's ATA and landed in the pool vault.
    assert_eq!(token_amount(&svm, &vault), 10_000_000);
    assert_eq!(token_amount(&svm, &wallet_ata), 90_000_000);
}
