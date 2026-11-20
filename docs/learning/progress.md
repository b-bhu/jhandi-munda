# Progress

**Phase:** 2 (build on devnet). Phase 1 closed 20 Nov with 32 decisions.
**Current concept:** First-principles thinking, applied to the house pool.

## Exact next step
Step 1 scaffold running on the box. When it builds: show Bibhu the folder layout, explaining each Anchor piece in plain words with a TypeScript comparison. Then Step 2: MUSD faucet with an Explorer link. Start the next session with the spaced-recall questions in tracker.md.

## Open decisions queue (Phase 1)
1. Payout table and house edge (verify math)
2. Pool model: Martingaler (unseeded, FIFO queue, loser tokens) vs funded pool
3. Max bet / exposure per round from pool size
4. Round logic: timing, multiplayer, VRF request/callback, Ephemeral Rollup session
5. Tech stack choices

## Notes
- Rust/Anchor is Bibhu's stated bottleneck. In Phase 2, explain each Anchor piece in plain words, with a TypeScript comparison, before writing it.
