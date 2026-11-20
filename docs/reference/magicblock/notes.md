# MagicBlock notes for Jhandi Munda Casino

Plain-English notes from the docs snapshot in this folder (fetched 20 Nov 2026, 11:20 AM IST, Asia/Calcutta). Anything not clearly stated in those pages is marked **UNVERIFIED**.

Recommended versions for this build are in [README.md](README.md): pin `ephemeral-rollups-sdk` **0.18.x** and (unless we later prove otherwise) Anchor **1.0.2**, matching MagicBlock's current examples.

---

## 1. VRF (the dice roll)

### What it is

A verifiable random function. Our program asks for a random 32-byte value. An oracle computes it with a proof. The MagicBlock VRF program checks that proof, then calls back into our program with the bytes. We turn those bytes into six dice faces. Nobody, including us, can pick the result after the bet is in.

VRF works on Solana itself and inside an Ephemeral Rollup. Official examples: https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/roll-dice/anchor and the live apps https://roll-dice.magicblock.app/ (base layer) and https://roll-dice.magicblock.app/delegated (inside an ER, claimed under 100 ms).

The math is a VRF on Curve25519's Ristretto group, with a Schnorr-like proof (RFC 9381).

### Crate / package and version

- Rust: `ephemeral-rollups-sdk` **0.18**, features `anchor,vrf`.
  - `cargo add ephemeral-rollups-sdk@0.18 --features anchor,vrf`
- Also enable Anchor `init-if-needed` if the init instruction uses it.
- There is no separate "vrf-only" crate in the current quickstart. The VRF module lives inside this SDK (`ephemeral_rollups_sdk::vrf`). The crate's `vrf` feature pulls in `ephemeral-vrf-sdk`.

**Gotcha:** do not install `0.20`. On `0.20.0` the `#[vrf]` macro fails to compile (`cannot find slot_hashes in sysvar`). `create_request_scoped_randomness_ix` from SDK 0.16 was removed in 0.17; the current helper is `create_request_randomness_ix_const`.

### Key instructions, macros, and SDK calls

Program side:

- Macros: `#[vrf]` on the request accounts, `#[vrf_callback]` on the callback accounts.
- Types: `RequestRandomnessParams`, `SerializableAccountMeta`.
- Helpers: `create_request_randomness_ix_const`, `program_identity_pubkey`, then `ctx.accounts.invoke_signed_vrf(...)`.
- Convert bytes: `ephemeral_rollups_sdk::vrf::rnd::random_u8_with_range(&randomness, 1, 6)` (one die). For six dice, derive six values from the same `[u8; 32]` — the exact "six independent faces from one 32-byte value" recipe is **UNVERIFIED** in the docs (we should not reuse the same byte six times).
- Typical flow: `roll_dice` sends the request with a `caller_seed`, callback discriminator, and the accounts the callback will need. `callback_roll_dice` receives `randomness: [u8; 32]` and writes the result.

Client / tests: pass the matching `oracle_queue`. Watch for the callback transaction rather than polling inside the program.

### Program IDs and queues (use the SDK constants, don't hardcode)

From `ephemeral_rollups_sdk::vrf::consts`:

| Constant | Purpose | Address |
| --- | --- | --- |
| `VRF_PROGRAM_ID` | VRF program | `Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz` |
| `VRF_PROGRAM_IDENTITY` | Legacy global callback signer. **Deprecated.** Do not check this in new callbacks. | `9irBy75QS2BN81FUgXuHcjqceJJRuc9oDkAe8TKVvvAw` |
| `DEFAULT_QUEUE` | Base-layer queue, mainnet and devnet | `Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh` |
| `DEFAULT_EPHEMERAL_QUEUE` | ER queue, mainnet and devnet | `5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc` |
| `DEFAULT_TEST_QUEUE` | Base-layer queue, localnet | `GKE6d7iv8kCBrsxr78W3xVdjGLLLJnxsGiuzrsZCGEvb` |
| `DEFAULT_EPHEMERAL_TEST_QUEUE` | ER queue, localnet | `Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT` |

Mainnet and devnet share the same queue addresses; only the cluster you connect to changes. Use `DEFAULT_QUEUE` when the request runs on Solana, and `DEFAULT_EPHEMERAL_QUEUE` when it runs inside an ER. We do not delegate queues ourselves.

New requests are scoped: the VRF program signs the callback with `PDA(["identity", your_program_id], VRF_PROGRAM_ID)`. `#[vrf_callback]` checks that signer. If we add our own `vrf_program_identity` field and check it against `VRF_PROGRAM_IDENTITY`, the macro skips its check and every new request will be rejected.

### Fees and timing

- ER queue requests (`DEFAULT_EPHEMERAL_QUEUE`) are **free**.
- Base-layer Solana requests cost 0.0008 SOL (faster, under 500 ms) or 0.0005 SOL (about half a second). The fee is taken when the request is made and is not refunded if the request expires.
- The request and the callback are two separate transactions. The oracle fulfills in a later slot. A request that is not fulfilled can be purged after about two minutes. A late callback can still arrive until someone purges it; the VRF program applies no age limit when it fulfills. Store a pending request id and ignore unexpected callbacks.

### Fairness page (decision 24)

The docs say each request includes a proof that anyone can verify on-chain, and that users can confirm fairness. The reliable public artifact we can already plan for is an Explorer link to the fulfillment / callback transaction. Whether the proof bytes are easy to read from an account and show as a "copy this proof" blob on our own fairness page is **UNVERIFIED**.

### Gotchas

- Do not let a player set the entire `caller_seed` (seed grinding). Mix in something they do not fully control.
- Consume the randomness once. Do not reuse it.
- Decision 11 (VRF for the six dice) and "VRF inside an ER" (decision 20) **do not conflict**. This is officially supported if we use the ER queue.

---

## 2. Ephemeral Rollups (delegate / commit / undelegate)

### What it is

A fast side table for Solana accounts. We temporarily hand a program account to a MagicBlock validator (delegate). While it is delegated, transactions on that account run on the rollup: low latency, and current public releases charge **0** for a normal ER transaction. When we are done, we commit the new state back to Solana and optionally undelegate, which returns ownership to our program.

Think of it as: lock the account, play quickly on the side table, write the final result back.

### Crate / package and version

- Rust: `ephemeral-rollups-sdk` **0.18**, feature `anchor`.
  - `cargo add ephemeral-rollups-sdk@0.18 --features anchor`
- JavaScript (web3.js): `@magicblock-labs/ephemeral-rollups-sdk@0.18` (npm dist-tag `legacy`).
- JavaScript (Kit): `@magicblock-labs/ephemeral-rollups-kit@0.18`.
- Local validator, if we need one: `@magicblock-labs/ephemeral-validator@1.1.1`.

`0.18.x` reads commit logs from both validator 1.1.1 (the public ERs and the TEE) and validator 2.0 (`devnet-us` today). `0.20.x` reads only the 2.0 log format.

### Key instructions, macros, and SDK calls

- `#[ephemeral]` on the program module.
- `#[delegate]` on the delegate accounts struct, with `#[account(mut, del)]` on the PDA being delegated.
- `ctx.accounts.delegate_pda(&payer, &[seeds], DelegateConfig { validator: Some(validator_pubkey), ..Default::default() })`. Always pass the validator we intend to use.
- While delegated, commit with `MagicIntentBundleBuilder::new(payer, magic_context, magic_program).commit(&[account]).build_and_invoke()?`.
- Come home with `.commit_and_undelegate(&[account]).build_and_invoke()?`.
- If the program changes the account in the same instruction as the commit, call `account.exit(&crate::ID)?` first so the CPI sees the new bytes.
- Client: `ConnectionMagicRouter` from `@magicblock-labs/ephemeral-rollups-sdk`, or `Connection` from `@magicblock-labs/ephemeral-rollups-kit`. Import `MAGIC_PROGRAM_ID` and `MAGIC_CONTEXT_ID` from the SDK rather than hardcoding. The raw pubkey values are **UNVERIFIED** here (the snapshot always uses the SDK constants).

Lifecycle in four steps:

1. **Delegate** (on Solana). Account owner becomes the Delegation Program `DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh`.
2. **Transact** on the ER (directly or through the Magic Router). The first ER transaction clones the account from Solana onto the rollup.
3. **Commit** when we want Solana to see the new state. Validators do **not** commit on their own. Our program has to schedule it.
4. **Undelegate** when we want our program to own the account again. This also commits the latest state.

### Devnet endpoints and validator pubkeys

Magic Router (preferred single entry): `https://devnet-router.magicblock.app` (mainnet: `https://router.magicblock.app`). API version 2.0.

The router does **not** implement every Solana RPC method. As of 19 Nov 2026 it answered `getAccountInfo`, `getMultipleAccounts`, `getBalance`, `getLatestBlockhash`, `getSignatureStatuses`, `getSignaturesForAddress`, `getTransaction`, `getIdentity`, `getRoutes`, `getBlockhashForAccounts`, and `getDelegationStatus`. It returned "method not found" for `getSlot`, `getBlockHeight`, `getHealth`, `getVersion`, `getGenesisHash`, `getProgramAccounts`, `getMinimumBalanceForRentExemption`, and `simulateTransaction`. Send those to Solana RPC or to the ER endpoint (`fqdn` from `getDelegationStatus`). Do not use a router-only `Connection` for simulate, rent, or `getProgramAccounts`.

| Role | Devnet URL | Validator pubkey | Validator version |
| --- | --- | --- | --- |
| Router | `https://devnet-router.magicblock.app` | (routes for us) | — |
| ER Asia | `https://devnet-as.magicblock.app` | `MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57` | 1.1.1 |
| ER EU | `https://devnet-eu.magicblock.app` | `MEUGGrYPxKk17hCr7wpT6s8dtNokZj5U2L57vjYMS8e` | 1.1.1 |
| ER US | `https://devnet-us.magicblock.app` | `MUS3hc9TCw4cGC12vHNoYcCGzJG1txjgQLZWVoeNHNd` | **2.0** |
| TEE (private ER) | `https://devnet-tee.magicblock.app` | `MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo` | 1.1.1 |
| Solana devnet | `https://api.devnet.solana.com` | — | — |

The older Anchor guide also mentions `https://devnet.magicblock.app` as "the" devnet ER endpoint. Treat that as a possible alias; prefer the regional URLs or the router. Whether `devnet.magicblock.app` still points at a 1.1.1 node is **UNVERIFIED**.

**Default for us:** Asia or EU (validator 1.1.1) plus the Magic Router. Avoid depending on `devnet-us` unless we deliberately move to SDK 0.20.

Local ER: `localhost:7799`, validator `mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev`.

### Fees and commit limits (checked in the docs on 19 Nov 2026)

- Normal ER transactions: `0` in the current release. Solana fees still apply on the base layer (delegate, and the settlement the validator posts).
- Delegating funds a deposit of about 0.0028–0.0033 SOL, depending on the PDA seeds.
- When we undelegate, a session charge of 0.003 SOL plus 0.001 SOL for each commit after the first is taken from that deposit (never more than the deposit; no debt).
- Without a delegated fee payer, an account can commit **10** times, then further commits fail (`0xA0000000`). A final commit-and-undelegate is still allowed so the account is not stuck.
- With a delegated fee payer and `magic_fee_vault`, the 10-commit cap is lifted. Live commit fees of 0.001 SOL per account start on commit 26.

### Runtime limits

- Same compute as Solana (200k CU default, 1.4M max per transaction).
- ER transactions can be up to **64 KB**. Delegation transactions still have to fit Solana's 1,232-byte limit.
- An account larger than **10,240 bytes** cannot be committed or undelegated back to Solana.
- ER slot time is about 10 ms, but the docs say this is not guaranteed. **Do not** time the 90-second round (decision 8) by counting slots. Use the clock / a unix timestamp.

### Token transfers on the ER

Plain SPL transfers of our MUSD will not just work on delegated game accounts. Moving SPL tokens on the rollup is a separate product, **Ephemeral SPL Token**, program id `SPLxh1LVZzEkX99H6rqYizhytLWPZVV296zyYDPagv2`. Flow: delegate the token account, transfer on the ER, undelegate / commit, withdraw back to a base-layer token account. Client helpers include `delegateSpl` / `withdrawSpl` in SDK 0.18 (`idempotent: false` uses the legacy vault path). This matters for bets and payouts that we want to run inside the rollup.

### Gotchas

- Keep delegated accounts at or under 10,240 bytes, or we cannot bring them home.
- Do not commit on every bet. Commit (or commit-and-undelegate) at settle, and add a delegated fee payer if we ever need more than 10 commits in a session.
- After undelegate, further ER writes on that account will not work until we delegate again.

---

## 3. Private Ephemeral Rollups

### What it is

The same Ephemeral Rollup idea, but the validator runs inside a TEE (Intel TDX). Program state on that rollup can be hidden from people who are not on a per-account member list. No ZK circuits. This is live on **devnet** at `https://devnet-tee.magicblock.app`.

Privacy is not a separate chain the player "joins" mid-round. We delegate the account to the **TEE validator** (`MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo`). Then, on the ER, our program creates an `EphemeralPermission` and can set `is_private: true` or `false`.

### Crate / package and version

Same SDK: `ephemeral-rollups-sdk` **0.18**, features `anchor` and `access-control`.

```toml
ephemeral-rollups-sdk = { version = "0.18", features = ["anchor", "access-control"] }
```

Without `access-control`, `ephemeral_rollups_sdk::access_control` does not exist and the program will not compile. The permission helpers need SDK v0.14.2 or later. Pin 0.18.x: public TEE endpoints run validator 1.1.1.

### Key instructions, macros, and SDK calls

- Permission program: `ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1`.
- Delegation program: the same `DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh`.
- On the ER, signed by the delegated PDA itself:
  - `CreateEphemeralPermissionCpi` — `init_permission` (idempotent).
  - `UpdateEphemeralPermissionCpi` — `set_privacy(is_private)` and replace the whole member list.
  - `CloseEphemeralPermissionCpi`.
- There is no base-layer permission account to create or delegate.
- `is_private: false` means the account is public, even if a member list exists. `is_private: true` means only members can read it.
- Member flags include `AUTHORITY`, `TX_LOGS`, `TX_BALANCES`, `TX_MESSAGE`, `ACCOUNT_SIGNATURES`.
- Client, before talking to the TEE:
  1. `verifyTeeRpcIntegrity(EPHEMERAL_RPC_URL)` from `@magicblock-labs/ephemeral-rollups-sdk` (checks attestation via `https://pccs.phala.network/tdx/certification/v4`).
  2. `const { token, expiresAt } = await getAuthToken(EPHEMERAL_RPC_URL, wallet.publicKey, signMessage)`.
  3. Connect to `https://devnet-tee.magicblock.app?token=${token}`. Pass the **string** `token`, not the whole object, or the TEE returns `401 InvalidToken`.

TEE RPC endpoints may require that token for some methods.

### Devnet availability

**Verified:** Private ER is available on devnet (`https://devnet-tee.magicblock.app`, validator pubkey above). It is also listed on mainnet (`https://mainnet-tee.magicblock.app`). We can run and test a local Private ER with a Query Filtering Service that emulates the TEE, with no hardware attestation.

### Gotchas and conflicts with our decisions

- A PDA is delegated to **one** validator. The same table account cannot be on the Asia/EU public ER and the TEE at the same time. Decision 20 ("the player chooses normal or private") cannot mean a per-player toggle on a shared public table. It has to mean a **separate private table** (delegated to the TEE), or running the table on the TEE and flipping `is_private`. Decision 20 already says: build normal first, add private after, normal is the default. Keep private as stretch.
- A shared 10-player table (decision 25) with `is_private: true` will hide state from anyone not on the member list, including other players and our public fairness page.
- Decision 24 (every round shows a VRF proof and an Explorer link) sits badly with private mode, because logs and accounts can be hidden from non-members. A public fairness page and a private table fight each other unless we keep the proof / settlement visible on Solana after commit.
- Pricing / extra cost of private vs normal, beyond normal ER commit/deposit fees, is **UNVERIFIED** in the pages we read.

---

## 4. Session Keys

### What it is

The player approves once. The app creates a short-lived extra keypair, scoped to our program, with an expiry (default 60 minutes, at most one week). Later bets can be signed by that key, so the wallet popup does not appear every time. This is a session, not a burner wallet.

Two parts: the ephemeral keypair (kept in the browser) and a Session Token PDA that stores the user's wallet, the target program, the session signer, and the expiry.

An official demo "runs on both the base layer and the Ephemeral Rollup using session keys": https://counter-session-keys-example.magicblock.app/ with source at https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/session-keys. So session keys **with** ERs are documented as working.

### Crate / package and version

- Rust: `session-keys` **3.1.1** (crates.io, last published 26 May 2026), feature `no-entrypoint`.
  - `session-keys = { version = "3.1.1", features = ["no-entrypoint"] }`
- Frontend: `@magicblock-labs/gum-react-sdk`. The current npm `latest` tag is **3.0.10** (26 Nov 2025). Whether that JS package is fully in sync with Rust `3.1.1` is **UNVERIFIED**.
- On-chain session program: `KeyspM2ssCJbqUhQ4k7sveSiY4WjnYsrXkC8oDbwde5`.
- Test helper seen in the docs: `createSessionToken` from `@session-keys/anchor`. Current npm version of that helper is **UNVERIFIED**.

### Key instructions, macros, and SDK calls

Program side:

- `use session_keys::{SessionError, SessionToken, session_auth_or, Session};`
- `#[derive(Accounts, Session)]` on the accounts struct.
- An optional session account:

  ```rust
  #[session(signer = signer, authority = /* the user's wallet */)]
  pub session_token: Option<Account<'info, SessionToken>>,
  ```

- On the instruction: `#[session_auth_or(ctx.accounts.foo.authority.key() == ctx.accounts.signer.key(), SessionError::InvalidToken)]`. Valid session token, or the real wallet is the signer.

Client (React):

- `yarn add @magicblock-labs/gum-react-sdk`
- `SessionWalletProvider` / `useSessionKeyManager` / `useSessionWallet`.
- `sessionWallet.createSession(targetProgramPublicKey, topUp, expiryInMinutes)` — `topUp` can be `true` (0.01 SOL) or a lamport amount; default expiry is 60 minutes.
- `sessionWallet.signAndSendTransaction(transaction)`
- `sessionWallet.revokeSession()` — drops the local key, revokes on-chain, and returns leftover lamports.

### Gotchas

- The session program does **not** enforce a max spend, a max number of transactions, or which instructions are allowed. That is our job (important for bets, per decision 12).
- V1 `SessionToken` (what the current React hooks create) can be revoked by **any wallet**, even while it is active. V2 `SessionTokenV2` can only be revoked by its authority while active. The two types are not interchangeable (`AccountDiscriminatorMismatch`, error 3002). Stick to V1 if we use the current Gum React hooks, unless we later verify V2 support.
- The top-up defaults to 0.01 SOL, but the caller can set any amount, and the session signer can spend all of it.
- Browser storage is IndexedDB, with the keypair encrypted. A compromised browser or a malicious extension can still abuse a live session. Keep the session's powers narrow (place bets up to a cap, not withdraw the pool).
- Compatibility of `session-keys` 3.1.1 with Anchor 1.0.2 (MagicBlock's example pin) and with Anchor 1.2.2 is **UNVERIFIED**. We should confirm at scaffold time.

---

## 5. Conflicts and risks vs our decisions

These are the ones that should shape the build plan. "VRF inside an ER" and "session keys with an ER" are **not** conflicts; both are officially supported.

1. **Decision 20, player chooses normal or private.** A table PDA is delegated to one validator. A shared round cannot have some players on the public ER and some on the TEE. Treat private as a separate table, and keep it stretch. Normal first, default public.
2. **Decision 24, public fairness page, vs private ER.** Private mode can hide logs and accounts from non-members. The public Explorer / proof page needs a settlement that is visible on Solana after commit, or private tables will not meet decision 24.
3. **SDK / Anchor pin.** We must pin `ephemeral-rollups-sdk` 0.18.x (npm tag `legacy`). Unpinned installs get 0.20, which breaks VRF and only talks cleanly to validator 2.0 (`devnet-us`). MagicBlock examples still list Anchor 1.0.2, not the latest stable 1.2.2.
4. **Commit cap and account size.** Without a fee payer, only 10 commits, then we must undelegate. Accounts over 10,240 bytes cannot come home. Round state has to stay small, and we should commit at settle, not on every bet.
5. **Session keys do not cap spending.** Our program must. Also, a V1 session can be revoked by a third party. The frontend Gum package may lag the Rust crate (see versions above).
6. **MUSD on the rollup** needs the Ephemeral SPL Token path, not a plain SPL transfer. Plan for that when we add ER, or keep token movement on Solana and only put the round/dice state on the ER (design choice to lock at the ER step).
7. **90-second timer.** Do not count ER slots. Use a timestamp. VRF callbacks can arrive late; unfulfilled requests can be purged after about two minutes, so request randomness when the join window closes, not long before.

Open items still **UNVERIFIED**: the exact recipe for six independent die faces from one 32-byte VRF value; whether a standalone VRF proof blob is easy to show on our fairness page; raw `MAGIC_PROGRAM_ID` / `MAGIC_CONTEXT_ID` pubkeys (use the SDK constants); private-ER extra pricing; `session-keys` 3.1.1 vs Anchor 1.0.2 / 1.2.2 and vs `gum-react-sdk` 3.0.10; whether `https://devnet.magicblock.app` is still a good alias.
