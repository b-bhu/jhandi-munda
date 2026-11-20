use {
    anchor_lang::{
        prelude::*, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
    },
    jhandi_munda::{
        state::{Bet, Round, RoundStatus},
        BET_SEED, CONFIG_SEED, FAUCET_RECORD_SEED, MUSD_MINT_AUTHORITY_SEED, MUSD_MINT_SEED,
        POOL_SEED, POOL_VAULT_SEED, ROUND_SEED, SHARE_MINT_SEED, SPL_TOKEN_PROGRAM_ID, TABLE_SEED,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const BASE_TIME: i64 = 1_700_000_000;
const SYS: Pubkey = anchor_lang::solana_program::system_program::ID;

// Everything a table test needs: the program is set up, the pool and table
// exist, and the round is idle. The admin pays for setup.
struct Table {
    svm: LiteSVM,
    admin: Keypair,
    config: Pubkey,
    musd_mint: Pubkey,
    mint_auth: Pubkey,
    pool: Pubkey,
    vault: Pubkey,
    table: Pubkey,
    round: Pubkey,
}

fn pda(seeds: &[&[u8]]) -> Pubkey {
    Pubkey::find_program_address(seeds, &jhandi_munda::id()).0
}

// Signs one instruction with `signer` and a fresh blockhash.
fn send(svm: &mut LiteSVM, signer: &Keypair, ix: Instruction) -> TransactionResult {
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&signer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[signer]).unwrap();
    svm.send_transaction(tx)
}

fn ix<A: ToAccountMetas, D: InstructionData>(accounts: A, data: D) -> Instruction {
    Instruction::new_with_bytes(jhandi_munda::id(), &data.data(), accounts.to_account_metas(None))
}

fn token_amount(svm: &LiteSVM, account: &Pubkey) -> u64 {
    let data = svm.get_account(account).unwrap().data;
    u64::from_le_bytes(data[64..72].try_into().unwrap())
}

fn setup() -> Table {
    let program_id = jhandi_munda::id();
    let admin = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/jhandi_munda.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = BASE_TIME;
    svm.set_sysvar::<Clock>(&clock);

    let t = Table {
        config: pda(&[CONFIG_SEED]),
        musd_mint: pda(&[MUSD_MINT_SEED]),
        mint_auth: pda(&[MUSD_MINT_AUTHORITY_SEED]),
        pool: pda(&[POOL_SEED]),
        vault: pda(&[POOL_VAULT_SEED]),
        table: pda(&[TABLE_SEED]),
        round: pda(&[ROUND_SEED]),
        svm,
        admin,
    };
    let (a, c, m, ma) = (&t.admin, t.config, t.musd_mint, t.mint_auth);
    let mut svm = t.svm;
    let steps = [
        ix(
            jhandi_munda::accounts::Initialize {
                config: c,
                musd_mint: m,
                musd_mint_authority: ma,
                payer: a.pubkey(),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::Initialize {},
        ),
        ix(
            jhandi_munda::accounts::InitializePool {
                config: c,
                pool: t.pool,
                share_mint: pda(&[SHARE_MINT_SEED]),
                vault: t.vault,
                musd_mint: m,
                payer: a.pubkey(),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::InitializePool {},
        ),
        ix(
            jhandi_munda::accounts::InitializeTable {
                table: t.table,
                round: t.round,
                payer: a.pubkey(),
                system_program: SYS,
            },
            jhandi_munda::instruction::InitializeTable {},
        ),
    ];
    for step in steps {
        let res = send(&mut svm, a, step);
        assert!(res.is_ok(), "setup failed: {:?}", res.err());
    }
    Table { svm, ..t }
}

// A new wallet that claims the faucet (100 MUSD), so it can bet.
fn new_player(t: &mut Table) -> Keypair {
    let wallet = Keypair::new();
    t.svm.airdrop(&wallet.pubkey(), 1_000_000_000).unwrap();
    let ata = jhandi_munda::musd_ata(&wallet.pubkey(), &t.musd_mint);
    let record = pda(&[FAUCET_RECORD_SEED, wallet.pubkey().as_ref()]);
    let res = send(
        &mut t.svm,
        &wallet,
        ix(
            jhandi_munda::accounts::MusdFaucet {
                config: t.config,
                musd_mint: t.musd_mint,
                musd_mint_authority: t.mint_auth,
                faucet_record: record,
                wallet: wallet.pubkey(),
                destination: ata,
                associated_token_program: jhandi_munda::ATA_PROGRAM_ID,
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::MusdFaucet {},
        ),
    );
    assert!(res.is_ok(), "faucet failed: {:?}", res.err());
    wallet
}

// Places one bet on die face `symbol` for `wallet` in round 0.
fn place(t: &mut Table, wallet: &Keypair, symbol: u8, amount: u64) -> TransactionResult {
    let bet = pda(&[BET_SEED, &0u64.to_le_bytes(), wallet.pubkey().as_ref(), &[symbol]]);
    let ata = jhandi_munda::musd_ata(&wallet.pubkey(), &t.musd_mint);
    send(
        &mut t.svm,
        wallet,
        ix(
            jhandi_munda::accounts::PlaceBet {
                table: t.table,
                round: t.round,
                bet,
                pool: t.pool,
                vault: t.vault,
                bettor: wallet.pubkey(),
                bettor_ata: ata,
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::PlaceBet { symbol, amount },
        ),
    )
}

fn round(t: &Table) -> Round {
    let data = t.svm.get_account(&t.round).unwrap().data;
    Round::try_deserialize(&mut data.as_slice()).unwrap()
}

fn bet(t: &Table, wallet: &Keypair, symbol: u8) -> Bet {
    let key = pda(&[BET_SEED, &0u64.to_le_bytes(), wallet.pubkey().as_ref(), &[symbol]]);
    let data = t.svm.get_account(&key).expect("bet account must exist").data;
    Bet::try_deserialize(&mut data.as_slice()).unwrap()
}

#[test]
fn test_first_bet_opens_round() {
    let mut t = setup();
    let player = new_player(&mut t);

    // A $0.50 bet is rejected: below the $1 minimum (D3). Nothing moves.
    assert!(place(&mut t, &player, 1, 500_000).is_err(), "a $0.50 bet must be rejected");
    assert_eq!(round(&t).status, RoundStatus::Idle, "a rejected bet must not open the round");

    // A $10 bet on die face 3 opens the round (D31) and starts the clock.
    let res = place(&mut t, &player, 3, 10_000_000);
    assert!(res.is_ok(), "$10 bet failed: {:?}", res.err());

    let r = round(&t);
    assert_eq!(r.status, RoundStatus::Open, "the first bet must open the round");
    assert_eq!(r.opened_at, BASE_TIME, "opened_at must be the clock time");
    assert_eq!(r.players, vec![player.pubkey()]);

    let b = bet(&t, &player, 3);
    assert_eq!(b.player, player.pubkey());
    assert_eq!(b.amount, 10_000_000, "bet must record $10");
    assert_eq!(b.symbol, 3);

    // The stake left the bettor's ATA and landed in the pool vault.
    assert_eq!(token_amount(&t.svm, &t.vault), 10_000_000);
    let ata = jhandi_munda::musd_ata(&player.pubkey(), &t.musd_mint);
    assert_eq!(token_amount(&t.svm, &ata), 90_000_000);
}

#[test]
fn test_two_players_join() {
    let mut t = setup();
    let alice = new_player(&mut t);
    let bob = new_player(&mut t);

    // Alice opens the round; Bob joins and bets on two faces (D10).
    assert!(place(&mut t, &alice, 3, 10_000_000).is_ok());
    assert!(place(&mut t, &bob, 5, 5_000_000).is_ok());
    assert!(place(&mut t, &bob, 6, 2_000_000).is_ok(), "a second face for Bob must be accepted");

    let r = round(&t);
    assert_eq!(r.status, RoundStatus::Open);
    assert_eq!(r.opened_at, BASE_TIME, "joining must not move opened_at");
    assert_eq!(r.players, vec![alice.pubkey(), bob.pubkey()]);
    assert_eq!(bet(&t, &alice, 3).amount, 10_000_000);
    assert_eq!(bet(&t, &bob, 5).amount, 5_000_000);
    assert_eq!(bet(&t, &bob, 6).amount, 2_000_000);
    assert_eq!(token_amount(&t.svm, &t.vault), 17_000_000, "all three stakes reach the vault");
}

#[test]
fn test_eleventh_player_rejected() {
    let mut t = setup();
    let players: Vec<Keypair> = (0..10).map(|_| new_player(&mut t)).collect();
    for p in &players {
        assert!(place(&mut t, p, 1, 1_000_000).is_ok());
    }
    assert_eq!(round(&t).players.len(), 10);

    // An 11th distinct wallet is turned away, and nothing is taken from it.
    let eleventh = new_player(&mut t);
    assert!(place(&mut t, &eleventh, 1, 1_000_000).is_err(), "the 11th player must be rejected");
    assert_eq!(round(&t).players.len(), 10, "a rejected player takes no seat");

    // A seated player can still bet another face: joining is not the same as
    // a new seat (D10).
    assert!(place(&mut t, &players[0], 2, 1_000_000).is_ok());
    assert_eq!(round(&t).players.len(), 10);
}

// Clock warp: rewrites the Clock sysvar that `Clock::get()` reads inside the
// program. Only `unix_timestamp` moves; the timer never looks at slots (D8).
fn set_time(t: &mut Table, unix_timestamp: i64) {
    let mut clock = t.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    t.svm.set_sysvar::<Clock>(&clock);
}

#[test]
fn test_bet_rejected_after_90s() {
    let mut t = setup();
    let alice = new_player(&mut t);
    let bob = new_player(&mut t);
    let carol = new_player(&mut t);

    // Alice opens the round at BASE_TIME.
    assert!(place(&mut t, &alice, 3, 10_000_000).is_ok());

    // Bob bets at 89 s, one second before the deadline: it still counts.
    set_time(&mut t, BASE_TIME + 89);
    assert!(place(&mut t, &bob, 5, 1_000_000).is_ok(), "a bet at 89 s must count");

    // Carol bets at 90 s, the deadline: the window is closed.
    set_time(&mut t, BASE_TIME + 90);
    assert!(place(&mut t, &carol, 5, 1_000_000).is_err(), "a bet at 90 s must be rejected");
    assert_eq!(round(&t).players, vec![alice.pubkey(), bob.pubkey()]);
    assert_eq!(round(&t).opened_at, BASE_TIME, "closing must not move opened_at");
}

// The VRF program is not loaded in LiteSVM. Stand in for it with
// tests/fixtures/vrf_stub.so (source in tests/fixtures/vrf_stub), a BPF program
// that accepts every call. The request CPI then runs for real, and the request
// accounts and every check in the program stay the real ones; only the
// oracle's own work is mocked.
const VRF_PROGRAM: &str = "Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz";
const DEFAULT_QUEUE_ID: &str = "Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh";
const SLOT_HASHES: &str = "SysvarS1otHashes111111111111111111111111111";

fn stub_vrf(t: &mut Table) {
    let bytes = include_bytes!("fixtures/vrf_stub.so");
    t.svm.add_program(VRF_PROGRAM.parse::<Pubkey>().unwrap(), bytes).unwrap();
}

// Requests a VRF roll for the open round, signed by `payer`.
fn request_roll(t: &mut Table, payer: &Keypair) -> TransactionResult {
    send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::RequestRoll {
                table: t.table,
                round: t.round,
                payer: payer.pubkey(),
                oracle_queue: DEFAULT_QUEUE_ID.parse().unwrap(),
                program_identity: pda(&[b"identity"]),
                vrf_program: VRF_PROGRAM.parse().unwrap(),
                slot_hashes: SLOT_HASHES.parse().unwrap(),
                system_program: SYS,
            },
            jhandi_munda::instruction::RequestRoll {},
        ),
    )
}

#[test]
fn test_request_vrf_after_window() {
    let mut t = setup();
    stub_vrf(&mut t);
    let alice = new_player(&mut t);
    let bob = new_player(&mut t);

    // Alice opens the round at BASE_TIME.
    assert!(place(&mut t, &alice, 3, 10_000_000).is_ok());

    // Still inside the window (89 s): the roll request is rejected.
    set_time(&mut t, BASE_TIME + 89);
    assert!(request_roll(&mut t, &bob).is_err(), "a roll inside the window must be rejected");
    assert_eq!(round(&t).status, RoundStatus::Open);
    assert_eq!(round(&t).request_id, [0u8; 32], "a rejected request stores nothing");

    // Window closed (90 s): the request goes out and the round stores its id.
    set_time(&mut t, BASE_TIME + 90);
    let res = request_roll(&mut t, &bob);
    assert!(res.is_ok(), "roll after the window failed: {:?}", res.err());
    let r = round(&t);
    assert_eq!(r.status, RoundStatus::Rolling);
    assert_ne!(r.request_id, [0u8; 32], "the pending request id must be stored");

    // A second request while one is pending is rejected, and so is a bet.
    assert!(request_roll(&mut t, &alice).is_err(), "a second pending roll must be rejected");
    assert!(place(&mut t, &bob, 5, 1_000_000).is_err(), "no bets once the roll is pending");
}
