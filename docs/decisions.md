# Decisions log

Answers from the planning Q&A, in order.

1. **Audience:** crypto natives with a Solana wallet and USDC. No India or US targeting.
2. **Region blocking:** a terms line only, no geofence.
3. **Bet sizes:** $1 minimum, a max set by the pool size, keep the 100x jackpot.
4. **Pool model:** Papertrade-style pool. It starts unseeded and grows from player losses. When the pool can't pay a win, the win goes into a first-in-first-out payout queue, paid from future losses. Our treasury keeps a cut of earnings as an insurance fund, which can lend to the pool when it's short and gets paid back with something extra.
   - *Why:* a funded pool needs seed money and nobody but us has a reason to seed it. A hard 10%-of-pool cap makes a small pool unplayable ($1,000 pool allows only $1 bets). The queue keeps the game open from day one, and the insurance fund shortens the wait for winners.
   - *Open:* how much of earnings goes to the insurance fund, the loan terms, and whether there's still a max bet.
5. **Rounds:** on demand, like a casino table. With no round running, any player can start one by placing a bet. A short timer (5–10 s) then lets anyone join with any bet. The dice roll, everyone settles, and the table waits until someone starts the next round.
   - *Why:* simpler than always-on rounds (no idle rounds, no keeper bot), and it still feels like the street dealer: you walk up, he starts, others crowd in.
6. **Risk caps:** every cap (per-round exposure, max bet, slice of pool or treasury at risk) is built in as a config value. For the devnet demo they're all off by default. Tested with devnet SOL/USDC only.
   - *Why:* keep the demo simple and quick to try, while still showing judges that the safety controls exist and can be switched on.
7. **Payout timing:** a win unlocks in full after one round's length of time (a fixed timer we control), whether or not anyone plays another round. "Next round" is the feel, the clock is the rule. A player who wants it now can take 95% instantly from the insurance fund, which collects the full amount later and keeps the 5%.
   - *Why:* gives the pool one round of breathing time, adds a little discipline (a winner waits a round), keeps the wait short enough to still feel like winning, and gives the insurance fund an income.
   - *Fixed:* rounds are on demand (decision 5), so tying payout to an actual next round could leave a winner stuck. Bibhu's fix: tie it to a fixed round length instead.
8. **Round length:** 90 seconds start to finish (join window, roll, reveal and the dealer's show). This is also the payout wait from decision 7.
   - *Why:* Bibhu's call. It leaves room for the dealer show and for players to join, and it gives the pool a real breathing window.
   - *Parked:* the split between join window and show is a tunable setting; decide during UI build.
9. **Payout display:** show multipliers the crypto way, as the total you get back (bet $1 at 3x, receive $3).
   - *Why:* crypto natives are used to it (Stake, Polymarket). The draft table was in street style (profit plus bet), so to keep the ~3.8% edge the shown numbers become 3x, 5x, 9x, 26x, 101x for 2–6 matches. Jackpot stays 101x (Bibhu, 20 Nov). Edge to be re-checked together.
10. **Multi-symbol bets:** a player can bet on several symbols in the same round.
   - *Why:* that's how the street game works, and it's more fun. (Rapid fire, Bibhu, 20 Nov.)
11. **Randomness:** MagicBlock VRF rolls the six dice.
   - *Why:* provably fair, so nobody (including us) can rig a roll, and it's the hackathon host's own tool. Exact API to be checked against current MagicBlock docs before building.
12. **Session keys:** players approve once with their wallet, then bets go through without a popup each time (MagicBlock session keys).
   - *Why:* smooth, fast play, close to "sign in and just play". Bibhu wants UX framing kept front and centre in every decision. Exact API to be checked against current MagicBlock docs.
13. **One shared "claim":** every result becomes a claim with three fields: what it pays (USDC for wins, house tokens for losses), when it unlocks, and the fee to exit early. The player sees one card with two buttons, "Wait" and "Take now".
   - *Why:* the same pattern showed up for wins (decision 7) and loser tokens, so building it once saves time in a 7-day build and gives judges one clear story. Token holder rights (pool profit vs insurance fees) still open.
14. **Token holder rights:** house token holders earn a share of both the pool's edge and the insurance fund's early-exit fees.
   - *Why:* losers become real owners of the whole house, which gives them a reason to stay. Exact split between holders, depositors and treasury still open.
15. **Fee split and staking:** the house charges a fee measured in basis points (BPS; 100 BPS = 1%). The fee is split 20% treasury, 10% developers, 70% stakers. Stakers are anyone who stakes the house SPL token (Papertrade-style). Holding alone earns nothing; you must stake.
   - *Why:* clear language, rewards people who commit, and funds the treasury/insurance and the team. Fee level (X BPS) and self-tuning risk rules (adjust fees/caps when the pool is stressed, like Papertrade's risk controls) still to be designed.
16. **Self-tuning risk:** when the pool is stressed (pool balance low compared with the biggest possible payout in a round), the max bet shrinks automatically. Fee and claim wait stay fixed.
   - *Why:* the max bet cuts risk directly at the source. Raising fees or wait times punishes every player, including the small ones. (Coach recommendation, accepted by Bibhu.) Off by default on devnet per decision 6.
17. **Fee:** 200 BPS (2%) on the profit part of a win only (stake returned untouched), Papertrade-style. Effective house edge ≈ 5.2% (table 3.80% + fee). Split per decision 15.
   - *Why:* losers already lost their stake; taking it from happy winners hurts least. Note: this stacks on the ~3.8% table edge, so the total house take is higher. Re-check the full edge math together before the build.
18. **Insurance fund funding:** the treasury's 20% slice goes into the insurance fund until it reaches a target size; after that, the treasury keeps it.
   - *Why:* the fund that pays instant cash-outs grows on its own without seed money. Target size to be set with the pool math.
19. **Tech stack:** Anchor (Rust) for the Solana program, Next.js + TypeScript for the frontend.
   - *Why:* Anchor is the standard Solana framework and fits MagicBlock's examples; it hides much of the hard Rust. Next.js/TS is Bibhu's strongest ground. Versions to be checked against current docs before building.
20. **Rounds run in MagicBlock Ephemeral Rollups:** both a normal ER and a private ER are offered, and the player chooses. The UI makes the choice simple, with a sensible default.
   - *Why:* fast, cheap rounds, and the host's flagship tech. Private mode gives privacy-minded players an option. Unverified: how MagicBlock's private ER currently works and what it costs. Check the docs before building; Bibhu: build normal first, add private after; normal is the default. Update 20 Nov: private mode ON HOLD (needs its own table and clashes with the public fairness page).
21. **Bet currency:** MUSD (Munda USD), our own devnet test stablecoin pegged at $1, with a faucet we control.
22. **Staking token:** placeholder name `MUNDA`; real name picked in Phase 3.
23. **Loss claim:** turns into tokens after 24 hours, or right away with a 10% tax.
24. **Fairness page:** each round shows its VRF proof and an Explorer link.
25. **Demo scope:** one table, one round at a time, up to ~10 players per round. Multi-table after the hackathon.
   - *Why (21–25):* coach recommendations accepted in rapid fire (20 Nov): full control on devnet, no naming detour, a loss unlock that's meaningful but short, cheap proof of fairness for judges, and a scope that fits 7 days.
26. **Two tokens:** MUSD is the money you bet with (always $1). MUNDA is the house share you earn when you lose. Its price floats with house earnings, and staking it earns fees. The counting rule is 1 MUNDA minted per 1 MUSD lost, and minting happens only when a loss claim is redeemed.
   - *Why:* keeps the betting money stable and simple, while MUNDA works like a share whose value follows how well the house does. Bibhu's MUSD naming.
27. **Pool deposits and NAV:** anyone can deposit MUSD into the pool and get pool shares back. Pool profit and loss go to depositors, separate from MUNDA stakers. Shares are shown as NAV (what one share is worth now): "bought at", "worth now" and "profit %". The same view is used for MUNDA.
   - *Why:* NAV is plain language for "what you paid vs what it's worth". (Bibhu, 20 Nov.)
28. **Daily snapshots:** every 24 hours we save NAV per share, total pool size, total MUNDA staked, open claims and queue size.
   - *Why:* anyone can see how the house is doing day by day, and it gives the demo a chart. (Bibhu, 20 Nov.)
29. **Staking lock:** unstake anytime, but rewards only count after 24 hours staked.
   - *Why:* stops people from jumping in right before a payout.
30. **Payout queue order:** strict first in, first out, with no skipping.
   - *Why:* fair and easy to explain.
31. **Starting and settling rounds:** the first bet opens a round and starts the 90 s timer. Anyone can call settle once it ends.
   - *Why:* rounds never get stuck waiting on us.
32. **Demo story:** the pitch shows a 50-round simulation where the pool survives a 101x jackpot.
   - *Why:* it answers the judge's "the pool just paid 101x, now what?" with something they can see.
   - *Why (29–32):* coach recommendations accepted in rapid fire (20 Nov).
33. **Faucet limit:** the MUSD faucet gives 100 MUSD per call, with a 1-hour cooldown per wallet (added as Step 2.4, after the basic faucet works).
   - *Why:* a PDA locks *who* can print, but not *how often*. Without a rule in the faucet, one script could print unlimited chips and fake pool deposits or wins. Coach recommendation after Bibhu's PDA answer (20 Nov).
34. **Program first, UI after:** build every program step first. Frontend-only steps (#10, #36, #47, #48, #51, #54) are labeled `ui-later` and wait until the program side is done. Steps are now coded with Claude Code on Bibhu's Mac, not Hermes.
   - *Why:* the UI reads program accounts. Building screens while those accounts are still changing means building them twice. (Bibhu, 20 Nov.)
35. **Loss Take-now tax:** the 10% cut on a loss Take-now is simply never minted, which works the same as a burn.
   - *Why:* simplest honest rule. No hidden house wallet collects it. (Step 7.3, PR #103, 20 Nov.)
36. **Win unlock:** a win claim unlocks 90 s after settle for the full 100% Wait payout.
   - *Why:* matches the round length and gives the pool one round to rebalance. (PR #103.)
37. **MUNDA decimals:** 6, the same as MUSD.
   - *Why:* 1 MUNDA for 1 MUSD lost reads cleanly, with no decimal conversion. (PR #103.)
38. **Insurance seeding:** tests pre-seed the insurance fund for now. Step 9 fills it from the treasury's 20% fee slice.
   - *Why:* lets claims be tested before staking and the fee split exist. (PR #103.)
39. **Chips at the cage (ER money model):** a player buys a chip stack once on Solana. Their MUSD moves into a separate **cage vault** (not the pool vault), and a per-player `ChipStack` PDA records the chip balance. The ChipStack is delegated to the Ephemeral Rollup with the round, and bets on the ER subtract from chips (no token transfers on the ER). At settle, each chip bet's stake moves from the cage vault to the pool vault, and wins are paid from the pool vault back into the cage vault and credited to chips. Cashing out (after the ChipStack is committed and undelegated) turns chips back into MUSD on Solana.
   - *Why:* tokens can't move inside the ER, and players want instant bets with no wallet popup each time. It's exactly how a real casino cage works, and it fits session keys (Step 12). (Bibhu, 20 Nov.) Revised the same night: unspent chips must not count as pool money, or LPs could withdraw players' chips and the max bet would be inflated (found by Blitz verify on PR #109).
40. **Who can delegate:** only the table authority (the Config admin) can delegate or undelegate the round.
   - *Why:* otherwise any wallet could push the round into the rollup and freeze the table. (Coach recommendation, 20 Nov.)
41. **Round PDA seed is versioned (`round_v2`).** The round PDA seed is `round_v2`. The v1 round (`round`) got stuck mid-undelegate on devnet before the `#[ephemeral]` callback existed, and was left delegated. `migrate_round` still targets only the v1 account (`ROUND_V1_SEED`). `init_round` (table authority only, D40) creates `round_v2` on an existing table.
   - *Why:* an account the rollup marks as undelegating cannot be reused from our side, and the seeds carry no round id, so a fresh round had to change the seed. Leaving v1 alone costs only its devnet rent. (Blitz verify, PR #109, comment 7058455144.)
42. **Chip bet slips are opened on Solana first:** for a chip bet, the player opens the empty `Bet` account on Solana (their wallet pays the rent there) and it is delegated to the ER. On the ER, `place_bet_chips` only fills in the slip and subtracts chips. After settle, the Bet is undelegated back to Solana. (D41, the `round_v2` versioned round seed, is recorded in PR #109.)
   - *Why:* the ER can't take rent from a wallet that isn't delegated, so it can't create new accounts paid by the player (found by Blitz verify on PR #109). Bibhu chose this over keeping bet slots inside the ChipStack, so the Bet PDA model (`["bet", round_id, wallet, symbol]`) stays the same for chip bets and normal bets. Trade-off he accepted: each new bet slip needs one Solana transaction before the fast ER part. (Bibhu, 21 Nov.)
