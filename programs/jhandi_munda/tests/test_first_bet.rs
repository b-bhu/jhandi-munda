use {
    anchor_lang::{
        prelude::*,
        solana_program::instruction::{AccountMeta, Instruction},
        InstructionData, ToAccountMetas,
    },
    jhandi_munda::{
        state::{Bet, Claim, ClaimAsset, Round, RoundStatus, TableConfig},
        BET_SEED, CLAIM_SEED, CONFIG_SEED, INSURANCE_SEED, INSURANCE_VAULT_SEED, MUNDA_MINT_AUTHORITY_SEED,
        MUNDA_MINT_SEED, FAUCET_RECORD_SEED, MUSD_MINT_AUTHORITY_SEED, MUSD_MINT_SEED,
        POOL_SEED, POOL_VAULT_SEED, QUEUE_SEED, ROUND_SEED, SHARE_MINT_SEED, SPL_TOKEN_PROGRAM_ID, TABLE_SEED,
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
    // Signature checks off: the VRF oracle's signature is mocked (see send_callback).
    let mut svm = LiteSVM::new().with_sigverify(false);
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
                config: c,
                table: t.table,
                round: t.round,
                payer: a.pubkey(),
                system_program: SYS,
            },
            jhandi_munda::instruction::InitializeTable {},
        ),
        ix(
            jhandi_munda::accounts::InitQueue {
                queue: pda(&[QUEUE_SEED]),
                payer: a.pubkey(),
                system_program: SYS,
            },
            jhandi_munda::instruction::InitQueue {},
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
    let bet = bet_pda_r(round(t).round_id, wallet, symbol);
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

// Requests a VRF roll for the open round, signed by `payer`, on the base-layer queue.
fn request_roll(t: &mut Table, payer: &Keypair) -> TransactionResult {
    request_roll_on(t, payer, DEFAULT_QUEUE_ID.parse().unwrap())
}

// Requests a VRF roll on the given queue (Step 11.3 accepts the ephemeral queue).
fn request_roll_on(t: &mut Table, payer: &Keypair, queue: Pubkey) -> TransactionResult {
    send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::RequestRoll {
                table: t.table,
                round: t.round,
                payer: payer.pubkey(),
                oracle_queue: queue,
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

// Sends the oracle's callback. The scoped VRF identity is a PDA, so it cannot
// sign here: its signature slot is left empty, and the setup turns signature
// checks off so the program sees it as a signer. That is the only thing mocked.
// The accounts, the `#[vrf_callback]` check and the handler are real.
fn send_callback(t: &mut Table, payer: &Keypair, randomness: [u8; 32], request_id: [u8; 32]) -> TransactionResult {
    let identity = Pubkey::find_program_address(
        &[b"identity", jhandi_munda::id().as_ref()],
        &VRF_PROGRAM.parse().unwrap(),
    )
    .0;
    let instruction = ix(
        jhandi_munda::accounts::CallbackRollDice { vrf_program_identity: identity, round: t.round },
        jhandi_munda::instruction::CallbackRollDice { randomness, request_id },
    );
    t.svm.expire_blockhash();
    let blockhash = t.svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let signature = payer.sign_message(&msg.serialize());
    let tx = VersionedTransaction {
        signatures: vec![signature, Default::default()],
        message: VersionedMessage::Legacy(msg),
    };
    t.svm.send_transaction(tx)
}

// Counting bytes 0..=31: the six faces 1..=6 come from the first six bytes.
fn counting_randomness() -> [u8; 32] {
    core::array::from_fn(|i| i as u8)
}

// Opens a round with Alice, closes the window and requests the roll.
fn open_and_request(t: &mut Table) -> (Keypair, Keypair) {
    let alice = new_player(t);
    let bob = new_player(t);
    assert!(place(t, &alice, 3, 10_000_000).is_ok());
    set_time(t, BASE_TIME + 90);
    assert!(request_roll(t, &bob).is_ok());
    (alice, bob)
}

#[test]
fn test_vrf_callback_six_faces() {
    let mut t = setup();
    stub_vrf(&mut t);
    let (_alice, bob) = open_and_request(&mut t);
    let request_id = round(&t).request_id;

    let res = send_callback(&mut t, &bob, counting_randomness(), request_id);
    assert!(res.is_ok(), "callback failed: {:?}", res.err());

    let r = round(&t);
    assert_eq!(r.status, RoundStatus::Rolled);
    assert_eq!(r.faces, [1, 2, 3, 4, 5, 6], "six independent faces from six bytes");
    assert_eq!(r.request_id, [0u8; 32], "the pending request is cleared");
}

#[test]
fn test_unexpected_vrf_callback_rejected() {
    // No roll requested yet: a callback has nothing to answer.
    let mut fresh = setup();
    stub_vrf(&mut fresh);
    let player = new_player(&mut fresh);
    assert!(send_callback(&mut fresh, &player, counting_randomness(), [0u8; 32]).is_err());

    let mut t = setup();
    stub_vrf(&mut t);
    let (_alice, bob) = open_and_request(&mut t);
    let request_id = round(&t).request_id;

    // A callback for some other request (another round) is rejected.
    assert!(send_callback(&mut t, &bob, counting_randomness(), [9u8; 32]).is_err());
    assert_eq!(round(&t).status, RoundStatus::Rolling, "a rejected callback changes nothing");

    // The right callback lands once; a repeat after fulfilment is rejected.
    assert!(send_callback(&mut t, &bob, counting_randomness(), request_id).is_ok());
    assert!(send_callback(&mut t, &bob, counting_randomness(), request_id).is_err());
    assert_eq!(round(&t).status, RoundStatus::Rolled);
}

// Settles some of round 0's bets: the given bet accounts are remaining accounts.
fn settle_bets(t: &mut Table, signer: &Keypair, bets: &[Pubkey]) -> TransactionResult {
    let mut instruction = ix(
        jhandi_munda::accounts::SettleBets { table: t.table, round: t.round },
        jhandi_munda::instruction::SettleBets {},
    );
    for bet in bets {
        instruction.accounts.push(AccountMeta::new(*bet, false));
    }
    send(&mut t.svm, signer, instruction)
}

// Closes round 0 once every bet is settled.
fn finish_round(t: &mut Table, signer: &Keypair) -> TransactionResult {
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::FinishRound { round: t.round },
            jhandi_munda::instruction::FinishRound {},
        ),
    )
}

fn bet_pda(wallet: &Keypair, symbol: u8) -> Pubkey {
    bet_pda_r(0, wallet, symbol)
}

// A bet's PDA in a given round.
fn bet_pda_r(round_id: u64, wallet: &Keypair, symbol: u8) -> Pubkey {
    pda(&[BET_SEED, &round_id.to_le_bytes(), wallet.pubkey().as_ref(), &[symbol]])
}

#[test]
fn test_settle_known_dice() {
    let mut t = setup();
    stub_vrf(&mut t);
    let alice = new_player(&mut t);
    let bob = new_player(&mut t);
    let carol = new_player(&mut t);

    // Three bets: Alice $10 on face 2, Bob $1 on face 4, Carol $1 on face 2.
    assert!(place(&mut t, &alice, 2, 10_000_000).is_ok());
    assert!(place(&mut t, &bob, 4, 1_000_000).is_ok());
    assert!(place(&mut t, &carol, 2, 1_000_000).is_ok());
    let bets = [bet_pda(&alice, 2), bet_pda(&bob, 4), bet_pda(&carol, 2)];

    // Settle before the faces exist (round still open) is rejected.
    assert!(settle_bets(&mut t, &bob, &bets).is_err(), "no settle before the window closes");

    set_time(&mut t, BASE_TIME + 90);
    assert!(request_roll(&mut t, &bob).is_ok());
    assert!(settle_bets(&mut t, &bob, &bets).is_err(), "no settle while the roll is pending");

    // Known dice: bytes 1,1,4,5,0,2 map to faces 2,2,5,6,1,3 (face = byte % 6 + 1).
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &bob, randomness, request_id).is_ok());
    assert_eq!(round(&t).faces, [2, 2, 5, 6, 1, 3]);

    // Settle with every bet: a 2-match win, a loss and a 2-match win.
    let res = settle_bets(&mut t, &bob, &bets);
    assert!(res.is_ok(), "settle failed: {:?}", res.err());
    assert!(finish_round(&mut t, &bob).is_ok(), "finish after full settle");

    // Alice $10 on face 2 (two matches, 3x): owed $29.60, fee $0.40.
    // Carol $1 on face 2 (two matches, 3x): owed $2.96, fee $0.04.
    // Bob $1 on face 4 (no matches): loses, owed $0, no fee.
    assert_eq!(bet(&t, &alice, 2).owed, 29_600_000);
    assert_eq!(bet(&t, &bob, 4).owed, 0);
    assert_eq!(bet(&t, &carol, 2).owed, 2_960_000);
    assert!(bet(&t, &alice, 2).settled && bet(&t, &bob, 4).settled && bet(&t, &carol, 2).settled);

    // Fees: 0.44 in total, split 20/10/70 per win (D15).
    let table = TableConfig::try_deserialize(
        &mut t.svm.get_account(&t.table).unwrap().data.as_slice(),
    )
    .unwrap();
    assert_eq!(table.fee_treasury, 88_000, "20% pot: 80,000 + 8,000");
    assert_eq!(table.fee_devs, 44_000, "10% pot: 40,000 + 4,000");
    assert_eq!(table.fee_stakers, 308_000, "70% pot: 280,000 + 28,000");

    // Nothing moved: all $12 of stakes, winners and losers alike, stay in the pool vault.
    assert_eq!(token_amount(&t.svm, &t.vault), 12_000_000);

    // The round is idle again for the next round.
    let r = round(&t);
    assert_eq!(r.status, RoundStatus::Idle);
    assert_eq!(r.round_id, 1, "settle bumps the round id");
    assert_eq!(r.bet_count, 0);

    // The round is closed: settling or finishing it again is rejected.
    assert!(settle_bets(&mut t, &bob, &bets).is_err(), "a second settle must be rejected");
    assert!(finish_round(&mut t, &bob).is_err(), "a second finish must be rejected");
}

#[test]
fn test_settle_full_table_in_chunks() {
    let mut t = setup();
    stub_vrf(&mut t);
    // A full table: 10 players, each on all six faces, $1 each = 60 bets.
    let players: Vec<Keypair> = (0..10).map(|_| new_player(&mut t)).collect();
    for p in &players {
        for symbol in 1..=6u8 {
            assert!(place(&mut t, p, symbol, 1_000_000).is_ok());
        }
    }
    assert_eq!(round(&t).bet_count, 60);
    let bets: Vec<Pubkey> = players
        .iter()
        .flat_map(|p| (1..=6u8).map(move |s| bet_pda(p, s)))
        .collect();

    set_time(&mut t, BASE_TIME + 90);
    assert!(request_roll(&mut t, &players[0]).is_ok());
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &players[0], counting_randomness(), request_id).is_ok());

    // Nothing is settled yet: finishing the round is rejected.
    assert!(finish_round(&mut t, &players[0]).is_err());

    // Six chunks of ten bets. Each chunk adds to the settled count.
    let chunks: Vec<&[Pubkey]> = bets.chunks(10).collect();
    assert!(settle_bets(&mut t, &players[0], chunks[0]).is_ok());
    assert_eq!(round(&t).settled_count, 10);
    assert!(finish_round(&mut t, &players[0]).is_err(), "finish before every bet is settled");

    // Settling the same bets again is rejected, and the count does not move.
    assert!(settle_bets(&mut t, &players[0], chunks[0]).is_err(), "a bet is settled once");
    assert_eq!(round(&t).settled_count, 10);

    for chunk in &chunks[1..] {
        assert!(settle_bets(&mut t, &players[0], chunk).is_ok());
    }
    assert_eq!(round(&t).settled_count, 60);
    assert!(finish_round(&mut t, &players[0]).is_ok());

    let r = round(&t);
    assert_eq!(r.status, RoundStatus::Idle);
    assert_eq!(r.round_id, 1);
    assert_eq!(r.bet_count, 0);
    assert_eq!(r.settled_count, 0);
}

fn claim(t: &Table, bet: &Pubkey) -> Claim {
    let data = t.svm.get_account(&pda(&[CLAIM_SEED, bet.as_ref()])).expect("claim must exist").data;
    Claim::try_deserialize(&mut data.as_slice()).unwrap()
}

fn open_claim(t: &mut Table, payer: &Keypair, bet: &Pubkey) -> TransactionResult {
    send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::OpenClaim {
                bet: *bet,
                claim: pda(&[CLAIM_SEED, bet.as_ref()]),
                payer: payer.pubkey(),
                system_program: SYS,
            },
            jhandi_munda::instruction::OpenClaim {},
        ),
    )
}

#[test]
fn test_open_claim_from_settled_bet() {
    let mut t = setup();
    stub_vrf(&mut t);
    let alice = new_player(&mut t);
    let bob = new_player(&mut t);
    assert!(place(&mut t, &alice, 2, 10_000_000).is_ok());
    assert!(place(&mut t, &bob, 4, 1_000_000).is_ok());
    set_time(&mut t, BASE_TIME + 90);
    assert!(request_roll(&mut t, &bob).is_ok());
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &bob, randomness, request_id).is_ok());

    // Settle at +100 s: settled_at is the settle time, so unlock is +190 s.
    set_time(&mut t, BASE_TIME + 100);
    let alice_bet = bet_pda(&alice, 2);
    let bob_bet = bet_pda(&bob, 4);
    assert!(settle_bets(&mut t, &bob, &[alice_bet, bob_bet]).is_ok());

    // Alice's win opens a claim: $29.60 MUSD, unlock 90 s after settle, 5% take-now cut.
    assert!(open_claim(&mut t, &bob, &alice_bet).is_ok());
    let c = claim(&t, &alice_bet);
    assert_eq!(c.bet, alice_bet);
    assert_eq!(c.player, alice.pubkey());
    assert_eq!(c.asset, ClaimAsset::Musd);
    assert_eq!(c.amount, 29_600_000);
    assert_eq!(c.unlock_at, BASE_TIME + 190);
    assert_eq!(c.early_exit_bps, 500);

    // A second claim for the same bet is rejected.
    assert!(open_claim(&mut t, &bob, &alice_bet).is_err(), "one claim per bet");
    // Bob's loss opens a MUNDA claim: 1 MUNDA per 1 MUSD lost, unlock 24 h after settle.
    assert!(open_claim(&mut t, &bob, &bob_bet).is_ok());
    let l = claim(&t, &bob_bet);
    assert_eq!(l.asset, ClaimAsset::Munda);
    assert_eq!(l.amount, 1_000_000);
    assert_eq!(l.unlock_at, BASE_TIME + 100 + 86_400);
    assert_eq!(l.early_exit_bps, 1000);
}

// Pool deposit (the existing `deposit`): the pool needs MUSD to pay winners.
fn deposit(t: &mut Table, funder: &Keypair, amount: u64) {
    let share_mint = pda(&[SHARE_MINT_SEED]);
    let res = send(
        &mut t.svm,
        funder,
        ix(
            jhandi_munda::accounts::Deposit {
                pool: t.pool,
                share_mint,
                vault: t.vault,
                depositor: funder.pubkey(),
                depositor_ata: jhandi_munda::musd_ata(&funder.pubkey(), &t.musd_mint),
                depositor_shares: jhandi_munda::share_ata(&funder.pubkey(), &share_mint),
                insurance_vault: Pubkey::find_program_address(&[jhandi_munda::INSURANCE_VAULT_SEED], &jhandi_munda::id()).0,
                queue: Pubkey::find_program_address(&[jhandi_munda::QUEUE_SEED], &jhandi_munda::id()).0,
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
                associated_token_program: jhandi_munda::ATA_PROGRAM_ID,
            },
            jhandi_munda::instruction::Deposit { amount },
        ),
    );
    assert!(res.is_ok(), "deposit failed: {:?}", res.err());
}

// Creates the insurance vault (the insurance PDA is its authority).
fn init_insurance(t: &mut Table, payer: &Keypair) {
    let res = send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::InitInsurance {
                config: t.config,
                insurance_vault: pda(&[INSURANCE_VAULT_SEED]),
                musd_mint: t.musd_mint,
                payer: payer.pubkey(),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::InitInsurance {},
        ),
    );
    assert!(res.is_ok(), "init_insurance failed: {:?}", res.err());
}

// Plain SPL transfer of MUSD from `from`'s ATA into a token account `to`.
fn transfer_musd(t: &mut Table, from: &Keypair, to: &Pubkey, amount: u64) {
    let mut data = vec![3u8];
    data.extend_from_slice(&amount.to_le_bytes());
    let instruction = Instruction::new_with_bytes(
        SPL_TOKEN_PROGRAM_ID,
        &data,
        vec![
            AccountMeta::new(jhandi_munda::musd_ata(&from.pubkey(), &t.musd_mint), false),
            AccountMeta::new(*to, false),
            AccountMeta::new_readonly(from.pubkey(), true),
        ],
    );
    let res = send(&mut t.svm, from, instruction);
    assert!(res.is_ok(), "transfer failed: {:?}", res.err());
}

// Shared start for the redeem tests: the pool holds 100 MUSD (a deposit), the
// insurance fund holds 50 MUSD (pre-seeded by transfer; Step 9 funds it from
// the treasury slice), and Alice has a $10 win on face 2 with a claim
// (owed $29.60) that unlocks at +190 s. The clock is left at +100 s.
fn winning_claim(t: &mut Table) -> (Keypair, Pubkey) {
    let funder = new_player(t);
    deposit(t, &funder, 100_000_000);
    let insurer = new_player(t);
    init_insurance(t, &insurer);
    transfer_musd(t, &insurer, &pda(&[INSURANCE_VAULT_SEED]), 50_000_000);

    let alice = new_player(t);
    let bob = new_player(t);
    assert!(place(t, &alice, 2, 10_000_000).is_ok());
    assert!(place(t, &bob, 4, 1_000_000).is_ok());
    set_time(t, BASE_TIME + 90);
    assert!(request_roll(t, &bob).is_ok());
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(t).request_id;
    assert!(send_callback(t, &bob, randomness, request_id).is_ok());
    set_time(t, BASE_TIME + 100);
    let alice_bet = bet_pda(&alice, 2);
    assert!(settle_bets(t, &bob, &[alice_bet, bet_pda(&bob, 4)]).is_ok());
    assert!(open_claim(t, &bob, &alice_bet).is_ok());
    (alice, alice_bet)
}

// Redeem accounts for a claim, signed by its player.
fn redeem_accounts(t: &Table, player: &Keypair, bet: &Pubkey) -> jhandi_munda::accounts::RedeemClaim {
    let munda_mint = pda(&[MUNDA_MINT_SEED]);
    jhandi_munda::accounts::RedeemClaim {
        bet: *bet,
        claim: pda(&[CLAIM_SEED, bet.as_ref()]),
        player: player.pubkey(),
        pool: t.pool,
        pool_vault: t.vault,
        insurance_vault: pda(&[INSURANCE_VAULT_SEED]),
        insurance_authority: pda(&[INSURANCE_SEED]),
        player_ata: jhandi_munda::musd_ata(&player.pubkey(), &t.musd_mint),
        queue: pda(&[QUEUE_SEED]),
        munda_mint,
        munda_mint_authority: pda(&[MUNDA_MINT_AUTHORITY_SEED]),
        player_munda_ata: jhandi_munda::musd_ata(&player.pubkey(), &munda_mint),
        token_program: SPL_TOKEN_PROGRAM_ID,
        associated_token_program: jhandi_munda::ATA_PROGRAM_ID,
        system_program: SYS,
    }
}

fn wait(t: &mut Table, player: &Keypair, bet: &Pubkey) -> TransactionResult {
    let accounts = redeem_accounts(t, player, bet);
    send(&mut t.svm, player, ix(accounts, jhandi_munda::instruction::RedeemWait {}))
}

fn take_now(t: &mut Table, player: &Keypair, bet: &Pubkey) -> TransactionResult {
    let accounts = redeem_accounts(t, player, bet);
    send(&mut t.svm, player, ix(accounts, jhandi_munda::instruction::RedeemTakeNow {}))
}

fn collect(t: &mut Table, payer: &Keypair, bet: &Pubkey) -> TransactionResult {
    send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::CollectInsurance {
                bet: *bet,
                claim: pda(&[CLAIM_SEED, bet.as_ref()]),
                pool: t.pool,
                pool_vault: t.vault,
                insurance_vault: pda(&[INSURANCE_VAULT_SEED]),
                token_program: SPL_TOKEN_PROGRAM_ID,
            },
            jhandi_munda::instruction::CollectInsurance {},
        ),
    )
}

#[test]
fn test_win_redeem_full_after_90s() {
    let mut t = setup();
    stub_vrf(&mut t);
    let (alice, bet) = winning_claim(&mut t);
    let alice_ata = jhandi_munda::musd_ata(&alice.pubkey(), &t.musd_mint);

    // Wait is locked until the claim unlocks at +190 s.
    assert!(wait(&mut t, &alice, &bet).is_err(), "Wait before unlock must be rejected");

    // At +190 s Wait pays the full $29.60 from the pool.
    set_time(&mut t, BASE_TIME + 190);
    assert!(wait(&mut t, &alice, &bet).is_ok());
    assert_eq!(token_amount(&t.svm, &alice_ata), 90_000_000 + 29_600_000);
    assert_eq!(token_amount(&t.svm, &t.vault), 111_000_000 - 29_600_000, "pool pays the win");
    assert!(claim(&t, &bet).redeemed);

    // A second redeem is rejected.
    assert!(wait(&mut t, &alice, &bet).is_err(), "a claim redeems once");
}

#[test]
fn test_win_redeem_take_now_95() {
    let mut t = setup();
    stub_vrf(&mut t);
    let (alice, bet) = winning_claim(&mut t);
    let alice_ata = jhandi_munda::musd_ata(&alice.pubkey(), &t.musd_mint);
    let insurance = pda(&[INSURANCE_VAULT_SEED]);

    // Take now pays 95% of $29.60 = $28.12 at once, from the insurance fund.
    assert!(take_now(&mut t, &alice, &bet).is_ok());
    assert_eq!(token_amount(&t.svm, &alice_ata), 90_000_000 + 28_120_000);
    assert_eq!(token_amount(&t.svm, &insurance), 50_000_000 - 28_120_000);
    let c = claim(&t, &bet);
    assert!(c.redeemed);
    assert_eq!(c.insurance_owed, 29_600_000, "the pool owes insurance the full 100%");

    // Take now and Wait are each one-time: a second redeem is rejected.
    assert!(take_now(&mut t, &alice, &bet).is_err());
    assert!(wait(&mut t, &alice, &bet).is_err());

    // The pool repays insurance the full $29.60 once the claim unlocks, and
    // insurance keeps the 5%: its balance ends at $51.48 (50 + 1.48).
    assert!(collect(&mut t, &alice, &bet).is_err(), "collect before unlock must be rejected");
    set_time(&mut t, BASE_TIME + 190);
    assert!(collect(&mut t, &alice, &bet).is_ok());
    assert_eq!(token_amount(&t.svm, &t.vault), 111_000_000 - 29_600_000);
    assert_eq!(token_amount(&t.svm, &insurance), 51_480_000, "insurance keeps its 5%");
    assert_eq!(claim(&t, &bet).insurance_owed, 0);
    assert!(collect(&mut t, &alice, &bet).is_err(), "nothing left to collect");
}

// Creates the MUNDA mint (the program's own mint authority PDA).
fn init_munda(t: &mut Table, payer: &Keypair) {
    let res = send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::InitMunda {
                munda_mint: pda(&[MUNDA_MINT_SEED]),
                munda_mint_authority: pda(&[MUNDA_MINT_AUTHORITY_SEED]),
                payer: payer.pubkey(),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::InitMunda {},
        ),
    );
    assert!(res.is_ok(), "init_munda failed: {:?}", res.err());
}

fn munda_supply(t: &Table) -> u64 {
    let data = t.svm.get_account(&pda(&[MUNDA_MINT_SEED])).unwrap().data;
    u64::from_le_bytes(data[36..44].try_into().unwrap())
}

// Shared start for the loss tests: MUNDA mint created, and Bob loses a $1 bet
// on face 4 (the dice are 2,2,5,6,1,3). His loss claim unlocks at +24 h.
fn losing_claim(t: &mut Table) -> (Keypair, Pubkey) {
    let deployer = new_player(t);
    init_munda(t, &deployer);
    let bob = new_player(t);
    assert!(place(t, &bob, 4, 1_000_000).is_ok());
    set_time(t, BASE_TIME + 90);
    assert!(request_roll(t, &bob).is_ok());
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(t).request_id;
    assert!(send_callback(t, &bob, randomness, request_id).is_ok());
    set_time(t, BASE_TIME + 100);
    let bob_bet = bet_pda(&bob, 4);
    assert!(settle_bets(t, &bob, &[bob_bet]).is_ok());
    assert!(open_claim(t, &bob, &bob_bet).is_ok());
    // MUNDA is minted only on redeem, never at settle (D26).
    assert_eq!(munda_supply(t), 0, "no MUNDA before a redeem");
    (bob, bob_bet)
}

#[test]
fn test_loss_redeem_full_after_24h() {
    let mut t = setup();
    stub_vrf(&mut t);
    let (bob, bet) = losing_claim(&mut t);
    let bob_munda_ata = jhandi_munda::musd_ata(&bob.pubkey(), &pda(&[MUNDA_MINT_SEED]));

    // Wait is locked for 24 hours after the settle at +100 s.
    assert!(wait(&mut t, &bob, &bet).is_err(), "Wait before 24 h must be rejected");
    set_time(&mut t, BASE_TIME + 100 + 86_399);
    assert!(wait(&mut t, &bob, &bet).is_err(), "one second early is still locked");

    // At 24 h, Wait mints the full 1 MUNDA for the 1 MUSD lost.
    set_time(&mut t, BASE_TIME + 100 + 86_400);
    assert!(wait(&mut t, &bob, &bet).is_ok());
    assert_eq!(token_amount(&t.svm, &bob_munda_ata), 1_000_000);
    assert_eq!(munda_supply(&t), 1_000_000, "supply rises on redeem only");
    assert!(claim(&t, &bet).redeemed);
    assert!(wait(&mut t, &bob, &bet).is_err(), "a claim redeems once");
}

#[test]
fn test_loss_redeem_now_10_percent_tax() {
    let mut t = setup();
    stub_vrf(&mut t);
    let (bob, bet) = losing_claim(&mut t);
    let bob_munda_ata = jhandi_munda::musd_ata(&bob.pubkey(), &pda(&[MUNDA_MINT_SEED]));

    // Take now mints 90% of the 1 MUNDA at once; the 10% tax is never minted.
    assert!(take_now(&mut t, &bob, &bet).is_ok());
    assert_eq!(token_amount(&t.svm, &bob_munda_ata), 900_000);
    assert_eq!(munda_supply(&t), 900_000, "only the 90% is ever minted");
    assert!(claim(&t, &bet).redeemed);
    assert!(take_now(&mut t, &bob, &bet).is_err(), "a claim redeems once");
}

#[test]
fn test_queue_init_empty() {
    let t = setup();
    let acc = t.svm.get_account(&pda(&[QUEUE_SEED])).expect("queue PDA must exist");
    assert_eq!(acc.owner, jhandi_munda::id(), "queue belongs to the program");
    // Layout: 8-byte discriminator, then head (u64) and tail (u64).
    let head = u64::from_le_bytes(acc.data[8..16].try_into().unwrap());
    let tail = u64::from_le_bytes(acc.data[16..24].try_into().unwrap());
    assert_eq!(head, 0, "an empty queue has head 0");
    assert_eq!(tail, head, "an empty queue has tail == head");
    assert_eq!(tail - head, 0, "length is 0");
    assert!(acc.data.len() < 10_240, "the queue fits the 10,240-byte ER cap");
}

// The queue as (head, tail, entries): entry i is (claim, amount), 40 bytes each
// after the 24-byte header (8-byte discriminator, head, tail).
fn queue_state(t: &Table) -> (u64, u64, Vec<(Pubkey, u64)>) {
    let data = t.svm.get_account(&pda(&[QUEUE_SEED])).unwrap().data;
    let head = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let tail = u64::from_le_bytes(data[16..24].try_into().unwrap());
    let entries = (head..tail)
        .map(|i| {
            let at = 24 + (i as usize % 100) * 40;
            let claim = Pubkey::from(<[u8; 32]>::try_from(&data[at..at + 32]).unwrap());
            let amount = u64::from_le_bytes(data[at + 32..at + 40].try_into().unwrap());
            (claim, amount)
        })
        .collect();
    (head, tail, entries)
}

#[test]
fn test_enqueue_win_when_pool_short() {
    let mut t = setup();
    stub_vrf(&mut t);
    // A tiny pool: 5 MUSD from a deposit.
    let funder = new_player(&mut t);
    deposit(&mut t, &funder, 5_000_000);

    // Two $10 wins on face 2: each is owed $29.60, more than the whole pool.
    let alice = new_player(&mut t);
    let bob = new_player(&mut t);
    assert!(place(&mut t, &alice, 2, 10_000_000).is_ok());
    assert!(place(&mut t, &bob, 2, 10_000_000).is_ok());
    set_time(&mut t, BASE_TIME + 90);
    assert!(request_roll(&mut t, &alice).is_ok());
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &alice, randomness, request_id).is_ok());
    set_time(&mut t, BASE_TIME + 100);
    let (ab, bb) = (bet_pda(&alice, 2), bet_pda(&bob, 2));
    assert!(settle_bets(&mut t, &alice, &[ab, bb]).is_ok());
    assert!(finish_round(&mut t, &alice).is_ok(), "the round still settles");
    assert!(open_claim(&mut t, &alice, &ab).is_ok());
    assert!(open_claim(&mut t, &bob, &bb).is_ok());

    // At unlock, the pool holds 5 + 20 = 25 MUSD, so neither $29.60 win can be paid.
    set_time(&mut t, BASE_TIME + 190);
    let (ac, bc) = (pda(&[CLAIM_SEED, ab.as_ref()]), pda(&[CLAIM_SEED, bb.as_ref()]));
    assert!(wait(&mut t, &alice, &ab).is_ok(), "a short pool queues the win, it does not fail");
    assert!(claim(&t, &ab).queued && !claim(&t, &ab).redeemed);
    let (head, tail, entries) = queue_state(&t);
    assert_eq!((head, tail), (0, 1));
    assert_eq!(entries, vec![(ac, 29_600_000)], "Alice is at the head, owed $29.60");

    // Bob's win goes behind Alice, even though he asks second.
    assert!(wait(&mut t, &bob, &bb).is_ok());
    let (head, tail, entries) = queue_state(&t);
    assert_eq!((head, tail), (0, 2));
    assert_eq!(entries, vec![(ac, 29_600_000), (bc, 29_600_000)], "FIFO: Alice, then Bob");

    // Nothing was paid: the pool still holds 25 MUSD and each queued claim waits.
    assert_eq!(token_amount(&t.svm, &t.vault), 25_000_000);
    assert!(wait(&mut t, &alice, &ab).is_err(), "a queued claim cannot be queued twice");
    assert!(take_now(&mut t, &alice, &ab).is_err(), "a queued claim cannot take now");
}

// Cranks the payout queue: `pairs` are (player, claim) in queue order. Each
// claim is followed by the player's MUSD ATA, as the instruction expects.
fn pay_queue(t: &mut Table, signer: &Keypair, pairs: &[(&Keypair, Pubkey)]) -> TransactionResult {
    let mut instruction = ix(
        jhandi_munda::accounts::PayQueue {
            queue: pda(&[QUEUE_SEED]),
            pool: t.pool,
            pool_vault: t.vault,
            token_program: SPL_TOKEN_PROGRAM_ID,
        },
        jhandi_munda::instruction::PayQueue {},
    );
    for (player, claim_key) in pairs {
        instruction.accounts.push(AccountMeta::new(*claim_key, false));
        instruction
            .accounts
            .push(AccountMeta::new(jhandi_munda::musd_ata(&player.pubkey(), &t.musd_mint), false));
    }
    send(&mut t.svm, signer, instruction)
}

#[test]
fn test_queue_pays_head_not_newer_winner() {
    let mut t = setup();
    stub_vrf(&mut t);
    let funder = new_player(&mut t);
    deposit(&mut t, &funder, 5_000_000);

    // Round 1: Alice and Bob each win $29.60 on face 2; the pool holds 25 MUSD.
    let alice = new_player(&mut t);
    let bob = new_player(&mut t);
    assert!(place(&mut t, &alice, 2, 10_000_000).is_ok());
    assert!(place(&mut t, &bob, 2, 10_000_000).is_ok());
    set_time(&mut t, BASE_TIME + 90);
    assert!(request_roll(&mut t, &alice).is_ok());
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &alice, randomness, request_id).is_ok());
    set_time(&mut t, BASE_TIME + 100);
    let (ab, bb) = (bet_pda(&alice, 2), bet_pda(&bob, 2));
    assert!(settle_bets(&mut t, &alice, &[ab, bb]).is_ok());
    assert!(finish_round(&mut t, &alice).is_ok());
    assert!(open_claim(&mut t, &alice, &ab).is_ok());
    assert!(open_claim(&mut t, &bob, &bb).is_ok());
    set_time(&mut t, BASE_TIME + 190);
    assert!(wait(&mut t, &alice, &ab).is_ok(), "Alice queues first");
    assert!(wait(&mut t, &bob, &bb).is_ok(), "Bob queues behind her");
    let (ac, bc) = (pda(&[CLAIM_SEED, ab.as_ref()]), pda(&[CLAIM_SEED, bb.as_ref()]));
    assert_eq!(queue_state(&t).2, vec![(ac, 29_600_000), (bc, 29_600_000)]);

    // Round 2: Carol loses $10 on face 4. Her stake flows into the pool: 35 MUSD.
    set_time(&mut t, BASE_TIME + 200);
    let carol = new_player(&mut t);
    assert!(place(&mut t, &carol, 4, 10_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &t.vault), 35_000_000);
    set_time(&mut t, BASE_TIME + 290);
    assert!(request_roll(&mut t, &carol).is_ok());
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &carol, randomness, request_id).is_ok());
    set_time(&mut t, BASE_TIME + 300);
    assert!(settle_bets(&mut t, &carol, &[bet_pda_r(1, &carol, 4)]).is_ok());
    assert!(finish_round(&mut t, &carol).is_ok());

    // Bob cannot jump the queue: passing his claim first is refused.
    assert!(pay_queue(&mut t, &carol, &[(&bob, bc)]).is_err(), "no skipping the head");

    // The crank pays Alice (the head) in full. Bob's $29.60 is more than the
    // $5.40 left, so the crank stops there: Bob stays in the queue.
    assert!(pay_queue(&mut t, &carol, &[(&alice, ac), (&bob, bc)]).is_ok());
    let alice_ata = jhandi_munda::musd_ata(&alice.pubkey(), &t.musd_mint);
    let bob_ata = jhandi_munda::musd_ata(&bob.pubkey(), &t.musd_mint);
    assert_eq!(token_amount(&t.svm, &alice_ata), 90_000_000 + 29_600_000, "Alice paid in full");
    assert_eq!(token_amount(&t.svm, &bob_ata), 90_000_000, "Bob not paid yet");
    assert_eq!(token_amount(&t.svm, &t.vault), 5_400_000);
    assert!(claim(&t, &ab).redeemed && !claim(&t, &ab).queued);
    assert!(claim(&t, &bb).queued && !claim(&t, &bb).redeemed);
    let (head, tail, entries) = queue_state(&t);
    assert_eq!((head, tail), (1, 2), "one paid, one waiting");
    assert_eq!(entries, vec![(bc, 29_600_000)]);
}

// An LP withdraws `amount` shares for MUSD (the program's withdraw instruction).
fn lp_withdraw(t: &mut Table, lp: &Keypair, amount: u64) -> TransactionResult {
    let share_mint = pda(&[SHARE_MINT_SEED]);
    send(
        &mut t.svm,
        lp,
        ix(
            jhandi_munda::accounts::Withdraw {
                pool: t.pool,
                share_mint,
                vault: t.vault,
                withdrawer: lp.pubkey(),
                withdrawer_shares: jhandi_munda::share_ata(&lp.pubkey(), &share_mint),
                withdrawer_ata: jhandi_munda::musd_ata(&lp.pubkey(), &t.musd_mint),
                queue: pda(&[QUEUE_SEED]),
                token_program: SPL_TOKEN_PROGRAM_ID,
            },
            jhandi_munda::instruction::Withdraw { amount },
        ),
    )
}

#[test]
fn test_lp_withdraw_waits_for_queue() {
    let mut t = setup();
    stub_vrf(&mut t);
    // An LP deposits 5 MUSD (5,000,000 shares at NAV $1).
    let lp = new_player(&mut t);
    deposit(&mut t, &lp, 5_000_000);

    // Alice wins $29.60 on a $10 bet, but the pool holds only $15: she queues.
    let alice = new_player(&mut t);
    assert!(place(&mut t, &alice, 2, 10_000_000).is_ok());
    set_time(&mut t, BASE_TIME + 90);
    assert!(request_roll(&mut t, &alice).is_ok());
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &alice, randomness, request_id).is_ok());
    set_time(&mut t, BASE_TIME + 100);
    let ab = bet_pda(&alice, 2);
    assert!(settle_bets(&mut t, &alice, &[ab]).is_ok());
    assert!(finish_round(&mut t, &alice).is_ok());
    assert!(open_claim(&mut t, &alice, &ab).is_ok());
    set_time(&mut t, BASE_TIME + 190);
    assert!(wait(&mut t, &alice, &ab).is_ok());
    let ac = pda(&[CLAIM_SEED, ab.as_ref()]);
    assert_eq!(queue_state(&t).2, vec![(ac, 29_600_000)], "Alice is queued");

    // The LP cannot take MUSD out while Alice waits.
    assert!(lp_withdraw(&mut t, &lp, 1_000_000).is_err(), "LP withdraw refused while queued");

    // Round 2: Carol loses $10 on face 4 and $10 on face 6, so the pool holds $35.
    set_time(&mut t, BASE_TIME + 200);
    let carol = new_player(&mut t);
    assert!(place(&mut t, &carol, 4, 10_000_000).is_ok());
    assert!(place(&mut t, &carol, 6, 10_000_000).is_ok());
    set_time(&mut t, BASE_TIME + 290);
    assert!(request_roll(&mut t, &carol).is_ok());
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &carol, randomness, request_id).is_ok());
    set_time(&mut t, BASE_TIME + 300);
    assert!(settle_bets(&mut t, &carol, &[bet_pda_r(1, &carol, 4), bet_pda_r(1, &carol, 6)]).is_ok());
    assert!(finish_round(&mut t, &carol).is_ok());
    assert!(lp_withdraw(&mut t, &lp, 1_000_000).is_err(), "still refused: Alice is not paid yet");

    // The crank pays Alice, the queue empties, and now the LP can withdraw.
    assert!(pay_queue(&mut t, &carol, &[(&alice, ac)]).is_ok());
    assert!(lp_withdraw(&mut t, &lp, 1_000_000).is_ok(), "LP withdraw after the queue is paid");
    // Vault: 35 - 29.60 paid = 5.40. Supply 5,000,000 shares, so 1,000,000 shares
    // are worth 1,000,000 x 5.40 / 5 = $1.08.
    let lp_ata = jhandi_munda::musd_ata(&lp.pubkey(), &t.musd_mint);
    assert_eq!(token_amount(&t.svm, &lp_ata), 95_000_000 + 1_080_000);
    assert_eq!(token_amount(&t.svm, &t.vault), 4_320_000);
}

// --- Staking helpers (Steps 9 and 10) ---

fn munda_ata(wallet: &Keypair) -> Pubkey {
    jhandi_munda::musd_ata(&wallet.pubkey(), &pda(&[MUNDA_MINT_SEED]))
}

fn clock_now(t: &Table) -> i64 {
    t.svm.get_sysvar::<Clock>().unix_timestamp
}

fn init_staking(t: &mut Table, payer: &Keypair) {
    let res = send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::InitStaking {
                stake_pool: pda(&[jhandi_munda::STAKE_POOL_SEED]),
                stake_vault: pda(&[jhandi_munda::STAKE_VAULT_SEED]),
                munda_mint: pda(&[MUNDA_MINT_SEED]),
                stake_authority: pda(&[jhandi_munda::STAKE_AUTHORITY_SEED]),
                payer: payer.pubkey(),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::InitStaking {},
        ),
    );
    assert!(res.is_ok(), "init_staking failed: {:?}", res.err());
}

fn stake(t: &mut Table, owner: &Keypair, amount: u64) -> TransactionResult {
    send(
        &mut t.svm,
        owner,
        ix(
            jhandi_munda::accounts::StakeMunda {
                stake_pool: pda(&[jhandi_munda::STAKE_POOL_SEED]),
                stake: pda(&[jhandi_munda::STAKE_SEED, owner.pubkey().as_ref()]),
                stake_vault: pda(&[jhandi_munda::STAKE_VAULT_SEED]),
                owner: owner.pubkey(),
                owner_munda_ata: munda_ata(owner),
                munda_mint: pda(&[MUNDA_MINT_SEED]),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::Stake { amount },
        ),
    )
}

fn stake_account(t: &Table, owner: &Keypair) -> jhandi_munda::state::Stake {
    let data = t.svm.get_account(&pda(&[jhandi_munda::STAKE_SEED, owner.pubkey().as_ref()])).unwrap().data;
    jhandi_munda::state::Stake::try_deserialize(&mut data.as_slice()).unwrap()
}

fn stake_pool(t: &Table) -> jhandi_munda::state::StakePool {
    let data = t.svm.get_account(&pda(&[jhandi_munda::STAKE_POOL_SEED])).unwrap().data;
    jhandi_munda::state::StakePool::try_deserialize(&mut data.as_slice()).unwrap()
}

// Mints MUNDA the real way: each holder loses a bet on face 4 (the dice below
// have no 4), the loss claims open, and after the 24 h unlock each holder
// redeems with Wait, which mints 1 MUNDA per 1 MUSD lost (D26).
fn mint_munda(t: &mut Table, holders: &[(&Keypair, u64)]) {
    init_munda(t, holders[0].0);
    let start = clock_now(t);
    for (h, amount) in holders {
        assert!(place(t, h, 4, *amount).is_ok());
    }
    set_time(t, start + 90);
    assert!(request_roll(t, holders[0].0).is_ok());
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(t).request_id;
    assert!(send_callback(t, holders[0].0, randomness, request_id).is_ok());
    set_time(t, start + 100);
    let bets: Vec<Pubkey> = holders.iter().map(|(h, _)| bet_pda(h, 4)).collect();
    assert!(settle_bets(t, holders[0].0, &bets).is_ok());
    assert!(finish_round(t, holders[0].0).is_ok());
    for (h, _) in holders {
        assert!(open_claim(t, h, &bet_pda(h, 4)).is_ok());
    }
    set_time(t, start + 100 + 86_400);
    for (h, _) in holders {
        assert!(wait(t, h, &bet_pda(h, 4)).is_ok());
    }
}

#[test]
fn test_stake_munda() {
    let mut t = setup();
    stub_vrf(&mut t);
    let a = new_player(&mut t);
    mint_munda(&mut t, &[(&a, 70_000_000)]);
    assert_eq!(token_amount(&t.svm, &munda_ata(&a)), 70_000_000, "70 MUNDA from the loss");
    init_staking(&mut t, &a);

    // A zero stake is rejected.
    assert!(stake(&mut t, &a, 0).is_err(), "a zero stake is rejected");

    // Staking 40 MUNDA: the wallet's free MUNDA falls by what the vault gains.
    assert!(stake(&mut t, &a, 40_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &munda_ata(&a)), 30_000_000);
    assert_eq!(token_amount(&t.svm, &pda(&[jhandi_munda::STAKE_VAULT_SEED])), 40_000_000);
    let s = stake_account(&t, &a);
    assert_eq!((s.owner, s.amount), (a.pubkey(), 40_000_000));
    let p = stake_pool(&t);
    assert_eq!((p.total_staked, p.stakers), (40_000_000, 1));
}

fn unstake(t: &mut Table, owner: &Keypair, amount: u64) -> TransactionResult {
    send(
        &mut t.svm,
        owner,
        ix(
            jhandi_munda::accounts::UnstakeMunda {
                stake_pool: pda(&[jhandi_munda::STAKE_POOL_SEED]),
                stake: pda(&[jhandi_munda::STAKE_SEED, owner.pubkey().as_ref()]),
                stake_vault: pda(&[jhandi_munda::STAKE_VAULT_SEED]),
                stake_authority: pda(&[jhandi_munda::STAKE_AUTHORITY_SEED]),
                owner: owner.pubkey(),
                owner_munda_ata: munda_ata(owner),
                token_program: SPL_TOKEN_PROGRAM_ID,
            },
            jhandi_munda::instruction::Unstake { amount },
        ),
    )
}

#[test]
fn test_unstake_munda_principal() {
    let mut t = setup();
    stub_vrf(&mut t);
    let a = new_player(&mut t);
    mint_munda(&mut t, &[(&a, 70_000_000)]);
    init_staking(&mut t, &a);
    assert!(stake(&mut t, &a, 40_000_000).is_ok());

    // Stake and unstake straight away: the principal comes back in full.
    assert!(unstake(&mut t, &a, 40_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &munda_ata(&a)), 70_000_000, "principal back");
    assert_eq!(token_amount(&t.svm, &pda(&[jhandi_munda::STAKE_VAULT_SEED])), 0);
    assert_eq!(stake_account(&t, &a).amount, 0);
    assert_eq!(stake_pool(&t).total_staked, 0);

    // Unstaking more than is staked is rejected and moves nothing.
    assert!(stake(&mut t, &a, 10_000_000).is_ok());
    assert!(unstake(&mut t, &a, 11_000_000).is_err(), "over-unstake is rejected");
    assert_eq!(stake_account(&t, &a).amount, 10_000_000);
    assert!(unstake(&mut t, &a, 10_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &munda_ata(&a)), 70_000_000);
}

// --- Fee distribution helpers (Step 9.3) ---

fn init_fee_vaults(t: &mut Table, payer: &Keypair) {
    let res = send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::InitFeeVaults {
                config: t.config,
                dev_vault: pda(&[jhandi_munda::DEV_VAULT_SEED]),
                treasury_vault: pda(&[jhandi_munda::TREASURY_VAULT_SEED]),
                rewards_vault: pda(&[jhandi_munda::REWARDS_VAULT_SEED]),
                musd_mint: t.musd_mint,
                treasury_authority: pda(&[jhandi_munda::TREASURY_AUTHORITY_SEED]),
                payer: payer.pubkey(),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::InitFeeVaults {},
        ),
    );
    assert!(res.is_ok(), "init_fee_vaults failed: {:?}", res.err());
}

// Distributes the accrued fee pots. `stakers` are the Stake accounts passed.
fn distribute(t: &mut Table, signer: &Keypair, stakers: &[&Keypair]) -> TransactionResult {
    let mut instruction = ix(
        jhandi_munda::accounts::DistributeFees {
            table: t.table,
            stake_pool: pda(&[jhandi_munda::STAKE_POOL_SEED]),
            pool: t.pool,
            pool_vault: t.vault,
            queue: pda(&[QUEUE_SEED]),
            dev_vault: pda(&[jhandi_munda::DEV_VAULT_SEED]),
            insurance_vault: pda(&[INSURANCE_VAULT_SEED]),
            treasury_vault: pda(&[jhandi_munda::TREASURY_VAULT_SEED]),
            rewards_vault: pda(&[jhandi_munda::REWARDS_VAULT_SEED]),
            token_program: SPL_TOKEN_PROGRAM_ID,
        },
        jhandi_munda::instruction::DistributeFees {},
    );
    for s in stakers {
        instruction.accounts.push(AccountMeta::new(pda(&[jhandi_munda::STAKE_SEED, s.pubkey().as_ref()]), false));
    }
    send(&mut t.svm, signer, instruction)
}

fn claim_rewards(t: &mut Table, owner: &Keypair) -> TransactionResult {
    send(
        &mut t.svm,
        owner,
        ix(
            jhandi_munda::accounts::ClaimStakeRewards {
                stake: pda(&[jhandi_munda::STAKE_SEED, owner.pubkey().as_ref()]),
                owner: owner.pubkey(),
                pool: t.pool,
                rewards_vault: pda(&[jhandi_munda::REWARDS_VAULT_SEED]),
                treasury_authority: pda(&[jhandi_munda::TREASURY_AUTHORITY_SEED]),
                owner_musd_ata: jhandi_munda::musd_ata(&owner.pubkey(), &t.musd_mint),
                token_program: SPL_TOKEN_PROGRAM_ID,
            },
            jhandi_munda::instruction::ClaimStakeRewards {},
        ),
    )
}

// One win on face 2 by a fresh wallet: its $10 stake earns the fee pots
// (treasury 80,000, devs 40,000, stakers 280,000).
fn earn_win_fee(t: &mut Table) {
    let now = clock_now(t);
    let winner = new_player(t);
    let rid = round(t).round_id;
    assert!(place(t, &winner, 2, 10_000_000).is_ok());
    set_time(t, now + 90);
    assert!(request_roll(t, &winner).is_ok());
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(t).request_id;
    assert!(send_callback(t, &winner, randomness, request_id).is_ok());
    set_time(t, now + 100);
    assert!(settle_bets(t, &winner, &[bet_pda_r(rid, &winner, 2)]).is_ok());
    assert!(finish_round(t, &winner).is_ok());
}

#[test]
fn test_fee_split_two_stakers() {
    let mut t = setup();
    stub_vrf(&mut t);
    let a = new_player(&mut t);
    let b = new_player(&mut t);
    let c = new_player(&mut t);
    mint_munda(&mut t, &[(&a, 70_000_000), (&b, 30_000_000), (&c, 5_000_000)]);
    init_staking(&mut t, &a);
    init_fee_vaults(&mut t, &a);
    init_insurance(&mut t, &a);
    assert!(stake(&mut t, &a, 70_000_000).is_ok());
    assert!(stake(&mut t, &b, 30_000_000).is_ok());
    // C holds 5 MUNDA but never stakes it.
    // Rewards count only after the 24-hour stake lock (D29).
    let now = clock_now(&t);
    set_time(&mut t, now + 86_400);

    earn_win_fee(&mut t);
    // The crank must be given every staker: leaving B out is refused.
    assert!(distribute(&mut t, &a, &[&a]).is_err(), "an incomplete staker set is refused");

    assert!(distribute(&mut t, &a, &[&a, &b]).is_ok());
    // 10% of the win fee goes to the dev vault; 20% goes to insurance (not yet full).
    assert_eq!(token_amount(&t.svm, &pda(&[jhandi_munda::DEV_VAULT_SEED])), 40_000);
    assert_eq!(token_amount(&t.svm, &pda(&[INSURANCE_VAULT_SEED])), 80_000);
    // The 70% slice, 280,000, splits 70 : 30 by stake.
    assert_eq!(stake_account(&t, &a).pending, 196_000);
    assert_eq!(stake_account(&t, &b).pending, 84_000);

    // Claims pay each staker's slice.
    let a_ata = jhandi_munda::musd_ata(&a.pubkey(), &t.musd_mint);
    let b_ata = jhandi_munda::musd_ata(&b.pubkey(), &t.musd_mint);
    assert!(claim_rewards(&mut t, &a).is_ok());
    assert!(claim_rewards(&mut t, &b).is_ok());
    assert_eq!(token_amount(&t.svm, &a_ata), 30_000_000 + 196_000, "A: 70 of the 100 staked");
    assert_eq!(token_amount(&t.svm, &b_ata), 70_000_000 + 84_000, "B: 30 of the 100 staked");

    // The unstaked holder (C) has no stake, so no reward and no claim.
    assert!(claim_rewards(&mut t, &c).is_err(), "unstaked MUNDA earns nothing");
    assert_eq!(token_amount(&t.svm, &jhandi_munda::musd_ata(&c.pubkey(), &t.musd_mint)), 95_000_000);
}

#[test]
fn test_insurance_fills_then_treasury() {
    let mut t = setup();
    stub_vrf(&mut t);
    let admin = new_player(&mut t);
    init_insurance(&mut t, &admin);
    init_fee_vaults(&mut t, &admin);
    init_munda(&mut t, &admin);
    init_staking(&mut t, &admin);

    // Fill insurance to $999.96, $0.04 short of the $1,000 demo target (D18).
    let feeders: Vec<Keypair> = (0..10).map(|_| new_player(&mut t)).collect();
    let insurance = pda(&[INSURANCE_VAULT_SEED]);
    for f in &feeders[..9] {
        transfer_musd(&mut t, f, &insurance, 100_000_000);
    }
    transfer_musd(&mut t, &feeders[9], &insurance, 99_960_000);
    assert_eq!(token_amount(&t.svm, &insurance), 999_960_000);

    // The first win's 20% slice (80,000) fills the last $0.04 of insurance; the
    // rest, 40,000, goes to treasury.
    earn_win_fee(&mut t);
    assert!(distribute(&mut t, &admin, &[]).is_ok());
    assert_eq!(token_amount(&t.svm, &insurance), jhandi_munda::INSURANCE_TARGET);
    assert_eq!(token_amount(&t.svm, &pda(&[jhandi_munda::TREASURY_VAULT_SEED])), 40_000);

    // Insurance is full, so the next 20% slice goes all to treasury.
    earn_win_fee(&mut t);
    assert!(distribute(&mut t, &admin, &[]).is_ok());
    assert_eq!(token_amount(&t.svm, &insurance), jhandi_munda::INSURANCE_TARGET, "insurance stays at target");
    assert_eq!(token_amount(&t.svm, &pda(&[jhandi_munda::TREASURY_VAULT_SEED])), 40_000 + 80_000);
    assert_eq!(token_amount(&t.svm, &pda(&[jhandi_munda::DEV_VAULT_SEED])), 80_000, "10% each win");
}

// Staker pot waiting in the table (carried until a distribution can pay it).
fn staker_pot(t: &Table) -> u64 {
    let data = t.svm.get_account(&t.table).unwrap().data;
    jhandi_munda::state::TableConfig::try_deserialize(&mut data.as_slice()).unwrap().fee_stakers
}

#[test]
fn test_stake_and_immediate_unstake_earns_zero() {
    let mut t = setup();
    stub_vrf(&mut t);
    let a = new_player(&mut t);
    let b = new_player(&mut t);
    mint_munda(&mut t, &[(&a, 70_000_000), (&b, 30_000_000)]);
    init_fee_vaults(&mut t, &a);
    init_insurance(&mut t, &a);
    init_staking(&mut t, &a);

    // B stakes and matures for a day. Then A stakes and unstakes straight away.
    assert!(stake(&mut t, &b, 30_000_000).is_ok());
    let now = clock_now(&t);
    set_time(&mut t, now + 86_400);
    assert!(stake(&mut t, &a, 70_000_000).is_ok());
    assert!(unstake(&mut t, &a, 70_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &munda_ata(&a)), 70_000_000, "principal back at once");

    earn_win_fee(&mut t);
    assert!(distribute(&mut t, &a, &[&a, &b]).is_ok());
    assert_eq!(stake_account(&t, &a).pending, 0, "stake and unstake earns zero");
    assert_eq!(stake_account(&t, &b).pending, 280_000, "the mature stake takes the whole slice");
}

#[test]
fn test_stake_24h_warp_earns() {
    let mut t = setup();
    stub_vrf(&mut t);
    let a = new_player(&mut t);
    let b = new_player(&mut t);
    mint_munda(&mut t, &[(&a, 70_000_000), (&b, 30_000_000)]);
    init_fee_vaults(&mut t, &a);
    init_insurance(&mut t, &a);
    init_staking(&mut t, &a);
    assert!(stake(&mut t, &a, 70_000_000).is_ok());
    assert!(stake(&mut t, &b, 30_000_000).is_ok());

    // Inside the 24 hours, a fee earns nothing: the staker pot waits in the table.
    earn_win_fee(&mut t);
    assert!(distribute(&mut t, &a, &[&a, &b]).is_ok());
    assert_eq!(stake_account(&t, &a).pending, 0);
    assert_eq!(stake_account(&t, &b).pending, 0);
    assert_eq!(staker_pot(&t), 280_000, "the pot waits for stakers who have matured");

    // After a 24-hour clock warp, the same stakes earn their 70 : 30 share.
    let now = clock_now(&t);
    set_time(&mut t, now + 86_400);
    assert!(distribute(&mut t, &a, &[&a, &b]).is_ok());
    assert_eq!(stake_account(&t, &a).pending, 196_000);
    assert_eq!(stake_account(&t, &b).pending, 84_000);
    assert_eq!(staker_pot(&t), 0);
}

#[test]
fn test_new_stake_lot_has_own_clock() {
    let mut t = setup();
    stub_vrf(&mut t);
    let w = new_player(&mut t);
    let x = new_player(&mut t);
    mint_munda(&mut t, &[(&w, 100_000_000), (&x, 30_000_000)]);
    init_fee_vaults(&mut t, &w);
    init_insurance(&mut t, &w);
    init_staking(&mut t, &w);

    // Lot A: W stakes 70 and X stakes 30 at T0.
    assert!(stake(&mut t, &w, 70_000_000).is_ok());
    assert!(stake(&mut t, &x, 30_000_000).is_ok());
    let now = clock_now(&t);
    set_time(&mut t, now + 86_400); // lot A matures

    // Lot B: W tops up 30 now. It is fresh and earns nothing yet.
    assert!(stake(&mut t, &w, 30_000_000).is_ok());
    assert_eq!(stake_account(&t, &w).lots[0].amount, 70_000_000, "lot A keeps its clock");

    // First fee: only lot A counts, so W has 70 of the 100 eligible; X has 30.
    earn_win_fee(&mut t);
    assert!(distribute(&mut t, &w, &[&w, &x]).is_ok());
    assert_eq!(stake_account(&t, &w).pending, 196_000, "lot B earns zero; W gets 70 : 30");
    assert_eq!(stake_account(&t, &x).pending, 84_000);

    // A further 24 hours: lot B matures too. Both lots now earn.
    let now = clock_now(&t);
    set_time(&mut t, now + 86_400);
    earn_win_fee(&mut t);
    assert!(distribute(&mut t, &w, &[&w, &x]).is_ok());
    // W has 100 of 130 eligible: 280,000 x 100 / 130 = 215,384; X gets 64,615.
    // One base unit of rounding stays in the pot.
    assert_eq!(stake_account(&t, &w).pending, 196_000 + 215_384, "B earns after its 24 hours");
    assert_eq!(stake_account(&t, &x).pending, 84_000 + 64_615);
    assert_eq!(staker_pot(&t), 1, "rounding dust stays in the pot");
}

#[test]
fn test_distribute_rejects_duplicate_staker() {
    let mut t = setup();
    stub_vrf(&mut t);
    let a = new_player(&mut t);
    let b = new_player(&mut t);
    mint_munda(&mut t, &[(&a, 50_000_000), (&b, 50_000_000)]);
    init_fee_vaults(&mut t, &a);
    init_insurance(&mut t, &a);
    init_staking(&mut t, &a);
    assert!(stake(&mut t, &a, 50_000_000).is_ok());
    assert!(stake(&mut t, &b, 50_000_000).is_ok());
    let now = clock_now(&t);
    set_time(&mut t, now + 86_400);
    earn_win_fee(&mut t);

    // Passing A twice would make the count and total look right while B is left out.
    assert!(distribute(&mut t, &a, &[&a, &a]).is_err(), "a duplicate staker is refused");
    assert_eq!(stake_account(&t, &b).pending, 0, "the refused call changed nothing");

    // The complete set works: 280,000 split 50 : 50.
    assert!(distribute(&mut t, &a, &[&a, &b]).is_ok());
    assert_eq!(stake_account(&t, &a).pending, 140_000);
    assert_eq!(stake_account(&t, &b).pending, 140_000);
}

#[test]
fn test_stakers_slice_leaves_pool_nav() {
    let mut t = setup();
    stub_vrf(&mut t);
    // An LP deposits 5 MUSD: 5,000,000 shares at NAV $1.
    let lp = new_player(&mut t);
    deposit(&mut t, &lp, 5_000_000);
    // Two stakers: losses of 50 MUNDA each flow into the pool vault as LP profit.
    let a = new_player(&mut t);
    let b = new_player(&mut t);
    mint_munda(&mut t, &[(&a, 50_000_000), (&b, 50_000_000)]);
    init_fee_vaults(&mut t, &a);
    init_insurance(&mut t, &a);
    init_staking(&mut t, &a);
    assert!(stake(&mut t, &a, 50_000_000).is_ok());
    assert!(stake(&mut t, &b, 50_000_000).is_ok());
    let now = clock_now(&t);
    set_time(&mut t, now + 86_400);
    earn_win_fee(&mut t);

    let vault = pda(&[POOL_VAULT_SEED]);
    let rewards = pda(&[jhandi_munda::REWARDS_VAULT_SEED]);
    let share_mint = pda(&[SHARE_MINT_SEED]);
    let before = token_amount(&t.svm, &vault);
    let supply = u64::from_le_bytes(t.svm.get_account(&share_mint).unwrap().data[36..44].try_into().unwrap());
    assert!(distribute(&mut t, &a, &[&a, &b]).is_ok());

    // The pool vault gives up the dev (40,000), insurance (80,000) and staker
    // (280,000) slices. The staker slice sits in the rewards vault, not the pool.
    let after = token_amount(&t.svm, &vault);
    assert_eq!(before - after, 400_000);
    assert_eq!(token_amount(&t.svm, &rewards), 280_000);

    // An LP withdraws at NAV = pool vault / supply. The rewards vault is not in NAV.
    let expected = (1_000_000u128 * after as u128 / supply as u128) as u64;
    let lp_ata = jhandi_munda::musd_ata(&lp.pubkey(), &t.musd_mint);
    let lp_before = token_amount(&t.svm, &lp_ata);
    assert!(lp_withdraw(&mut t, &lp, 1_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &lp_ata) - lp_before, expected, "LP paid at pool-only NAV");

    // Stakers can still claim their full rewards, from the rewards vault.
    let a_ata = jhandi_munda::musd_ata(&a.pubkey(), &t.musd_mint);
    let b_ata = jhandi_munda::musd_ata(&b.pubkey(), &t.musd_mint);
    assert!(claim_rewards(&mut t, &a).is_ok());
    assert!(claim_rewards(&mut t, &b).is_ok());
    // Each staker lost 50 MUSD in the mint round, so holds 50 MUSD before the 140,000 reward.
    assert_eq!(token_amount(&t.svm, &a_ata), 50_000_000 + 140_000);
    assert_eq!(token_amount(&t.svm, &b_ata), 50_000_000 + 140_000);
    assert_eq!(token_amount(&t.svm, &rewards), 0, "the rewards vault pays out in full");
}

// Delegation program (DLP) id from the SDK. The tests load the no-op stub here,
// so the SDK's CPIs succeed and the instruction's own checks run for real.
fn dlp_id() -> Pubkey {
    Pubkey::new_from_array(ephemeral_rollups_sdk::id().to_bytes())
}

#[test]
fn test_delegate_round_size_under_er_cap() {
    let mut t = setup();
    stub_vrf(&mut t);
    t.svm.add_program(dlp_id(), include_bytes!("fixtures/vrf_stub.so")).unwrap();

    // A live round with an open bet: this is what gets delegated.
    let alice = new_player(&mut t);
    assert!(place(&mut t, &alice, 2, 10_000_000).is_ok());
    let round = t.round;

    // Size budget (D20, notes section 2): the delegated round must stay under the
    // 10,240-byte ER commit cap.
    let size = t.svm.get_account(&round).unwrap().data.len();
    assert_eq!(size, 392, "measured round size, as recorded in docs/er-design.md");
    assert!(size <= 10_240, "round is {size} bytes, over the 10,240-byte cap");

    let program = jhandi_munda::id();
    let buffer = Pubkey::find_program_address(
        &[ephemeral_rollups_sdk::pda::DELEGATE_BUFFER_TAG, round.as_ref()],
        &program,
    )
    .0;
    let record = Pubkey::find_program_address(
        &[ephemeral_rollups_sdk::pda::DELEGATION_RECORD_TAG, round.as_ref()],
        &dlp_id(),
    )
    .0;
    let metadata = Pubkey::find_program_address(
        &[ephemeral_rollups_sdk::pda::DELEGATION_METADATA_TAG, round.as_ref()],
        &dlp_id(),
    )
    .0;
    // Only the table authority (D40) may delegate: the admin pays here.
    let res = send(
        &mut t.svm,
        &t.admin,
        ix(
            jhandi_munda::accounts::DelegateRound {
                payer: t.admin.pubkey(),
                config: t.config,
                round,
                buffer_round: buffer,
                delegation_record_round: record,
                delegation_metadata_round: metadata,
                owner_program: program,
                delegation_program: dlp_id(),
                system_program: SYS,
            },
            jhandi_munda::instruction::DelegateRound {},
        ),
    );
    assert!(res.is_ok(), "delegate_round failed: {:?}", res.err());

    // The SDK hands the round to the delegation program; the data stays at the same size.
    let after = t.svm.get_account(&round).unwrap();
    assert_eq!(after.owner, dlp_id(), "the round is now owned by the delegation program");
    assert_eq!(after.data.len(), size, "delegated round keeps its size");
    assert!(after.data.len() <= 10_240);
}

#[test]
fn test_delegate_rejects_stranger() {
    let mut t = setup();
    stub_vrf(&mut t);
    t.svm.add_program(dlp_id(), include_bytes!("fixtures/vrf_stub.so")).unwrap();
    let alice = new_player(&mut t);
    assert!(place(&mut t, &alice, 2, 10_000_000).is_ok());
    let round = t.round;
    let owner_before = t.svm.get_account(&round).unwrap().owner;

    // A stranger (not the config admin, D40) cannot delegate the round.
    let program = jhandi_munda::id();
    let buffer = Pubkey::find_program_address(&[ephemeral_rollups_sdk::pda::DELEGATE_BUFFER_TAG, round.as_ref()], &program).0;
    let record = Pubkey::find_program_address(&[ephemeral_rollups_sdk::pda::DELEGATION_RECORD_TAG, round.as_ref()], &dlp_id()).0;
    let metadata = Pubkey::find_program_address(&[ephemeral_rollups_sdk::pda::DELEGATION_METADATA_TAG, round.as_ref()], &dlp_id()).0;
    let stranger_ix = || ix(
        jhandi_munda::accounts::DelegateRound {
            payer: alice.pubkey(),
            config: t.config,
            round,
            buffer_round: buffer,
            delegation_record_round: record,
            delegation_metadata_round: metadata,
            owner_program: program,
            delegation_program: dlp_id(),
            system_program: SYS,
        },
        jhandi_munda::instruction::DelegateRound {},
    );
    assert!(send(&mut t.svm, &alice, stranger_ix()).is_err(), "a stranger is rejected");
    assert_eq!(t.svm.get_account(&round).unwrap().owner, owner_before, "the round did not move");
}

// --- Chips at the cage (Step 11.3) ---

fn chip_stack(owner: &Keypair) -> Pubkey {
    pda(&[jhandi_munda::CHIPS_SEED, owner.pubkey().as_ref()])
}

fn chip_balance(t: &Table, owner: &Keypair) -> u64 {
    let data = t.svm.get_account(&chip_stack(owner)).unwrap().data;
    jhandi_munda::state::ChipStack::try_deserialize(&mut data.as_slice()).unwrap().balance
}

fn cage() -> Pubkey {
    pda(&[jhandi_munda::CAGE_VAULT_SEED])
}

fn init_cage(t: &mut Table, payer: &Keypair) {
    let res = send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::InitCage {
                config: t.config,
                pool: t.pool,
                cage_vault: cage(),
                musd_mint: t.musd_mint,
                payer: payer.pubkey(),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::InitCage {},
        ),
    );
    assert!(res.is_ok(), "init_cage failed: {:?}", res.err());
}

// Settles chip bets: `pairs` are (bet, chip stack).
fn settle_chip_bets(t: &mut Table, signer: &Keypair, pairs: &[(Pubkey, Pubkey)]) -> TransactionResult {
    let mut instruction = ix(
        jhandi_munda::accounts::SettleChipBets {
            table: t.table,
            round: t.round,
            pool: t.pool,
            pool_vault: t.vault,
            cage_vault: cage(),
            token_program: SPL_TOKEN_PROGRAM_ID,
            queue: pda(&[QUEUE_SEED]),
        },
        jhandi_munda::instruction::SettleChipBets {},
    );
    for (bet, chips) in pairs {
        instruction.accounts.push(AccountMeta::new(*bet, false));
        instruction.accounts.push(AccountMeta::new(*chips, false));
    }
    send(&mut t.svm, signer, instruction)
}

fn buy_chips(t: &mut Table, owner: &Keypair, amount: u64) -> TransactionResult {
    send(
        &mut t.svm,
        owner,
        ix(
            jhandi_munda::accounts::BuyChips {
                pool: t.pool,
                cage_vault: cage(),
                chips: chip_stack(owner),
                owner: owner.pubkey(),
                owner_musd_ata: jhandi_munda::musd_ata(&owner.pubkey(), &t.musd_mint),
                token_program: SPL_TOKEN_PROGRAM_ID,
                system_program: SYS,
            },
            jhandi_munda::instruction::BuyChips { amount },
        ),
    )
}

fn cash_out(t: &mut Table, owner: &Keypair, amount: u64) -> TransactionResult {
    send(
        &mut t.svm,
        owner,
        ix(
            jhandi_munda::accounts::CashOut {
                pool: t.pool,
                cage_vault: cage(),
                queue: pda(&[QUEUE_SEED]),
                chips: chip_stack(owner),
                owner: owner.pubkey(),
                owner_musd_ata: jhandi_munda::musd_ata(&owner.pubkey(), &t.musd_mint),
                token_program: SPL_TOKEN_PROGRAM_ID,
            },
            jhandi_munda::instruction::CashOut { amount },
        ),
    )
}

// A bet paid from chips: no token transfer, the round is the only thing that changes.
// D42: the slip is opened on Solana first. The helper opens it when it is not open yet, so
// existing chip-bet tests keep their meaning; `place_chip_bet_raw` does not open anything.
fn place_chip_bet(t: &mut Table, bettor: &Keypair, symbol: u8, amount: u64) -> TransactionResult {
    let round_id = round(t).round_id;
    if t.svm.get_account(&bet_pda_r(round_id, bettor, symbol)).is_none() {
        open_chip_bet(t, bettor, round_id, symbol).expect("open the slip");
    }
    place_chip_bet_raw(t, bettor, bet_pda_r(round_id, bettor, symbol), symbol, amount)
}

fn place_chip_bet_raw(t: &mut Table, bettor: &Keypair, bet: Pubkey, symbol: u8, amount: u64) -> TransactionResult {
    send(
        &mut t.svm,
        bettor,
        ix(
            jhandi_munda::accounts::PlaceBetChips {
                table: t.table,
                round: t.round,
                bet,
                chips: chip_stack(bettor),
                bettor: bettor.pubkey(),
                system_program: SYS,
            },
            jhandi_munda::instruction::PlaceBetChips { symbol, amount },
        ),
    )
}

#[test]
fn test_er_bets_on_chips() {
    let mut t = setup();
    stub_vrf(&mut t);
    // The pool holds 100 MUSD from an LP deposit: wins can be paid from it.
    let funder = new_player(&mut t);
    deposit(&mut t, &funder, 100_000_000);
    init_cage(&mut t, &funder);
    let a = new_player(&mut t);
    let b = new_player(&mut t);

    // Chips are bought on Solana: the MUSD moves into the cage, not the pool.
    assert!(buy_chips(&mut t, &a, 20_000_000).is_ok());
    assert_eq!(chip_balance(&t, &a), 20_000_000);
    assert_eq!(token_amount(&t.svm, &cage()), 20_000_000, "chip money is in the cage");
    assert_eq!(token_amount(&t.svm, &t.vault), 100_000_000, "pool is unchanged by chip money");
    assert!(buy_chips(&mut t, &b, 5_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &cage()), 25_000_000);

    // A chip bet subtracts chips and moves no tokens: the stake waits in the cage.
    assert!(place_chip_bet(&mut t, &a, 2, 10_000_000).is_ok(), "a chip bet opens the round");
    assert_eq!(chip_balance(&t, &a), 10_000_000);
    assert_eq!(token_amount(&t.svm, &cage()), 25_000_000, "no token transfer for a chip bet");
    assert!(place_chip_bet(&mut t, &a, 4, 11_000_000).is_err(), "more than the chips held is refused");
    assert!(place_chip_bet(&mut t, &b, 4, 1_000_000).is_ok(), "a second player joins");
    assert_eq!(round(&t).players, vec![a.pubkey(), b.pubkey()]);

    // The VRF request goes on the ephemeral queue (D11, D20); any other queue is refused.
    set_time(&mut t, BASE_TIME + 90);
    let ephemeral_queue: Pubkey = "5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc".parse().unwrap();
    assert!(request_roll_on(&mut t, &a, pda(&[b"not_a_queue"])).is_err(), "a foreign queue is refused");
    assert!(request_roll_on(&mut t, &a, ephemeral_queue).is_ok(), "the ephemeral queue is accepted");
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &a, randomness, request_id).is_ok());
    assert_eq!(round(&t).faces, [2, 2, 5, 6, 1, 3]);

    // Settle the chip bets (D39): the stakes move cage -> pool (+11), and Alice's
    // win is paid pool -> cage (29.60) and credited to her chips.
    set_time(&mut t, BASE_TIME + 100);
    let (ab, bb) = (bet_pda_r(0, &a, 2), bet_pda_r(0, &b, 4));
    let (ac, bc) = (pda(&[jhandi_munda::CHIPS_SEED, a.pubkey().as_ref()]), pda(&[jhandi_munda::CHIPS_SEED, b.pubkey().as_ref()]));
    assert!(settle_chip_bets(&mut t, &a, &[(ab, ac), (bb, bc)]).is_ok());
    assert!(finish_round(&mut t, &a).is_ok());
    assert_eq!(token_amount(&t.svm, &t.vault), 100_000_000 + 11_000_000 - 29_600_000, "pool: stakes in, win out");
    assert_eq!(token_amount(&t.svm, &cage()), 25_000_000 - 11_000_000 + 29_600_000, "cage: stakes out, win in");
    assert_eq!(chip_balance(&t, &a), 10_000_000 + 29_600_000, "Alice's win is credited to chips");
    assert_eq!(chip_balance(&t, &b), 4_000_000, "Bob's loss: no credit");
    // Invariant: the cage holds exactly the chips players hold.
    assert_eq!(token_amount(&t.svm, &cage()), chip_balance(&t, &a) + chip_balance(&t, &b));
    // A chip bet is paid into chips, so it has no claim.
    assert!(open_claim(&mut t, &a, &ab).is_err(), "no claim for a chip bet");

    // Cash-out turns chips back into MUSD from the cage (queue empty, D30).
    let ata = jhandi_munda::musd_ata(&a.pubkey(), &t.musd_mint);
    let ata_before = token_amount(&t.svm, &ata);
    assert!(cash_out(&mut t, &a, 5_000_000).is_ok());
    assert_eq!(chip_balance(&t, &a), 34_600_000);
    assert_eq!(token_amount(&t.svm, &ata), ata_before + 5_000_000);
    assert_eq!(token_amount(&t.svm, &cage()), 43_600_000 - 5_000_000);
    assert!(cash_out(&mut t, &a, 50_000_000).is_err(), "cannot cash out more chips than held");
}

#[test]
fn test_chip_money_not_in_lp_nav() {
    let mut t = setup();
    stub_vrf(&mut t);
    // An LP deposits 5 MUSD: 5,000,000 shares, pool NAV $1.
    let lp = new_player(&mut t);
    deposit(&mut t, &lp, 5_000_000);
    init_cage(&mut t, &lp);
    let nav_before = token_amount(&t.svm, &t.vault);

    // A player buys 20 chips. The pool is untouched, so NAV is unchanged.
    let buyer = new_player(&mut t);
    assert!(buy_chips(&mut t, &buyer, 20_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &t.vault), nav_before, "chip money is not pool money");

    // One share is still worth $1: 1,000,000 shares pay 1,000,000 MUSD.
    let lp_ata = jhandi_munda::musd_ata(&lp.pubkey(), &t.musd_mint);
    let before = token_amount(&t.svm, &lp_ata);
    assert!(lp_withdraw(&mut t, &lp, 1_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &lp_ata) - before, 1_000_000, "LP paid at pool NAV");

    // The LP cannot take the chip money: the rest of the shares pay only pool money.
    let before = token_amount(&t.svm, &lp_ata);
    assert!(lp_withdraw(&mut t, &lp, 4_000_000).is_ok());
    assert_eq!(token_amount(&t.svm, &lp_ata) - before, 4_000_000, "not 24,000,000");
    assert_eq!(token_amount(&t.svm, &cage()), 20_000_000, "the cage still holds the chips");
    assert_eq!(chip_balance(&t, &buyer), 20_000_000);
}

#[test]
fn test_delegated_chips_cannot_cash_out() {
    let mut t = setup();
    stub_vrf(&mut t);
    t.svm.add_program(dlp_id(), include_bytes!("fixtures/vrf_stub.so")).unwrap();
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    let a = new_player(&mut t);
    assert!(buy_chips(&mut t, &a, 10_000_000).is_ok());

    // Delegate the chip stack with the owner's signature; it is tiny (49 bytes).
    let chips = chip_stack(&a);
    let size = t.svm.get_account(&chips).unwrap().data.len();
    assert!(size <= 10_240, "chip stack is {size} bytes");
    let buffer = pda_with(&[ephemeral_rollups_sdk::pda::DELEGATE_BUFFER_TAG, chips.as_ref()], &jhandi_munda::id());
    let record = pda_with(&[ephemeral_rollups_sdk::pda::DELEGATION_RECORD_TAG, chips.as_ref()], &dlp_id());
    let metadata = pda_with(&[ephemeral_rollups_sdk::pda::DELEGATION_METADATA_TAG, chips.as_ref()], &dlp_id());
    let res = send(
        &mut t.svm,
        &a,
        ix(
            jhandi_munda::accounts::DelegateChipStack {
                owner: a.pubkey(),
                chips,
                buffer_chips: buffer,
                delegation_record_chips: record,
                delegation_metadata_chips: metadata,
                owner_program: jhandi_munda::id(),
                delegation_program: dlp_id(),
                system_program: SYS,
            },
            jhandi_munda::instruction::DelegateChipStack {},
        ),
    );
    assert!(res.is_ok(), "delegate_chip_stack failed: {:?}", res.err());
    assert_eq!(t.svm.get_account(&chips).unwrap().owner, dlp_id(), "the chips moved to the rollup");

    // Cash-out needs Solana: a delegated stack is refused.
    assert!(cash_out(&mut t, &a, 1_000_000).is_err(), "no cash-out while delegated");
}

fn pda_with(seeds: &[&[u8]], program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(seeds, program).0
}

// --- migrate_config (Step 11.2 follow-up) ---

// Makes `authority` the upgrade authority in this program's ProgramData account.
fn set_upgrade_authority(t: &mut Table, authority: &Pubkey) {
    let pd = pda_with(&[jhandi_munda::id().as_ref()], &anchor_lang::solana_program::bpf_loader_upgradeable::id());
    let mut acc = t.svm.get_account(&pd).expect("ProgramData exists");
    let elf = acc.data[13..].to_vec(); // the header is 13 bytes when the authority is None
    let mut data = Vec::with_capacity(45 + elf.len());
    data.extend_from_slice(&[3, 0, 0, 0]);
    data.extend_from_slice(&acc.data[4..12]); // slot
    data.push(1); // Some(authority)
    data.extend_from_slice(authority.as_ref());
    data.extend_from_slice(&elf);
    acc.data = data;
    t.svm.set_account(pd, acc).unwrap();
}

// Puts the config back in the layout before `admin` existed: 42 bytes.
fn write_old_config(t: &mut Table) {
    let mut acc = t.svm.get_account(&t.config).unwrap();
    let new = acc.data.clone();
    let mut old = Vec::with_capacity(42);
    old.extend_from_slice(&new[0..8]); // discriminator
    old.extend_from_slice(&new[40..72]); // musd_mint
    old.push(new[72]); // bump
    old.push(new[73]); // musd_bump
    acc.data = old;
    t.svm.set_account(t.config, acc).unwrap();
}

fn migrate(t: &mut Table, signer: &Keypair) -> TransactionResult {
    let pd = pda_with(&[jhandi_munda::id().as_ref()], &anchor_lang::solana_program::bpf_loader_upgradeable::id());
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::MigrateConfig {
                config: t.config,
                upgrade_authority: signer.pubkey(),
                program_data: pd,
                system_program: SYS,
            },
            jhandi_munda::instruction::MigrateConfig {},
        ),
    )
}

fn delegate_round_by(t: &mut Table, signer: &Keypair) -> TransactionResult {
    let round = t.round;
    let program = jhandi_munda::id();
    let buffer = pda_with(&[ephemeral_rollups_sdk::pda::DELEGATE_BUFFER_TAG, round.as_ref()], &program);
    let record = pda_with(&[ephemeral_rollups_sdk::pda::DELEGATION_RECORD_TAG, round.as_ref()], &dlp_id());
    let metadata = pda_with(&[ephemeral_rollups_sdk::pda::DELEGATION_METADATA_TAG, round.as_ref()], &dlp_id());
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::DelegateRound {
                payer: signer.pubkey(),
                config: t.config,
                round,
                buffer_round: buffer,
                delegation_record_round: record,
                delegation_metadata_round: metadata,
                owner_program: program,
                delegation_program: dlp_id(),
                system_program: SYS,
            },
            jhandi_munda::instruction::DelegateRound {},
        ),
    )
}

#[test]
fn test_migrate_config_then_delegate() {
    let mut t = setup();
    stub_vrf(&mut t);
    t.svm.add_program(dlp_id(), include_bytes!("fixtures/vrf_stub.so")).unwrap();
    let admin = t.admin.insecure_clone();
    let stranger = new_player(&mut t);
    let alice = new_player(&mut t);
    assert!(place(&mut t, &alice, 2, 10_000_000).is_ok());

    // The deployed devnet config is in the old 42-byte layout, and the admin is
    // the program's upgrade authority.
    set_upgrade_authority(&mut t, &admin.pubkey());
    write_old_config(&mut t);
    assert_eq!(t.svm.get_account(&t.config).unwrap().data.len(), 42);
    assert!(delegate_round_by(&mut t, &admin).is_err(), "the old config cannot be read yet");

    // Only the upgrade authority can migrate: a stranger is refused.
    assert!(migrate(&mut t, &stranger).is_err(), "a stranger cannot migrate the config");
    assert_eq!(t.svm.get_account(&t.config).unwrap().data.len(), 42, "the refused call changed nothing");

    // The upgrade authority migrates; `admin` becomes the table authority.
    assert!(migrate(&mut t, &admin).is_ok(), "the upgrade authority migrates");
    let data = t.svm.get_account(&t.config).unwrap().data;
    assert_eq!(data.len(), 74, "the config is in the current layout");
    assert_eq!(&data[8..40], admin.pubkey().as_ref(), "admin is the upgrade authority");
    assert!(migrate(&mut t, &admin).is_err(), "a second migration is refused");

    // Now delegation works, for the admin only (D40).
    assert!(delegate_round_by(&mut t, &admin).is_ok(), "the admin delegates the round");
}

// --- migrate_table and migrate_round (Blitz verify, PR #109) ---

fn migrate_table_ix(t: &mut Table, signer: &Keypair) -> TransactionResult {
    let pd = pda_with(&[jhandi_munda::id().as_ref()], &anchor_lang::solana_program::bpf_loader_upgradeable::id());
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::MigrateTable {
                table: pda(&[TABLE_SEED]),
                upgrade_authority: signer.pubkey(),
                program_data: pd,
                system_program: SYS,
            },
            jhandi_munda::instruction::MigrateTable {},
        ),
    )
}

/// The v1 round PDA (`ROUND_V1_SEED`, D41). `migrate_round` migrates this account only.
fn v1_round() -> Pubkey {
    Pubkey::find_program_address(&[jhandi_munda::ROUND_V1_SEED], &jhandi_munda::id()).0
}

fn migrate_round_ix(t: &mut Table, signer: &Keypair) -> TransactionResult {
    let pd = pda_with(&[jhandi_munda::id().as_ref()], &anchor_lang::solana_program::bpf_loader_upgradeable::id());
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::MigrateRound {
                round: v1_round(),
                upgrade_authority: signer.pubkey(),
                program_data: pd,
                system_program: SYS,
            },
            jhandi_munda::instruction::MigrateRound {},
        ),
    )
}

#[test]
fn test_migrate_table_then_place_bet() {
    let mut t = setup();
    stub_vrf(&mut t);
    let admin = t.admin.insecure_clone();
    let stranger = new_player(&mut t);
    let alice = new_player(&mut t);
    set_upgrade_authority(&mut t, &admin.pubkey());

    // The deployed table is the 34-byte layout: no fee pots. Write it.
    let table_bump = Pubkey::find_program_address(&[TABLE_SEED], &jhandi_munda::id()).1;
    let mut old = t.svm.get_account(&t.table).unwrap();
    let mut bytes = old.data[0..8].to_vec();
    bytes.extend_from_slice(&90i64.to_le_bytes()); // round_length_secs
    bytes.extend_from_slice(&1_000_000u64.to_le_bytes()); // min_bet
    bytes.extend_from_slice(&0u64.to_le_bytes()); // max_bet
    bytes.push(0); // max_bet_enabled
    bytes.push(table_bump);
    assert_eq!(bytes.len(), 34);
    old.data = bytes;
    t.svm.set_account(t.table, old).unwrap();

    // The old table cannot be read: placing a bet fails.
    assert!(place(&mut t, &alice, 2, 1_000_000).is_err(), "the old table cannot place bets");

    // A stranger cannot migrate it; the upgrade authority can, once.
    assert!(migrate_table_ix(&mut t, &stranger).is_err(), "a stranger cannot migrate the table");
    assert!(migrate_table_ix(&mut t, &admin).is_ok(), "the upgrade authority migrates the table");
    let data = t.svm.get_account(&t.table).unwrap().data;
    assert_eq!(data.len(), 8 + TableConfig::INIT_SPACE, "sized from the struct");
    let cfg = TableConfig::try_deserialize(&mut data.as_slice()).unwrap();
    assert_eq!((cfg.round_length_secs, cfg.min_bet), (90, 1_000_000), "old fields kept");
    assert_eq!((cfg.fee_treasury, cfg.fee_devs, cfg.fee_stakers), (0, 0, 0), "new fee pots are zero");
    assert!(migrate_table_ix(&mut t, &admin).is_err(), "a second migration is refused");

    // Placing a bet works on the migrated table.
    assert!(place(&mut t, &alice, 2, 1_000_000).is_ok(), "place_bet works after migration");
}

#[test]
fn test_migrate_round_388_to_392() {
    let mut t = setup();
    stub_vrf(&mut t);
    let admin = t.admin.insecure_clone();
    let stranger = new_player(&mut t);
    set_upgrade_authority(&mut t, &admin.pubkey());

    // The deployed round is 388 bytes: no bet_count or settled_count. Idle, round 0.
    let round_bump = Pubkey::find_program_address(&[jhandi_munda::ROUND_V1_SEED], &jhandi_munda::id()).1;
    let write_old_round = |t: &mut Table, status: u8| write_old_round_with(t, status, &[], round_bump);

    // A live round is never migrated.
    write_old_round(&mut t, 1); // Open
    assert!(migrate_round_ix(&mut t, &admin).is_err(), "a live round is refused");

    // An idle 388-byte round migrates; a stranger cannot.
    write_old_round(&mut t, 0); // Idle
    assert!(migrate_round_ix(&mut t, &stranger).is_err(), "a stranger cannot migrate the round");
    assert!(migrate_round_ix(&mut t, &admin).is_ok(), "the upgrade authority migrates the round");
    let acc = t.svm.get_account(&v1_round()).unwrap();
    assert_eq!(acc.data.len(), 8 + Round::INIT_SPACE, "392 bytes, sized from the struct");
    let r = Round::try_deserialize(&mut acc.data.as_slice()).unwrap();
    assert_eq!((r.status, r.round_id, r.bet_count, r.settled_count), (RoundStatus::Idle, 0, 0, 0));
    assert_eq!(r.bump, round_bump, "the old fields are kept");

    // The current round (round_v2) takes bets. The migrated v1 account is no longer the table's round.
    let alice = new_player(&mut t);
    assert!(place(&mut t, &alice, 2, 1_000_000).is_ok(), "the current round takes a bet");
}

// Writes the deployed 388-byte round layout: status, opened_at, round_id, players
// (with room for 10), request_id, faces, bump. No counters.
fn write_old_round_with(t: &mut Table, status: u8, players: &[Pubkey], bump: u8) {
    // The v1 account does not exist in setup (only round_v2 does): create it from the v2 account's
    // owner and rent, then overwrite its data with the old layout.
    let mut acc = t.svm.get_account(&t.round).unwrap();
    let mut b = acc.data[0..8].to_vec();
    b.push(status);
    b.extend_from_slice(&0i64.to_le_bytes()); // opened_at
    b.extend_from_slice(&0u64.to_le_bytes()); // round_id
    b.extend_from_slice(&(players.len() as u32).to_le_bytes());
    for p in players {
        b.extend_from_slice(p.as_ref());
    }
    b.extend_from_slice(&vec![0u8; (10 - players.len()) * 32]); // unused slots
    b.extend_from_slice(&[0u8; 32]); // request_id
    b.extend_from_slice(&[0u8; 6]); // faces
    b.push(bump);
    assert_eq!(b.len(), 388);
    acc.data = b;
    t.svm.set_account(v1_round(), acc).unwrap();
}

#[test]
fn test_migrate_rolled_round_then_finish() {
    let mut t = setup();
    stub_vrf(&mut t);
    let admin = t.admin.insecure_clone();
    let stranger = new_player(&mut t);
    let alice = new_player(&mut t);
    set_upgrade_authority(&mut t, &admin.pubkey());
    let round_bump = Pubkey::find_program_address(&[jhandi_munda::ROUND_V1_SEED], &jhandi_munda::id()).1;

    // The devnet leftover: a finished (Rolled) round with one player, in the old layout.
    write_old_round_with(&mut t, 3, &[alice.pubkey()], round_bump);

    // Only the upgrade authority may migrate it.
    assert!(migrate_round_ix(&mut t, &stranger).is_err(), "a stranger cannot migrate the round");
    assert_eq!(t.svm.get_account(&v1_round()).unwrap().data.len(), 388, "the refused call changed nothing");

    // The authority migrates: it grows, adds zero counters, and changes nothing else.
    assert!(migrate_round_ix(&mut t, &admin).is_ok());
    let r = Round::try_deserialize(&mut t.svm.get_account(&v1_round()).unwrap().data.as_slice()).unwrap();
    assert_eq!(r.status, RoundStatus::Rolled, "the migration keeps the status");
    assert_eq!((r.bet_count, r.settled_count), (0, 0), "the counters start at zero");
    assert_eq!(r.players, vec![alice.pubkey()], "players are kept by the migration");
    assert_eq!(r.round_id, 0);

    // A second migration is refused: the account is already the current size.
    assert!(migrate_round_ix(&mut t, &admin).is_err(), "already migrated: refused cleanly");

    // finish_round works on round_v2 (D41), not the v1 account, so the migrated v1 round is not
    // finished here. The table's current round (round_v2) is untouched by the v1 migration.
    let cur = Round::try_deserialize(&mut t.svm.get_account(&t.round).unwrap().data.as_slice()).unwrap();
    assert_eq!((cur.status, cur.round_id), (RoundStatus::Idle, 0), "round_v2 is untouched");
}

// --- commit and undelegate (Step 11.4, #44) ---
// UNVERIFIED on a validator: the magic program is a no-op stub here, so only our
// checks and the SDK's builder call run. The rollup performs the commit.

fn magic_program_id() -> Pubkey {
    Pubkey::new_from_array(ephemeral_rollups_sdk::consts::MAGIC_PROGRAM_ID.to_bytes())
}

fn magic_context_id() -> Pubkey {
    Pubkey::new_from_array(ephemeral_rollups_sdk::consts::MAGIC_CONTEXT_ID.to_bytes())
}

fn undelegate_round_by(t: &mut Table, signer: &Keypair) -> TransactionResult {
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::UndelegateRound {
                payer: signer.pubkey(),
                config: t.config,
                round: t.round,
                magic_context: magic_context_id(),
                magic_program: magic_program_id(),
            },
            jhandi_munda::instruction::UndelegateRound {},
        ),
    )
}

fn undelegate_chips_by(t: &mut Table, signer: &Keypair, owner: &Pubkey) -> TransactionResult {
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::UndelegateChips {
                payer: signer.pubkey(),
                config: t.config,
                chips: pda(&[jhandi_munda::CHIPS_SEED, owner.as_ref()]),
                magic_context: magic_context_id(),
                magic_program: magic_program_id(),
            },
            jhandi_munda::instruction::UndelegateChips { owner: *owner },
        ),
    )
}

#[test]
fn test_undelegate_round_and_chips_admin_only() {
    let mut t = setup();
    stub_vrf(&mut t);
    t.svm.add_program(magic_program_id(), include_bytes!("fixtures/vrf_stub.so")).unwrap();
    let admin = t.admin.insecure_clone();
    let stranger = new_player(&mut t);
    let alice = new_player(&mut t);

    // A live round and a chip stack, as they would be on the rollup.
    assert!(place(&mut t, &alice, 2, 1_000_000).is_ok());
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    assert!(buy_chips(&mut t, &alice, 5_000_000).is_ok());

    // Only the table authority (D40) can commit and undelegate the round.
    assert!(undelegate_round_by(&mut t, &stranger).is_err(), "a stranger cannot undelegate the round");
    assert!(undelegate_round_by(&mut t, &admin).is_ok(), "the admin undelegates the round");

    // The chip stack: the admin with the right owner; a wrong owner is refused.
    let bob = new_player(&mut t);
    assert!(undelegate_chips_by(&mut t, &admin, &bob.pubkey()).is_err(), "the wrong owner is refused");
    assert!(undelegate_chips_by(&mut t, &stranger, &alice.pubkey()).is_err(), "a stranger cannot undelegate chips");
    assert!(undelegate_chips_by(&mut t, &admin, &alice.pubkey()).is_ok(), "the admin undelegates Alice's chips");
    // The round and chips are still readable on Solana in this stub test.
    assert_eq!(t.svm.get_account(&t.round).unwrap().owner, jhandi_munda::id());
}

// The delegation program calls back into our program on Solana when a delegated account is
// undelegated, with the SDK's external-undelegate instruction (`process_undelegation`, added by
// `#[ephemeral]`). Without that callback the call fails with InstructionFallbackNotFound (101),
// which is what stuck devnet undelegates. UNVERIFIED on a validator: the delegation program is a
// no-op stub here, so this checks only that the instruction is routed, not that it undelegates.
#[test]
fn test_external_undelegate_callback_is_routed() {
    let mut t = setup();
    let payer = t.admin.insecure_clone();
    // Anchor discriminator: first 8 bytes of sha256("global:process_undelegation").
    let disc: [u8; 8] = [196, 28, 41, 206, 48, 37, 51, 167];
    let (buffer, _) = Pubkey::find_program_address(&[b"undelegate-buffer", t.round.as_ref()], &dlp_id());
    let mut data = disc.to_vec();
    data.extend_from_slice(&0u32.to_le_bytes()); // account_seeds: an empty Vec<Vec<u8>>
    let ix = Instruction::new_with_bytes(
        jhandi_munda::id(),
        &data,
        vec![
            AccountMeta::new(t.round, false),
            AccountMeta::new_readonly(buffer, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
        ],
    );
    if let Err(e) = send(&mut t.svm, &payer, ix) {
        let text = format!("{:?} {:?}", e.err, e.meta.logs);
        assert!(
            !text.contains("Custom(101)") && !text.contains("InstructionFallbackNotFound"),
            "the undelegate callback must be routed, not fall back: {text}"
        );
    }
}

// --- round_v2 (D41): init, and clear undelegate errors ---

fn init_round_by(t: &mut Table, signer: &Keypair) -> TransactionResult {
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::InitRound {
                config: t.config,
                round: t.round,
                payer: signer.pubkey(),
                system_program: SYS,
            },
            jhandi_munda::instruction::InitRound {},
        ),
    )
}

#[test]
fn test_init_round_is_table_authority_only_and_not_twice() {
    let mut t = setup();
    let admin = t.admin.insecure_clone();
    let stranger = new_player(&mut t);
    // initialize_table already created round_v2, so init_round cannot create it again.
    assert!(init_round_by(&mut t, &stranger).is_err(), "a stranger cannot init the round");
    assert!(init_round_by(&mut t, &admin).is_err(), "round_v2 already exists: init is refused");
}

#[test]
fn test_undelegate_on_an_account_already_undelegating_says_so() {
    let mut t = setup();
    let admin = t.admin.insecure_clone();
    let alice = new_player(&mut t);
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    assert!(buy_chips(&mut t, &alice, 5_000_000).is_ok());

    // The rollup copy the delegation program owns: the rollup has already scheduled the undelegate.
    let mut round = t.svm.get_account(&t.round).unwrap();
    round.owner = dlp_id();
    t.svm.set_account(t.round, round).unwrap();
    let e = undelegate_round_by(&mut t, &admin).expect_err("an undelegating round is refused");
    let logs = format!("{:?}", e.meta.logs);
    assert!(logs.contains("RoundAlreadyUndelegating"), "the error names the cause: {logs}");
    assert!(!logs.contains("cannot be migrated"), "not the migration message: {logs}");

    // Same for a chip stack the delegation program owns.
    let chips = Pubkey::find_program_address(&[jhandi_munda::CHIPS_SEED, alice.pubkey().as_ref()], &jhandi_munda::id()).0;
    let mut stack = t.svm.get_account(&chips).unwrap();
    stack.owner = dlp_id();
    t.svm.set_account(chips, stack).unwrap();
    let e = undelegate_chips_by(&mut t, &admin, &alice.pubkey()).expect_err("an undelegating stack is refused");
    let logs = format!("{:?}", e.meta.logs);
    assert!(logs.contains("ChipStackAlreadyUndelegating"), "the error names the cause: {logs}");
}


// --- open_chip_bet, place_bet_chips on an opened slip, delegate_bet, undelegate_bet (D42) ---

fn open_chip_bet(t: &mut Table, bettor: &Keypair, round_id: u64, symbol: u8) -> TransactionResult {
    send(
        &mut t.svm,
        bettor,
        ix(
            jhandi_munda::accounts::OpenChipBet {
                round: t.round,
                bet: bet_pda_r(round_id, bettor, symbol),
                bettor: bettor.pubkey(),
                system_program: SYS,
            },
            jhandi_munda::instruction::OpenChipBet { round_id, symbol },
        ),
    )
}

fn delegate_bet_by(t: &mut Table, signer: &Keypair, round_id: u64, symbol: u8) -> TransactionResult {
    let bet = bet_pda_r(round_id, signer, symbol);
    let buffer = pda_with(&[ephemeral_rollups_sdk::pda::DELEGATE_BUFFER_TAG, bet.as_ref()], &jhandi_munda::id());
    let record = pda_with(&[ephemeral_rollups_sdk::pda::DELEGATION_RECORD_TAG, bet.as_ref()], &dlp_id());
    let metadata = pda_with(&[ephemeral_rollups_sdk::pda::DELEGATION_METADATA_TAG, bet.as_ref()], &dlp_id());
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::DelegateBet {
                owner: signer.pubkey(),
                bet,
                buffer_bet: buffer,
                delegation_record_bet: record,
                delegation_metadata_bet: metadata,
                owner_program: jhandi_munda::id(),
                delegation_program: dlp_id(),
                system_program: SYS,
            },
            jhandi_munda::instruction::DelegateBet { round_id, symbol },
        ),
    )
}

fn undelegate_bet_by(t: &mut Table, signer: &Keypair, round_id: u64, player: &Pubkey, symbol: u8) -> TransactionResult {
    send(
        &mut t.svm,
        signer,
        ix(
            jhandi_munda::accounts::UndelegateBet {
                payer: signer.pubkey(),
                config: t.config,
                bet: pda_with(
                    &[BET_SEED, &round_id.to_le_bytes(), player.as_ref(), &[symbol]],
                    &jhandi_munda::id(),
                ),
                magic_context: magic_context_id(),
                magic_program: magic_program_id(),
            },
            jhandi_munda::instruction::UndelegateBet { round_id, player: *player, symbol },
        ),
    )
}

fn slip(t: &Table, bettor: &Keypair, symbol: u8) -> Bet {
    let data = t.svm.get_account(&bet_pda_r(round(t).round_id, bettor, symbol)).unwrap().data;
    Bet::try_deserialize(&mut data.as_slice()).unwrap()
}

#[test]
fn test_open_chip_bet_creates_an_empty_slip() {
    let mut t = setup();
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    let alice = new_player(&mut t);
    assert!(buy_chips(&mut t, &alice, 5_000_000).is_ok());

    assert!(open_chip_bet(&mut t, &alice, 0, 2).is_ok(), "the bettor opens a slip");
    let acc = t.svm.get_account(&bet_pda_r(0, &alice, 2)).unwrap();
    assert_eq!(acc.owner, jhandi_munda::id());
    let b = slip(&t, &alice, 2);
    assert_eq!((b.player, b.amount, b.symbol, b.round_id), (alice.pubkey(), 0, 2, 0), "empty, for this bettor and face");
    assert!(b.paid_with_chips && !b.settled, "a chip slip, not yet settled");

    // A wrong round id, an invalid face, and a second open are all refused.
    assert!(open_chip_bet(&mut t, &alice, 7, 3).is_err(), "a round id that is not the current round");
    assert!(open_chip_bet(&mut t, &alice, 0, 7).is_err(), "face 7 is not a face");
    assert!(open_chip_bet(&mut t, &alice, 0, 2).is_err(), "the slip already exists");
}

#[test]
fn test_place_bet_chips_fills_the_slip_and_creates_nothing() {
    let mut t = setup();
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    let alice = new_player(&mut t);
    assert!(buy_chips(&mut t, &alice, 5_000_000).is_ok());
    assert!(open_chip_bet(&mut t, &alice, 0, 2).is_ok());

    let bet = bet_pda_r(0, &alice, 2);
    let before = t.svm.get_account(&bet).unwrap();
    let chips_before = chip_balance(&t, &alice);
    assert!(place_chip_bet_raw(&mut t, &alice, bet, 2, 1_000_000).is_ok(), "fills the opened slip");

    // The same account, same size, same rent: nothing was created or paid for.
    let after = t.svm.get_account(&bet).unwrap();
    assert_eq!((after.lamports, after.data.len()), (before.lamports, before.data.len()), "no rent moved");
    let b = slip(&t, &alice, 2);
    assert_eq!((b.amount, b.symbol, b.paid_with_chips), (1_000_000, 2, true));
    assert_eq!(chip_balance(&t, &alice), chips_before - 1_000_000, "the stake comes off the chips");
    assert!(round(&t).players.contains(&alice.pubkey()), "the bettor takes a seat");

    // A filled slip cannot be filled again.
    let e = place_chip_bet_raw(&mut t, &alice, bet, 2, 1_000_000).expect_err("a second fill is refused");
    assert!(format!("{:?}", e.meta.logs).contains("BetSlipAlreadyFilled"), "{:?}", e.meta.logs);
}

#[test]
fn test_place_bet_chips_fails_when_the_slip_was_not_opened() {
    let mut t = setup();
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    let alice = new_player(&mut t);
    assert!(buy_chips(&mut t, &alice, 5_000_000).is_ok());

    // No open_chip_bet for face 3: the place fails and creates nothing (the rollup can't pay rent).
    let bet = bet_pda_r(0, &alice, 3);
    assert!(place_chip_bet_raw(&mut t, &alice, bet, 3, 1_000_000).is_err(), "no slip, no bet");
    assert!(t.svm.get_account(&bet).is_none(), "the failed place created no account");
    assert_eq!(chip_balance(&t, &alice), 5_000_000, "no chips were taken");
}

#[test]
fn test_stranger_cannot_fill_someone_elses_slip() {
    let mut t = setup();
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    let alice = new_player(&mut t);
    let bob = new_player(&mut t);
    assert!(buy_chips(&mut t, &alice, 5_000_000).is_ok());
    assert!(buy_chips(&mut t, &bob, 5_000_000).is_ok());
    assert!(open_chip_bet(&mut t, &alice, 0, 4).is_ok());

    // Bob signs, but the slip is Alice's: the seeds include her key, so Bob cannot fill it.
    let alice_slip = bet_pda_r(0, &alice, 4);
    assert!(place_chip_bet_raw(&mut t, &bob, alice_slip, 4, 1_000_000).is_err(), "a stranger cannot fill it");
    assert_eq!(slip(&t, &alice, 4).amount, 0, "Alice's slip is still empty");
    assert_eq!(chip_balance(&t, &bob), 5_000_000, "Bob's chips are untouched");
}

#[test]
fn test_delegate_and_undelegate_a_chip_slip() {
    let mut t = setup();
    stub_vrf(&mut t);
    t.svm.add_program(dlp_id(), include_bytes!("fixtures/vrf_stub.so")).unwrap();
    t.svm.add_program(magic_program_id(), include_bytes!("fixtures/vrf_stub.so")).unwrap();
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    let alice = new_player(&mut t);
    let stranger = new_player(&mut t);
    let admin = t.admin.insecure_clone();
    assert!(buy_chips(&mut t, &alice, 5_000_000).is_ok());
    assert!(open_chip_bet(&mut t, &alice, 0, 5).is_ok());
    let slip_key = bet_pda_r(0, &alice, 5);
    let original = t.svm.get_account(&slip_key).unwrap();

    // Only the slip's owner delegates it.
    assert!(delegate_bet_by(&mut t, &stranger, 0, 5).is_err(), "a stranger cannot delegate Alice's slip");
    assert!(delegate_bet_by(&mut t, &alice, 0, 5).is_ok(), "the owner delegates the slip");
    // The SDK zeroes the PDA's data on delegation (cpi.rs); the rollup holds the real bytes. Model
    // that rollup copy here: owned by this program, with the slip's own data.
    assert_eq!(t.svm.get_account(&bet_pda_r(0, &alice, 5)).unwrap().owner, dlp_id(), "the slip moved to the rollup");

    // Bringing it home: the owner or the table authority; a stranger is refused.
    assert!(undelegate_bet_by(&mut t, &stranger, 0, &alice.pubkey(), 5).is_err(), "a stranger cannot undelegate it");

    // The rollup's view of the slip: owned by this program, so undelegate is allowed.
    let mut acc = original.clone();
    acc.owner = jhandi_munda::id();
    t.svm.set_account(slip_key, acc.clone()).unwrap();
    assert!(undelegate_bet_by(&mut t, &alice, 0, &alice.pubkey(), 5).is_ok(), "the owner brings it home");
    assert!(undelegate_bet_by(&mut t, &admin, 0, &alice.pubkey(), 5).is_ok(), "the table authority can too");

    // Already undelegating (owned by the delegation program): named, not the migration message.
    acc.owner = dlp_id();
    t.svm.set_account(slip_key, acc).unwrap();
    let e = undelegate_bet_by(&mut t, &alice, 0, &alice.pubkey(), 5).expect_err("already undelegating");
    assert!(format!("{:?}", e.meta.logs).contains("BetAlreadyUndelegating"), "{:?}", e.meta.logs);
}

// --- D43: orphan MUSD (no shares, MUSD in the vault) ---

// Moves MUSD straight into the pool vault, like a lost stake landing in an empty pool.
fn musd_transfer_to_vault(t: &mut Table, from: &Keypair, amount: u64) -> TransactionResult {
    let mut data = vec![3u8]; // SPL token Transfer
    data.extend_from_slice(&amount.to_le_bytes());
    let transfer = Instruction::new_with_bytes(
        SPL_TOKEN_PROGRAM_ID,
        &data,
        vec![
            AccountMeta::new(jhandi_munda::musd_ata(&from.pubkey(), &t.musd_mint), false),
            AccountMeta::new(t.vault, false),
            AccountMeta::new_readonly(from.pubkey(), true),
        ],
    );
    send(&mut t.svm, from, transfer)
}

// SPL mint supply (36..44).
fn mint_supply(t: &Table, mint: &Pubkey) -> u64 {
    let data = t.svm.get_account(mint).unwrap().data;
    u64::from_le_bytes(data[36..44].try_into().unwrap())
}

#[test]
fn test_deposit_into_orphan_vault() {
    let mut t = setup();
    let alice = new_player(&mut t); // the only LP, then gone
    let bob = new_player(&mut t); // the losing bettor whose stake lands in the empty pool
    let carol = new_player(&mut t); // the first new LP
    let dave = new_player(&mut t); // a later LP, priced pro rata
    init_insurance(&mut t, &alice);
    let share_mint = pda(&[SHARE_MINT_SEED]);
    let insurance = pda(&[INSURANCE_VAULT_SEED]);
    let shares = |t: &Table, who: &Keypair| token_amount(&t.svm, &jhandi_munda::share_ata(&who.pubkey(), &share_mint));

    // Alice is the only LP, and she withdraws everything: no shares, empty vault.
    deposit(&mut t, &alice, 1_000_000);
    let alice_shares = shares(&t, &alice);
    assert!(lp_withdraw(&mut t, &alice, alice_shares).is_ok(), "the only LP leaves");
    assert_eq!((mint_supply(&t, &share_mint), token_amount(&t.svm, &t.vault)), (0, 0));

    // A lost stake lands in the empty pool: MUSD in the vault with zero shares outstanding.
    assert!(musd_transfer_to_vault(&mut t, &bob, 1_000_000).is_ok(), "the stake lands");
    assert_eq!((mint_supply(&t, &share_mint), token_amount(&t.svm, &t.vault)), (0, 1_000_000));

    // Carol deposits 2 MUSD. The orphan 1 MUSD moves to insurance first; she owns exactly her 2 MUSD.
    let insurance_before = token_amount(&t.svm, &insurance);
    deposit(&mut t, &carol, 2_000_000);
    assert_eq!(shares(&t, &carol), 2_000_000, "1:1 with what she put in");
    assert_eq!(token_amount(&t.svm, &insurance), insurance_before + 1_000_000, "the orphan MUSD is in insurance");
    assert_eq!(token_amount(&t.svm, &t.vault), 2_000_000, "the vault holds only Carol's deposit");
    assert_eq!(mint_supply(&t, &share_mint), 2_000_000);

    // Dave comes later: priced pro rata at NAV $1.00, so 1 MUSD buys 1 MUSD of shares.
    deposit(&mut t, &dave, 1_000_000);
    assert_eq!(shares(&t, &dave), 1_000_000, "pro rata at NAV $1.00");
    assert_eq!(token_amount(&t.svm, &t.vault), 3_000_000);
    assert_eq!(mint_supply(&t, &share_mint), 3_000_000);
}

// #110: pays queued chip wins. `stacks` are the ChipStacks in queue order (cage vault added here).
fn pay_queue_chips(t: &mut Table, signer: &Keypair, stacks: &[Pubkey]) -> TransactionResult {
    let mut instruction = ix(
        jhandi_munda::accounts::PayQueue {
            queue: pda(&[QUEUE_SEED]),
            pool: t.pool,
            pool_vault: t.vault,
            token_program: SPL_TOKEN_PROGRAM_ID,
        },
        jhandi_munda::instruction::PayQueue {},
    );
    for stack in stacks {
        instruction.accounts.push(AccountMeta::new(*stack, false));
        instruction.accounts.push(AccountMeta::new(cage(), false));
    }
    send(&mut t.svm, signer, instruction)
}

// #110 (D30): a chip win the pool cannot pay joins the FIFO queue; the round still settles.
#[test]
fn test_chip_win_queues_when_pool_short_then_pays_after_deposit() {
    let mut t = setup();
    stub_vrf(&mut t);
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    let a = new_player(&mut t);
    let b = new_player(&mut t);
    assert!(buy_chips(&mut t, &a, 20_000_000).is_ok());
    assert!(buy_chips(&mut t, &b, 5_000_000).is_ok());
    // No LP yet: the pool vault is empty, so Alice's win cannot be paid when it settles.
    assert_eq!(token_amount(&t.svm, &t.vault), 0);

    assert!(place_chip_bet(&mut t, &a, 2, 10_000_000).is_ok());
    assert!(place_chip_bet(&mut t, &b, 4, 1_000_000).is_ok());
    set_time(&mut t, BASE_TIME + 90);
    let ephemeral_queue: Pubkey = "5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc".parse().unwrap();
    assert!(request_roll_on(&mut t, &a, ephemeral_queue).is_ok());
    let mut randomness = [0u8; 32];
    randomness[..6].copy_from_slice(&[1, 1, 4, 5, 0, 2]);
    let request_id = round(&t).request_id;
    assert!(send_callback(&mut t, &a, randomness, request_id).is_ok());
    assert_eq!(round(&t).faces, [2, 2, 5, 6, 1, 3]);

    // Alice matches twice on face 2: owed 10 + 0.98 x 20 = 29.60, but the pool holds only the
    // 11 of stakes that just moved in. Settle must succeed: her win is queued, Bob's loss settles.
    set_time(&mut t, BASE_TIME + 100);
    let (ab, bb) = (bet_pda_r(0, &a, 2), bet_pda_r(0, &b, 4));
    let (ac, bc) = (pda(&[jhandi_munda::CHIPS_SEED, a.pubkey().as_ref()]), pda(&[jhandi_munda::CHIPS_SEED, b.pubkey().as_ref()]));
    assert!(settle_chip_bets(&mut t, &a, &[(ab, ac), (bb, bc)]).is_ok(), "a short pool does not fail the settle");
    assert_eq!(chip_balance(&t, &a), 10_000_000, "the win is queued, not credited yet");
    assert_eq!(token_amount(&t.svm, &cage()), 14_000_000, "stakes moved to the pool; nothing paid out");
    assert_eq!(token_amount(&t.svm, &t.vault), 11_000_000);
    let bob_slip = Bet::try_deserialize(&mut t.svm.get_account(&bb).unwrap().data.as_slice()).unwrap();
    assert!(bob_slip.settled, "the other bet settled in the same round");
    assert!(finish_round(&mut t, &a).is_ok(), "the round finishes with a win still queued");

    // An LP deposit funds the pool. The queued win is paid into the cage and credited to chips.
    deposit(&mut t, &funder, 100_000_000);
    let stack_a = pda(&[jhandi_munda::CHIPS_SEED, a.pubkey().as_ref()]);
    assert!(pay_queue_chips(&mut t, &a, &[stack_a]).is_ok(), "the queued chip win is paid");
    assert_eq!(chip_balance(&t, &a), 39_600_000, "Alice's win is credited to chips");
    assert_eq!(token_amount(&t.svm, &cage()), 14_000_000 + 29_600_000, "the win came out of the pool into the cage");
    // Invariant: the cage holds exactly the chips players hold.
    assert_eq!(token_amount(&t.svm, &cage()), chip_balance(&t, &a) + chip_balance(&t, &b));
}

// --- #46: program-enforced session spend cap (D12, notes.md §4) ---

fn slip_pda(round_id: u64, owner: &Pubkey, symbol: u8) -> Pubkey {
    pda(&[BET_SEED, &round_id.to_le_bytes(), owner.as_ref(), &[symbol]])
}

fn session_cap_pda(owner: &Pubkey) -> Pubkey {
    pda(&[jhandi_munda::SESSION_CAP_SEED, owner.as_ref()])
}

// The SDK's V1 session token address for (session key, wallet, this program).
fn session_token_pda(owner: &Pubkey, session_key: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            session_keys::SessionToken::SEED_PREFIX.as_bytes(),
            jhandi_munda::id().as_ref(),
            session_key.as_ref(),
            owner.as_ref(),
        ],
        &session_keys::ID,
    )
    .0
}

fn grant_session_by(t: &mut Table, owner: &Keypair, session_key: &Pubkey, cap: u64, valid_for_secs: i64) -> TransactionResult {
    send(
        &mut t.svm,
        owner,
        ix(
            jhandi_munda::accounts::GrantSession {
                owner: owner.pubkey(),
                cap: session_cap_pda(&owner.pubkey()),
                system_program: SYS,
            },
            jhandi_munda::instruction::GrantSession {
                session_signer: *session_key,
                cap,
                valid_for_secs,
            },
        ),
    )
}

// Writes the SDK's V1 SessionToken for (wallet, session key) at its PDA, owned by the session
// program, the way the session program would create it. LiteSVM has no session program binary here.
fn put_session_token(t: &mut Table, owner: &Pubkey, session_key: &Pubkey, valid_until: i64) {
    let token = session_keys::SessionToken {
        authority: *owner,
        target_program: jhandi_munda::id(),
        session_signer: *session_key,
        valid_until,
    };
    let mut data = Vec::new();
    anchor_lang::AccountSerialize::try_serialize(&token, &mut data).unwrap();
    let mut acc = t.svm.get_account(&t.round).unwrap(); // any program-owned account, reshaped
    acc.data = data;
    acc.owner = session_keys::ID;
    t.svm.set_account(session_token_pda(owner, session_key), acc).unwrap();
}

fn place_session_bet(t: &mut Table, session_key: &Keypair, owner: &Pubkey, symbol: u8, amount: u64) -> TransactionResult {
    let round_id = round(t).round_id;
    send(
        &mut t.svm,
        session_key,
        ix(
            jhandi_munda::accounts::PlaceBetChipsSession {
                table: t.table,
                round: t.round,
                bet: slip_pda(round_id, owner, symbol),
                chips: pda(&[jhandi_munda::CHIPS_SEED, owner.as_ref()]),
                cap: session_cap_pda(owner),
                session_token: session_token_pda(owner, &session_key.pubkey()),
                owner: *owner,
                session_signer: session_key.pubkey(),
            },
            jhandi_munda::instruction::PlaceBetChipsSession { symbol, amount },
        ),
    )
}

// A wallet with chips and a session key with a 15 MUSD cap for an hour.
fn session_setup() -> (Table, Keypair, Keypair) {
    let mut t = setup();
    stub_vrf(&mut t);
    let funder = new_player(&mut t);
    init_cage(&mut t, &funder);
    let alice = new_player(&mut t);
    assert!(buy_chips(&mut t, &alice, 20_000_000).is_ok());
    let session = Keypair::new();
    t.svm.airdrop(&session.pubkey(), 1_000_000_000).unwrap();
    set_time(&mut t, BASE_TIME);
    assert!(grant_session_by(&mut t, &alice, &session.pubkey(), 15_000_000, 3600).is_ok(), "the wallet grants the session");
    put_session_token(&mut t, &alice.pubkey(), &session.pubkey(), BASE_TIME + 3600);
    (t, alice, session)
}

#[test]
fn test_session_bet_under_cap() {
    let (mut t, alice, session) = session_setup();
    assert!(open_chip_bet(&mut t, &alice, 0, 2).is_ok());

    assert!(place_session_bet(&mut t, &session, &alice.pubkey(), 2, 10_000_000).is_ok(), "a session bet under the cap");
    assert_eq!(slip(&t, &alice, 2).amount, 10_000_000, "the session filled Alice's slip");
    assert_eq!(chip_balance(&t, &alice), 10_000_000, "the stake came off her chips");
    let cap = jhandi_munda::state::SessionCap::try_deserialize(
        &mut t.svm.get_account(&session_cap_pda(&alice.pubkey())).unwrap().data.as_slice(),
    )
    .unwrap();
    assert_eq!(cap.spent, 10_000_000, "the cap records what the session staked");
}

#[test]
fn test_session_bet_over_cap_rejected() {
    let (mut t, alice, session) = session_setup();
    assert!(open_chip_bet(&mut t, &alice, 0, 2).is_ok());
    assert!(open_chip_bet(&mut t, &alice, 0, 4).is_ok());
    assert!(place_session_bet(&mut t, &session, &alice.pubkey(), 2, 10_000_000).is_ok());

    // 10 + 6 = 16 MUSD is over the 15 MUSD cap: refused, nothing moves.
    let e = place_session_bet(&mut t, &session, &alice.pubkey(), 4, 6_000_000).expect_err("over the cap");
    assert!(format!("{:?}", e.meta.logs).contains("SessionCapExceeded"), "{:?}", e.meta.logs);
    assert_eq!(slip(&t, &alice, 4).amount, 0, "the refused bet filled nothing");
    assert_eq!(chip_balance(&t, &alice), 10_000_000);

    // Past the hour the session stops: the cap has no value after expiry.
    set_time(&mut t, BASE_TIME + 3600);
    assert!(place_session_bet(&mut t, &session, &alice.pubkey(), 4, 1_000_000).is_err(), "an expired session is refused");
}

#[test]
fn test_session_key_cannot_use_other_instructions() {
    let (mut t, alice, session) = session_setup();
    assert!(open_chip_bet(&mut t, &alice, 0, 2).is_ok());
    let before = chip_balance(&t, &alice);

    // Cash-out and the wallet's bet need the wallet to sign. The session key cannot reach Alice's
    // chips through them: it passes its own accounts and gets nothing.
    let cashout = send(
        &mut t.svm,
        &session,
        ix(
            jhandi_munda::accounts::CashOut {
                pool: t.pool,
                cage_vault: cage(),
                queue: pda(&[QUEUE_SEED]),
                chips: chip_stack(&session),
                owner: session.pubkey(),
                owner_musd_ata: jhandi_munda::musd_ata(&session.pubkey(), &t.musd_mint),
                token_program: SPL_TOKEN_PROGRAM_ID,
            },
            jhandi_munda::instruction::CashOut { amount: 1_000_000 },
        ),
    );
    assert!(cashout.is_err(), "a session cannot cash out");
    let bet = send(
        &mut t.svm,
        &session,
        ix(
            jhandi_munda::accounts::PlaceBetChips {
                table: t.table,
                round: t.round,
                bet: slip_pda(0, &alice.pubkey(), 2),
                chips: chip_stack(&alice),
                bettor: session.pubkey(),
                system_program: SYS,
            },
            jhandi_munda::instruction::PlaceBetChips { symbol: 2, amount: 1_000_000 },
        ),
    );
    assert!(bet.is_err(), "the wallet's bet path does not take a session signer");
    assert_eq!(chip_balance(&t, &alice), before, "Alice's chips are untouched");
}

// --- #50: daily snapshot of the house (D28) ---

fn take_snapshot_by(t: &mut Table, payer: &Keypair, day: u64) -> TransactionResult {
    send(
        &mut t.svm,
        payer,
        ix(
            jhandi_munda::accounts::TakeSnapshot {
                payer: payer.pubkey(),
                snapshot: pda(&[jhandi_munda::SNAPSHOT_SEED, &day.to_le_bytes()]),
                pool: t.pool,
                share_mint: pda(&[SHARE_MINT_SEED]),
                vault: t.vault,
                stake_pool: pda(&[jhandi_munda::STAKE_POOL_SEED]),
                queue: pda(&[QUEUE_SEED]),
                system_program: SYS,
            },
            jhandi_munda::instruction::TakeSnapshot { day },
        ),
    )
}

fn snapshot_of(t: &Table, day: u64) -> jhandi_munda::DailySnapshot {
    let data = t.svm.get_account(&pda(&[jhandi_munda::SNAPSHOT_SEED, &day.to_le_bytes()])).unwrap().data;
    jhandi_munda::DailySnapshot::try_deserialize(&mut data.as_slice()).unwrap()
}

// Two snapshots a UTC day apart with different NAVs; a third on the same day is refused (D28, #50).
#[test]
fn test_two_snapshots_a_day_apart() {
    let mut t = setup();
    let funder = new_player(&mut t);
    // The staking accounts need the MUNDA mint first (init_munda), as the staking tests do.
    init_munda(&mut t, &funder);
    init_staking(&mut t, &funder);
    let lp = new_player(&mut t);
    let keeper = new_player(&mut t); // anyone can crank
    let day1 = (BASE_TIME / 86_400) as u64;
    let day2 = ((BASE_TIME + 86_400) / 86_400) as u64;

    set_time(&mut t, BASE_TIME);
    assert!(take_snapshot_by(&mut t, &keeper, day1).is_ok(), "today's snapshot");
    assert_eq!(snapshot_of(&t, day1).nav_per_share, 1_000_000, "no shares: NAV is $1.00");

    // A day passes; in between, an LP deposits 100 MUSD and a 50 MUSD profit lands in the vault.
    deposit(&mut t, &lp, 100_000_000);
    assert!(musd_transfer_to_vault(&mut t, &keeper, 50_000_000).is_ok(), "the profit");
    set_time(&mut t, BASE_TIME + 86_400);
    assert!(take_snapshot_by(&mut t, &keeper, day2).is_ok(), "the next day's snapshot");

    let (first, second) = (snapshot_of(&t, day1), snapshot_of(&t, day2));
    assert_ne!(first.nav_per_share, second.nav_per_share, "NAV moved with the profit");
    assert_eq!(second.nav_per_share, 1_500_000, "150 MUSD over 100 MUSD of shares");
    assert_eq!(second.pool_size, 150_000_000);
    assert_eq!((second.day, second.taken_at), (day2, BASE_TIME + 86_400));
    assert_eq!((second.open_claims, second.queue_size), (0, 0), "nothing queued");

    // A third crank on the same day is refused: the day's PDA already exists.
    assert!(take_snapshot_by(&mut t, &keeper, day2).is_err(), "one snapshot per day");
    // A day that is not today is refused.
    assert!(take_snapshot_by(&mut t, &keeper, day2 + 1).is_err(), "only the current UTC day");
}
