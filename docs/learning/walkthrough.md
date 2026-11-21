# Code walkthrough: learn what we built (no new code)

Started 21 Nov 2026. Goal: understand the whole program before any UI work. Six modules, about 30–45 minutes each.
Each one goes: explain with a casino example, point at your own files, give references, then a few questions.
There's a short quiz after modules 2, 4 and 6. Only answers in your own words count as demonstrated.

| # | Module | Casino picture | Files | Status |
|---|--------|----------------|-------|--------|
| 1 | Accounts and PDAs: the floor plan | Every box, drawer and slip in the casino | `state.rs`, `lib.rs`, `constants.rs` | in progress |
| 2 | A round's life | Bets open, dice roll, table pays, reset | `place_bet.rs`, `request_roll.rs`, `callback_roll.rs`, `settle.rs`, `payout.rs` | |
| 3 | The house pool's money | Investors fund the bank, own shares | `pool_init.rs`, `pool_deposit.rs`, `pool_withdraw.rs`, `faucet.rs` (D27, D43, D44) | |
| 4 | Claims and the waiting line | Winner's ticket, "wait or take 95% now", IOU line | `claim_open.rs`, `claim_redeem.rs`, `payout_queue.rs` (D4, D7, D30) | |
| 5 | The fast lane | A side table that plays fast, then reports back to the main hall | `er_delegate.rs`, `undelegate.rs`, `chips.rs` (D39, D41, D42, session keys) | |
| 6 | The side systems | Fees, loyalty tokens, daily books, renovations | `fees.rs`, `staking.rs`, `snapshot.rs`, `migrate.rs`, tests | |

References:
- Solana accounts: https://solana.com/docs/core/accounts
- PDAs: https://solana.com/docs/core/pda
- Transactions and instructions: https://solana.com/docs/core/transactions
- Anchor account constraints: https://www.anchor-lang.com/docs/references/account-constraints
- MagicBlock Ephemeral Rollups: https://docs.magicblock.gg/pages/ephemeral-rollups-ers/introduction/why
- MagicBlock VRF: https://docs.magicblock.gg/pages/verifiable-randomness-functions-vrfs/introduction
- Our own notes: docs/reference/magicblock/notes.md
