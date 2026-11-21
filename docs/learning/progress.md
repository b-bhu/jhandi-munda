# Progress

**Phase:** 2 (build on devnet). Phase 1 closed 20 Nov with 32 decisions.
**Current concept:** First-principles thinking, applied to the house pool.

## Exact next step
Step 2: MUSD test mint + faucet on devnet, with an Explorer link. First explain "accounts" (Solana stores everything in accounts, like rows in a DB that each have an owner program) using TS comparisons. Run `source scripts/env.sh` before any build.

## Open decisions queue (Phase 1)
1. Payout table and house edge (verify math)
2. Pool model: Martingaler (unseeded, FIFO queue, loser tokens) vs funded pool
3. Max bet / exposure per round from pool size
4. Round logic: timing, multiplayer, VRF request/callback, Ephemeral Rollup session
5. Tech stack choices

## Notes
- Rust/Anchor is Bibhu's stated bottleneck. In Phase 2, explain each Anchor piece in plain words, with a TypeScript comparison, before writing it.

## Now (20 Nov, afternoon)
- Hermes is on Step 2.2 (#3, faucet). Next: verify its PR on the box, then teach token accounts (ATA = a player's wallet pocket for one token) before Step 2.3.
- Small fix pending: Anchor.toml wallet path / `anchor test` on box; misleading comment in initialize.rs.

## Now (20 Nov, 5:20 PM IST)
- Program on devnet. Hermes on Step 3.3 withdraw (#8). Loop auto-merges passing PRs.
- Next teaching: Step 4 round/bet accounts; open with spaced recall (NAV at 90/100, accounts up front).

## Next (20 Nov, 8 pm)
- Build: Step 4.4, reject bets after the 90-second timer (#15), being coded by Claude Code.
- Recall question for next session: "Why did `[\"bet\", wallet]` as a bet address break multi-symbol bets? What did we add to the seeds?"
- Open item: an admin-only check on initialize_table/initialize_pool before the demo (Bibhu not asked yet to confirm).

## 21 Nov, 4:15 PM IST: program side finished (PR #115 merged, 81 tests)
Next step: **code walkthrough session before any UI** (Bibhu's request). Walk his own code file by file: lib.rs and #[ephemeral], account structs and PDAs (Round, ChipStack, Bet, Pool), deposit/withdraw share maths (D43, D44), settle and the FIFO queue, the delegate/undelegate flow, session keys. Explain first, then one question each.
Open design questions to use as systems-thinking material: NAV readout ignores queued IOUs (should be (vault - owed) / shares); 0 shares + queue owes more than the vault; strict FIFO vs small wins skipping the stuck 99 MUSD jackpot IOU.
Still to do on devnet: upgrade with PR #115 and prove the 0-payout burn clears the stuck pool.


## 21 Nov 2026, 6:30 PM IST
The walkthrough has started (plan in walkthrough.md). Bibhu wants explanations and references first, a quiz later, and speed. Module 1 (accounts and PDAs) has been taught. Open question: why do the bet seeds include round_id, the wallet AND the symbol, and what would break if symbol were dropped?
