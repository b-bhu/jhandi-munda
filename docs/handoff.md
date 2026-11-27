# Handoff: MUNDA token economics (Step 18)

Last updated 27 Nov 2026. Read this first, then `docs/decisions.md` (D45 to D49) and epic #123.

## Where we are
All the code and docs for Step 18 are on `main`, commits `153a37c` (program code and tests) and `ed31163` (docs). They were committed straight to `main`, with no pull request and no human review yet.

Tests: `cargo test -p jhandi_munda` gives 94 passed, 0 failed, 1 ignored (after #127 and #128).

**Devnet:** upgraded on 27 Nov 2026 to `main` with #127 and #128, and `migrate_config` was run (config 74 -> 91 bytes). Signatures and the round proof are in `docs/deploys.md`. `anchor test` has not been run (it needs a local validator).

```bash
source scripts/env.sh
anchor build
cargo test -p jhandi_munda
```

Tests load `target/deploy/jhandi_munda.so`, so run `anchor build` after any program edit.

## What we decided
- **D45:** the 2% fee also applies to losing bets, taken from the pool's side. The player pays nothing extra. The split stays 20% insurance, 10% devs, 70% stakers.
- **D46 and D47:** MUNDA is staked-only at launch. A loss mints MUNDA straight into the player's stake. Unstake is refused until an admin switch (`transfers_enabled`, off by default) is turned on.
- **D48:** above $10,000 of pool assets, half of the excess goes to eligible stakers.
- **D49:** 5 MUNDA per $1 lost on the first $1,000 of total losses ever, then 1:1. The bonus is fixed when the claim opens.

Why: with the fee on wins only, one MUNDA earned about 1.3 cents over its whole life (a rough estimate from the dice odds, not measured). Papertrade's token avoids this with a fee on losses and an overflow payout above a cap (docs.papertrade.xyz/learn/staking-dividends).

## The flow, as the code does it today
1. Bet and roll: stakes go to the table, six dice roll after the 90-second window.
2. Settle (`settle.rs`): each stake moves into the pool vault. A win is owed stake plus 98% of profit (fee 2% of profit). A loss is owed nothing and books a fee of 2% of the stake from pool money. Fees go into three pots (insurance, devs, stakers) that still sit in the pool vault until `distribute_fees`.
3. Winner: a chip bet is paid at once if the queue is empty and the pool can pay, otherwise it joins the FIFO queue. A normal bet gets a claim with two buttons: Wait (full after 90 s) or Take now (95% from insurance). Take now is for winners only (D50).
4. Loser: a claim is opened (anyone can open it). The MUNDA amount is fixed then (5 per $1 on the first $1,000 of losses, then 1 per $1) and unlocks 24 hours after the loss.
5. Redeem: a win pays by Wait (100%) or Take now (95%). A loss has Wait only (D50, `redeem_take_now` refuses a MUNDA claim with `LossClaimWaitOnly`), 100% of its MUNDA after the 24 hours. The MUNDA goes straight into the stake, dated from the loss.
6. Unstake is refused while `transfers_enabled` is off.
7. Value to stakers, only for stakes older than 24 hours and only while the payout queue is empty: `distribute_fees` (70% slice) and `distribute_overflow` (half of assets above $10,000).

## Open problems (start here)
- **#124 (done, Step 18.8):** loser Take now is removed (D50). Loss claims are Wait only and `LOSS_EARLY_EXIT_BPS` is gone. Tests that needed a fresh stake lot now set the lot's start time directly (`set_lot_start`), because no loss redeem makes a fresh lot any more.
- **#125 (fixed in its own PR):** overflow counted LP deposits as gains. `deposit` and `withdraw` now keep `overflow_mark` in step with LP money (D48 update). `deposit` and `withdraw` take the `config` account now.

Both were found by reading the code, not by a failing test.

## Before deploying to devnet
- The config account grew by 17 bytes, not one (`transfers_enabled` 1, `overflow_target` 8, `overflow_mark` 8): 74 -> 91. `migrate_config` now takes the 42-byte and the 74-byte layouts, keeping the admin from 74. Check the real size of the devnet config account before running it; any other length is refused. (Done on 27 Nov: it was 74 bytes and is now 91.)
- Loss claims opened before the 5x change keep their old amounts (no backfill).
- Many existing tests were rewritten for the new rules (loss MUNDA now stakes, 5x amounts, new fee totals). Read the test diff in `153a37c`.
- Per-issue line counts and test counts in the issues came from the agent's log and were not re-measured.

## Issues
- Epic #123, sub-issues #116 to #122, all still open.
- #116 to #121 are built on `main`. Close them after review.
- #122 (loss claim card wording): the app has no claim card yet (Step 15). Only the explainer card text (`docs/learning/cards/winlose.html`) was fixed.
- Existing issues that need a small change: #52 (snapshot: add how much of the pool is above $10,000), #59 (claim card, see #122), #64 (simulation: include the loss fee and overflow).
- README, D13 and D23 now describe the current loser flow (#121). D26 and D29 were already annotated by D47.

## Not decided or built
- Turning MUNDA transfers on (the admin switch exists, nothing uses it yet).
- A separate MUNDA/MUSD swap pool. It should stay apart from the house pool, because wins drain the pool's MUSD and would move MUNDA's price. Stretch only.
