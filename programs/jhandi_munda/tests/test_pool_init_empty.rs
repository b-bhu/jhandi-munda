use {
    anchor_lang::{
        prelude::*,
        solana_program::instruction::Instruction,
        InstructionData, ToAccountMetas,
    },
    jhandi_munda::{
        state::PoolState, CONFIG_SEED, MUSD_DECIMALS, MUSD_MINT_AUTHORITY_SEED, MUSD_MINT_SEED,
        POOL_SEED, POOL_VAULT_SEED, SHARE_MINT_SEED, SPL_TOKEN_PROGRAM_ID,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

// SPL layouts (same as the faucet tests): mint supply at 36, decimals at 44;
// token account mint at 0..32, owner at 32..64, amount at 64.
const MINT_SPACE: usize = 82;
const MINT_SUPPLY_OFFSET: usize = 36;
const MINT_DECIMALS_OFFSET: usize = 44;
const TOKEN_ACCOUNT_SPACE: usize = 165;
const TOKEN_MINT_OFFSET: usize = 0;
const TOKEN_OWNER_OFFSET: usize = 32;
const TOKEN_AMOUNT_OFFSET: usize = 64;

// Runs Step 2's `initialize` (config + MUSD mint) and then `initialize_pool`,
// returning everything the tests derive addresses from.
fn setup() -> (LiteSVM, Pubkey, Pubkey, Pubkey, Pubkey) {
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

    // Step 2 first: the config and the one dollar mint this pool reuses.
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

    // The pool init: creates pool state, vault, and share mint, all empty.
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

    // Fresh blockhash: the previous transaction must not collide.
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "initialize_pool failed: {:?}", res.err());

    (svm, pool, vault, musd_mint, share_mint)
}

fn pubkey_at(data: &[u8], offset: usize) -> Pubkey {
    Pubkey::from(<[u8; 32]>::try_from(&data[offset..offset + 32]).unwrap())
}

fn u64_at(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(<[u8; 8]>::try_from(&data[offset..offset + 8]).unwrap())
}

#[test]
fn test_pool_init_empty() {
    let (svm, pool, vault, musd_mint, share_mint) = setup();

    // The pool account exists, belongs to the program, decodes as PoolState,
    // and wires together exactly the Step 2 MUSD mint, the vault and the
    // share mint derived from the seeds.
    let (pool_bump, vault_bump, share_mint_bump) = {
        let program_id = jhandi_munda::id();
        (
            Pubkey::find_program_address(&[POOL_SEED], &program_id).1,
            Pubkey::find_program_address(&[POOL_VAULT_SEED], &program_id).1,
            Pubkey::find_program_address(&[SHARE_MINT_SEED], &program_id).1,
        )
    };
    let pool_acc = svm.get_account(&pool).expect("pool PDA must exist");
    assert_eq!(
        pool_acc.owner,
        jhandi_munda::id(),
        "pool PDA must belong to the program"
    );
    assert_eq!(
        pool_acc.data.len(),
        8 + PoolState::INIT_SPACE,
        "pool space must stay at the declared minimum"
    );
    let mut data: &[u8] = &pool_acc.data;
    let state = PoolState::try_deserialize(&mut data).expect("pool state must decode");
    assert_eq!(state.musd_mint, musd_mint, "pool must reuse the Step 2 MUSD mint");
    assert_eq!(state.share_mint, share_mint);
    assert_eq!(state.vault, vault);
    assert_eq!(state.bump, pool_bump, "the pool must store its canonical bump");
    assert_eq!(state.vault_bump, vault_bump);
    assert_eq!(state.share_mint_bump, share_mint_bump);

    // The vault is a real SPL token account for the Step 2 MUSD mint, owned
    // by the pool PDA, and holds 0 MUSD (D4: the pool starts unseeded).
    let vault_acc = svm.get_account(&vault).expect("pool vault must exist");
    assert_eq!(
        vault_acc.owner, SPL_TOKEN_PROGRAM_ID,
        "vault owner must be the SPL token program"
    );
    assert_eq!(
        vault_acc.data.len(),
        TOKEN_ACCOUNT_SPACE,
        "vault must be a plain SPL token account"
    );
    assert_eq!(
        pubkey_at(&vault_acc.data, TOKEN_MINT_OFFSET),
        musd_mint,
        "vault must hold the Step 2 MUSD mint"
    );
    assert_eq!(
        pubkey_at(&vault_acc.data, TOKEN_OWNER_OFFSET),
        pool,
        "vault authority must be the pool PDA"
    );
    assert_eq!(
        u64_at(&vault_acc.data, TOKEN_AMOUNT_OFFSET),
        0,
        "vault must start empty"
    );

    // The share mint is a separate LP mint (not a second dollar mint), with
    // the pool PDA as its authority, 6 decimals like MUSD, and 0 supply.
    let share_acc = svm.get_account(&share_mint).expect("pool share mint must exist");
    assert_eq!(
        share_acc.owner, SPL_TOKEN_PROGRAM_ID,
        "share mint owner must be the SPL token program"
    );
    assert_eq!(
        share_acc.data.len(),
        MINT_SPACE,
        "share mint must be a plain SPL mint account"
    );
    assert_ne!(share_mint, musd_mint, "the LP mint must not be the MUSD mint");
    assert_eq!(&share_acc.data[0..4], &[1, 0, 0, 0], "share mint authority not set");
    assert_eq!(
        pubkey_at(&share_acc.data, 4),
        pool,
        "share mint authority must be the pool PDA"
    );
    assert_eq!(
        u64_at(&share_acc.data, MINT_SUPPLY_OFFSET),
        0,
        "share supply must start at 0"
    );
    assert_eq!(
        share_acc.data[MINT_DECIMALS_OFFSET],
        MUSD_DECIMALS,
        "shares use the same 6 decimals as MUSD so 1 share is $1.00 at empty NAV"
    );

    // Nothing ever minted MUSD in this test: the dollar mint from Step 2 is
    // untouched (only the faucet mints MUSD; the pool starts unseeded).
    let musd_acc = svm.get_account(&musd_mint).expect("MUSD mint must exist");
    assert_eq!(
        u64_at(&musd_acc.data, MINT_SUPPLY_OFFSET),
        0,
        "initializing the pool must not mint any MUSD"
    );

    // The NAV convention: with a 0 vault and 0 shares, NAV is $1.00 by
    // definition (1_000_000 = one share in MUSD base units).
    assert_eq!(
        jhandi_munda::nav_per_share(0, 0),
        1_000_000,
        "empty-pool NAV must be defined as $1.00"
    );
}
