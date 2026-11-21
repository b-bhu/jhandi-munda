# Ephemeral Rollup design (Step 11.1, #41)

Decision D20: rounds run in a normal (public) Ephemeral Rollup first. Private is on hold.

## Token-movement rule (chosen)

**MUSD never moves on the rollup. Only round and dice state is delegated.**

- Round state (the `Round` account, and later the bet and claim state for the
  current round) runs on the ER: fast, cheap, and the dice roll there.
- MUSD stays on Solana: the pool vault, the insurance and fee vaults, the stake
  vault, and every player's MUSD account. Nothing in these moves on the ER.

Why this rule:
- Plain SPL transfers cannot run on delegated accounts (notes.md, section 2, "Token
  transfers on the ER"). Moving MUSD on the rollup needs the Ephemeral SPL Token
  path. Its Rust API in SDK 0.18 is **UNVERIFIED** (the notes only name the JS
  helpers `delegateSpl` / `withdrawSpl`).
- The pool, insurance, fee and stake vaults are long-lived and read by other
  instructions. Keeping them on Solana keeps Steps 6–10 unchanged.

Consequence to settle in Step 11.3 (not decided here): a bet changes the round
on the ER, but its MUSD stake cannot move on the ER. The stake needs to be funded
on Solana in a way that does not block the round. This is the open point for 11.3.

## Delegated accounts and sizes

Only the round is delegated in this step. The cap is **10,240 bytes** per account
(notes.md, section 2, "Runtime limits"). Sizes are measured in the test
`test_delegate_round_size_under_er_cap` (account data length, including the
8-byte discriminator):

| Account | Delegated in 11.1 | Size (bytes) | Cap |
| --- | --- | ---: | ---: |
| Round (up to 10 players) | yes | 392 | 10,240 |
| Bet | no (not yet) | 75 | 10,240 |

## Validator and SDK

- SDK: `ephemeral-rollups-sdk` 0.18.x (pinned). Not 0.20.
- Validator: Asia, `MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57`, validator 1.1.1
  (notes.md, section 2 table). EU is the other 1.1.1 option.
- The delegate call is `ephemeral_rollups_sdk::cpi::delegate_account` with the
  seeds **without** the bump (the SDK appends it).

## Commit and undelegate (Step 11.4, #44)

- `undelegate_round` and `undelegate_chips(owner)` run on the rollup and CPI the
  magic program's `commit_and_undelegate`. Only the table authority (`config.admin`)
  may undelegate the round (D40). A chip stack may be undelegated only by its owner.
- A round that is old-size while delegated is handled in the devnet script: undelegate,
  wait until Solana owns it, `migrate_round` on Solana, `finish_round` if Rolled,
  delegate again. The wallet pays the rent on Solana.
- Chip money stays in the cage vault on Solana. Chip bets are settled on Solana with
  `settle_chip_bets` after the round comes home.

## Not in this step

Playing a full round on devnet was not run from here (the PR checker runs the script).
The Magic Router and the ER VRF queue are the same as in Step 11.3. See #42–#44.

## Unverified (written down, not guessed)

- The Magic Router endpoint is listed in the notes but has not been called from here.
- Whether `#[ephemeral]` is required on the program for delegation in SDK 0.18.
  Not applied in 11.1; undelegation (which uses it) comes later.
- The delegation program's real behaviour on devnet. The tests use a no-op stub at
  the delegation program's address, so only the SDK's own checks and our account
  checks run for real.
- The Ephemeral SPL Token Rust API (see above).
