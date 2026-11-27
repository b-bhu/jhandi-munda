# Handoff: MUNDA token economics (Step 18)

Last updated 26 Nov 2026. Read this first, then `docs/decisions.md` (D45 to D49) and epic #123.

## Where we are
All the code and docs for Step 18 are on `main`, commits `153a37c` (program code and tests) and `ed31163` (docs). They were committed straight to `main`, with no pull request and no human review yet.

Tests: `cargo test -p jhandi_munda` gives 91 passed, 0 failed, 1 ignored. `anchor test` has not been run (it needs a local validator).

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
3. Winner: a chip bet is paid at once if the queue is empty and the pool can pay, otherwise it joins the FIFO queue. A normal bet gets a claim: Wait (full after 90 s) or Take now (95% from insurance).
4. Loser: a claim is opened (anyone can open it). The MUNDA amount is fixed then (5 per $1 on the first $1,000 of losses, then 1 per $1) and unlocks 24 hours after the loss.
5. Redeem: Wait gives 100%, Take now gives 90% (the 10% is never minted). The MUNDA goes straight into the stake, dated from the loss.
6. Unstake is refused while `transfers_enabled` is off.
7. Value to stakers, only for stakes older than 24 hours and only while the payout queue is empty: `distribute_fees` (70% slice) and `distribute_overflow` (half of assets above $10,000).

## Open problems (start here)
- **#124: loser "Take now" is strictly worse than Wait.** Both stake the MUNDA and start earning at the same moment, and unstake is locked either way. Take now only gives 10% fewer tokens. Decision needed: remove it, or give it a real benefit.
- **#125 (fixed in its own PR):** overflow counted LP deposits as gains. `deposit` and `withdraw` now keep `overflow_mark` in step with LP money (D48 update). `deposit` and `withdraw` take the `config` account now.

Both were found by reading the code, not by a failing test.

## Before deploying to devnet
- The config account grew by one byte (the transfer switch). `migrate_config` only handles the oldest layout, so the devnet config needs its own migration.
- Loss claims opened before the 5x change keep their old amounts (no backfill).
- Many existing tests were rewritten for the new rules (loss MUNDA now stakes, 5x amounts, new fee totals). Read the test diff in `153a37c`.
- Per-issue line counts and test counts in the issues came from the agent's log and were not re-measured.

## Issues
- Epic #123, sub-issues #116 to #122, all still open.
- #116 to #121 are built on `main`. Close them after review.
- #122 (loss claim card wording): the app has no claim card yet (Step 15). Only the explainer card text (`docs/learning/cards/winlose.html`) was fixed.
- Existing issues that need a small change: #52 (snapshot: add how much of the pool is above $10,000), #59 (claim card, see #122), #64 (simulation: include the loss fee and overflow).
- Check that the README and the older D13 and D23 entries do not still describe the old loser flow.

## Not decided or built
- Turning MUNDA transfers on (the admin switch exists, nothing uses it yet).
- A separate MUNDA/MUSD swap pool. It should stay apart from the house pool, because wins drain the pool's MUSD and would move MUNDA's price. Stretch only.
