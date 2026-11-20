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
