# Phase 2 build plan

Jhandi Munda Casino, MagicBlock Solana Blitz v10. Build window: 20–27 Nov 2026. **Devnet and local only.** No mainnet, no real funds, no checking in secrets.

This plan comes from `docs/decisions.md` (D1–D32) and the MagicBlock notes in `docs/reference/magicblock/`. Versions and gotchas are in those notes. Pin **Anchor 1.0.2** and **ephemeral-rollups-sdk 0.18.x** unless we later prove a newer pair works.

---

## Game math check

Six fair dice, six faces each. A player picks one symbol. Let `k` be how many dice show that symbol. Then

```
P(k) = C(6, k) × (1/6)^k × (5/6)^(6-k) = C(6, k) × 5^(6-k) / 46,656
```

Payouts are **total return** (decision 9): bet $1 at 3x, get $3 back. 0 or 1 match pays 0. Then 2 / 3 / 4 / 5 / 6 matches pay 3x / 5x / 9x / 26x / 101x.

"Expected return per $1" in the last column is `P(k) × multiplier`. Add those up to get what a $1 bet is worth before any fee.

| k matches | P(k) exact | P(k) decimal | Total return | Contribution to expected return |
| ---: | --- | ---: | ---: | --- |
| 0 | 15,625 / 46,656 | 0.33489798 | 0 | 0 |
| 1 | 18,750 / 46,656 | 0.40187757 | 0 | 0 |
| 2 | 9,375 / 46,656 | 0.20093879 | 3x | 28,125 / 46,656 = 0.60281636 |
| 3 | | | 5x | |
| 4 | 375 / 46,656 | 0.00803755 | 9x | 3,375 / 46,656 = 0.07233796 |
| 5 | 30 / 46,656 | 0.00064300 | 26x | 780 / 46,656 = 0.01671811 |
| 6 | 1 / 46,656 | 0.00002143 | 101x | 101 / 46,656 = 0.00216478 |
| **Total** | 46,656 / 46,656 | 1 | | **44,881 / 46,656 ≈ 0.96195559** |

**Bibhu check (blank row above):** fill in the 3-match row yourself. Use `C(6, 3) × (1/6)^3 × (5/6)^3`, then multiply by 5. The total already includes that row, so your two numbers should fit the 44,881 / 46,656 sum.

### House edge (no fee)

```
Expected return = 44,881 / 46,656 ≈ 0.96195559
House edge      = 1 − that = 1,775 / 46,656 ≈ 0.03804441  →  3.804441%
```

This is the "~3.8%" from decision 9. The table is slightly in the house's favour before any extra fee.

### Decision 17: 2% fee on wins only (extended to losses by decision 45)

> **Update (D45):** the 2% fee now also applies to losing bets. It is taken from the pool's side, so a loser pays nothing extra and the odds do not change. This section's worked examples still describe the win side. Fee split stays 20/10/70.

Decision 17 does not say whether the 2% is taken from the **total payout** (the 3x / 5x / … you receive) or from the **profit** (payout minus the original $1 stake). Losers still pay nothing extra in both readings. Here are both, so we can lock one before we code the fee.

Worked $1 examples:

| Result | Total return before fee | Fee on total payout (you get 98% of the payout) | Fee on profit (you keep the $1 stake, and 98% of the gain) |
| --- | ---: | ---: | ---: |
| 2 matches | $3 | $2.94 | $2.96 |
| 3 matches | $5 | $4.90 | $4.92 |
| 4 matches | $9 | $8.82 | $8.84 |
| 5 matches | $26 | $25.48 | $25.50 |
| 6 matches | $101 | $98.98 | $99.00 |
| 0 or 1 match | $0 | $0 | $0 |

**If the 2% is on the total payout:**

```
Expected return = 0.98 × 44,881 / 46,656 = 2,199,169 / 2,332,800 ≈ 0.94271648
Effective edge  = 133,631 / 2,332,800 ≈ 0.05728352  →  5.728352%
```

**If the 2% is on the profit** (on winning bets only: you receive `1 + 0.98 × (multiplier − 1)`):

```
P(win) = P(k ≥ 2) = 12,281 / 46,656 ≈ 0.26322445
Expected return = 0.98 × E[multiplier] + 0.02 × P(win) = 14,743 / 15,552 ≈ 0.94798097
Effective edge  = 809 / 15,552 ≈ 0.05201903  →  5.201903%
```

**Recommendation:** lock **fee on profit**. It matches "take it from winners, and only from what they won." Fee on total payout also taxes the returned stake, and the edge jumps to about 5.73%. Fee on profit lands near 5.20%. Simple to code either way: fee-on-payout is `payout × 9,800 / 10,000`; fee-on-profit is `stake + (payout − stake) × 9,800 / 10,000`.

The insurance fund's 5% early-exit cut (decision 7) is a **separate, optional** charge on people who tap "Take now." It is not included in the edges above.

The 20 / 10 / 70 split (decision 15) applies to this 2% fee, and also to insurance early-exit fees (decision 14). The treasury's 20% goes into the insurance fund until the fund hits a target size (decision 18). That target is still open; pick a demo number at the staking step (for example a fixed MUSD amount).

---

## Must-have for the demo

A judge on devnet should be able to:

1. Get MUSD from the faucet and deposit some into the pool (NAV shares visible).
2. Approve a session once, then place a bet without a wallet popup on every click.
3. Join a 90-second public table (normal Ephemeral Rollup), see the bowl reveal, and watch the round settle with the 3 / 5 / 9 / 26 / 101x table and the 2% fee (wins from the winner's profit, losses from the pool's side, D45).
4. Open a claim card: a win claim shows both "Wait" and "Take now" (95%); a loss claim shows "Wait" only (D50). A loss claim mints MUNDA straight into the stake (D47).
5. Open the fairness page and follow an Explorer link for that round's VRF fulfillment.
6. Watch (or replay) the 50-round simulation in which the pool survives a 101x jackpot, using the queue if needed.

If time is tight, ship those six before anything in the stretch list. Private ER, self-tuning max-bet (already off by default on devnet, decision 6 and 16), multi-table, and the real token name are **stretch**.

---

## Build order

Do the game on Solana devnet first, then wrap the live round in a normal ER, then session keys, then polish. Private ER last.

### 1. Repo scaffold (Anchor + Next.js / TypeScript)

**What.** Create the Anchor program workspace and a Next.js + TypeScript app (decision 19). Pin Anchor 1.0.2, Solana 3.1.9, Rust 1.89, and `ephemeral-rollups-sdk` 0.18.x (features `anchor`, `vrf`; add `access-control` later only for private ER). Confirm `session-keys` 3.1.1 compiles against that Anchor pin.
**Why.** A known-good toolchain avoids the 0.20 / validator 2.0 trap in the MagicBlock notes.
**Done-when.** `anchor build` succeeds on an empty program, the Next.js app starts locally, and a README lists the pinned versions.

### 2. MUSD test mint + faucet

**What.** Create a devnet MUSD mint (6 or 9 decimals, document the choice) that we control. Add a faucet instruction or a simple faucet script/UI so anyone with a wallet can mint test MUSD to themselves (decision 21, 26). No real USDC.
**Why.** We need a $1 stand-in we fully control, and judges need an easy way to get chips.
**Done-when.** A wallet on devnet can click "Get MUSD," the balance goes up, and the mint address is visible on Solana Explorer.

### 3. Pool deposit and withdraw, with NAV shares

**What.** Anyone can deposit MUSD and receive pool shares. Withdraw burns shares for MUSD at the current NAV. The UI shows "bought at," "worth now," and "profit %" (decision 27). Start the pool empty (decision 4).
**Why.** This is the house. Everything else settles against it.
**Done-when.** A test deposits 100 MUSD, sees 100 shares at NAV $1.00, a second deposit after a mocked profit sees a higher NAV, and both can withdraw the right amount.

### 4. Round open, bet, and timer

**What.** With no round running, the first bet opens a 90-second round (decisions 5, 8, 31). Other players can join with any mix of symbols (decision 10). Min bet $1; max bet is a config value, off by default on devnet (decisions 3, 6). Time the window with a timestamp, not slot counts. One table, one round, about 10 players (decision 25).
**Why.** This is the casino table. The join window has to work before we add a real roll.
**Done-when.** Two test wallets can join the same open round, a third is rejected after the timer, and the round account shows both bets.

### 5. VRF roll

**What.** When the join window ends, anyone can request a MagicBlock VRF roll of the six dice (decision 11, 31). Start on the base-layer queue (`DEFAULT_QUEUE`); switch to `DEFAULT_EPHEMERAL_QUEUE` in step 11. Store a pending request id. Map the `[u8; 32]` into six faces without repeating the same byte. Reject unexpected callbacks. Use `#[vrf]` and `#[vrf_callback]`.
**Why.** Provably fair dice, and the host's own tool.
**Done-when.** A test round requests randomness, the callback writes six faces, and we can point at the callback transaction on Explorer.

### 6. Settle, payout table, and 2% fee (wins and losses)

**What.** After the callback, settle every bet with the total-return table 0 / 0 / 3 / 5 / 9 / 26 / 101x (decision 9). Apply the 2% fee (decisions 17, 45): on a win, 2% of the profit (using the reading locked in the math section above); on a loss, 2% of the lost stake, taken from the pool's side so the player pays nothing extra. Split every fee 20% insurance / 10% devs / 70% stakers (decision 15), even if the staker pot just accrues until staking ships. Losing bets stay in the pool.
**Why.** This is the house edge plus the extra fee, in code.
**Done-when.** A unit test (or devnet script) settles known dice and known bets, and the paid amounts and fee splits match the table to the lamport, and a loss accrues its 2% fee from the pool without charging the player more.

### 7. Claims (the shared "Wait" / "Take now" card)

**What.** Every result becomes one claim: asset (MUSD for a win, MUNDA for a loss), unlock time, and early-exit fee (decision 13). A win unlocks in full after 90 seconds, or 95% now from the insurance fund, which later collects 100% and keeps 5% (decision 7). A loss becomes MUNDA after 24 hours at 1 MUNDA per 1 MUSD lost (decisions 23, 26); a loss claim has Wait only, no Take now (decision 50). The first $1,000 of cumulative losses mints 5x; the bonus is stored in the claim when it opens, not at redeem (decision 49). MUNDA is minted only when the loss claim is redeemed, and it is minted straight into the player's stake, merged into the newest lot and dated from the loss (decision 47). Insurance does not advance MUSD for losses.
**Why.** One primitive, two uses, and the "losers become owners" story.
**Done-when.** Tests redeem a win both ways (full after 90s, 95% instantly) and a loss by Wait only (full MUNDA after 24h; Take now is refused, D50), the mint supply only rises on redeem, no loose MUNDA appears in the wallet, and a loss inside the first $1,000 mints 5x while one after it mints 1x.

### 8. FIFO payout queue

**What.** If the pool cannot pay a win (or cannot fund the later claim), the unpaid win joins a strict first-in, first-out queue (decisions 4, 30). Future losses pay the head of the queue first. No skipping.
**Why.** The game stays open with an unseeded pool, even after a fat win.
**Done-when.** A test creates a tiny pool, lands a win larger than the pool, shows the winner in the queue, then a later loss pays that winner first, not a newer one.

### 9. MUNDA staking and the fee split

**What.** Staking the MUNDA token is what earns the 70% staker slice of the 2% fee on wins and losses (decisions 15, 45). Holding alone earns nothing. Above $10,000 of pool assets (vault minus queued IOUs), half of every further gain is paid to eligible stakers by a crank like `distribute_fees`, and half stays with depositors (decision 48). Both payouts wait for an empty payout queue. While the admin transfer switch is off, staked principal is locked (decision 47). The 20% treasury slice goes into the insurance fund until a documented target is reached, then to the treasury (decision 18). The 10% dev slice goes to a dev treasury. Show a simple NAV-style "worth when you got it / worth now" for staked MUNDA (decision 27).
**Why.** This is the token economics judges will ask about.
**Done-when.** Two stakers of different sizes receive a 70% fee slice in proportion, the insurance fund grows by the 20% until the target, and an unstaked holder gets zero. A pool pushed past $10,000 pays half the excess to stakers, and the same pool with a non-empty queue pays nothing.

### 10. 24-hour staking lock

**What.** Rewards only start counting after a lot has been staked for 24 hours (decision 29). At launch `unstake` is refused until the admin transfer switch is turned on (decisions 46, 47); after that, anyone can unstake at any time. A lot minted from a Wait claim is dated from the loss, so it does not wait a second 24 hours.
**Why.** Stops people from hopping in just before a payout.
**Done-when.** With the switch off, `unstake` fails. With it on, a test that stakes and immediately unstakes earns no rewards, and a test that waits 24 hours (or a dev clock warp) does.

### 11. Normal Ephemeral Rollup integration

**What.** Delegate the live round account (and any other accounts written during the 90 seconds) to a 1.1.1 public ER (Asia or EU), using the Magic Router at `https://devnet-router.magicblock.app`. Bets and the VRF request/callback run on the ER with `DEFAULT_EPHEMERAL_QUEUE`. At settle, commit-and-undelegate so claims, the queue, and pool balances live back on Solana. Keep accounts ≤ 10,240 bytes. Do not commit on every bet. If we move MUSD inside the rollup, use Ephemeral SPL Token; otherwise keep token movement on Solana and only put round/dice state on the ER (pick one and document it).
**Why.** Fast, cheap rounds, and the flagship thing MagicBlock judges want to see (decision 20). Normal is the default.
**Done-when.** A full devnet round opens, takes bets, rolls, and settles on the ER, then the final state (dice, claims, pool NAV) is visible on Solana Explorer after undelegate.

### 12. Session keys

**What.** The player creates a session for our program once (`@magicblock-labs/gum-react-sdk` + `session-keys` 3.1.1). Later bets on the open round can be signed by the session (decision 12). The program must enforce a spend cap and allowed instructions; the session program will not. Prefer a short expiry (for example 60 minutes). Use V1 tokens if we use the current React hooks.
**Why.** "Sign in and just play," without a popup on every bet.
**Done-when.** A demo wallet approves one session, places two bets with no second popup, a bet over the cap is rejected, and revoke works.

### 13. Daily snapshots

**What.** Every 24 hours, save NAV per share, total pool size, total MUNDA staked, open claims, queue size, and how much of the pool is above the $10,000 overflow target (decisions 28, 48). Anyone can crank this once the day has turned (same "anyone can call it" spirit as settle).
**Why.** A day-by-day chart of the house, and a nice demo visual.
**Done-when.** A test (with a clock warp or two crank calls a day apart) writes two snapshots with different NAVs, and a simple chart can read them.

### 14. Fairness page

**What.** Each settled round has a public page with the six faces, the bets, the VRF request / callback, and an Explorer link (decision 24). If we can also show a proof blob from the VRF request, do it; if not, the Explorer link is the proof trail.
**Why.** Judges and players can check that we did not rig the roll.
**Done-when.** After a devnet round, the fairness page loads and the Explorer link opens the fulfillment or callback transaction.

### 15. UI with the bowl reveal

**What.** One table screen that feels like the street game: worn board, six symbols, a dealer, the covered bowl, then the reveal (`docs/characters/`, `docs/ideas.md`). During the 90 seconds: join, bet, wait, reveal, then the claim card with "Wait" and "Take now" (decisions 8, 13). Normal mode is the default; no private toggle required for the demo. Park the exact join-window vs show split as a UI setting (decision 8).
**Why.** This is what a judge will actually remember.
**Done-when.** A person can complete one full happy-path round in the browser on devnet, including the reveal and a claim, without running CLI commands.

### 16. 50-round simulation (the 101x story)

**What.** A scripted 50-round run, using the real program on devnet or a tight local harness against the same logic, that includes at least one 101x six-match hit. Show the pool, the queue, and the insurance fund before and after (decision 32). Chart it with the snapshot data if that is ready.
**Why.** Answers "the pool just paid 101x, now what?" with something a judge can see.
**Done-when.** We can replay the 50 rounds, point at the jackpot round, and show the pool still taking bets (queue and all) afterwards.

### 17. Private Ephemeral Rollup — stretch

**What.** Only after the public demo works. Delegate a **separate** private table to the TEE validator (`devnet-tee.magicblock.app`, pubkey `MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo`). Use `access-control`, `CreateEphemeralPermissionCpi`, and `getAuthToken`. Do not try to put some players of the public table on the TEE. After commit, keep a public settlement / fairness trail on Solana so decision 24 still holds.
**Why.** Decision 20 wants both modes, with private second. The docs do not support a per-player private toggle on a shared public table.
**Done-when.** One private-table round can be played by an authorized wallet, unauthorized wallets cannot read the live ER state, and the settled result is still visible on Solana.

---

## Stretch (after the must-have list)

- Private ER (step 17).
- Turning on the self-tuning max bet when the pool is stressed (decision 16). Keep it off for the default demo.
- More than one public table, or more than about 10 players (decision 25).
- A real name for the MUNDA token (decision 22, Phase 3).
- Turning MUNDA transfers on (the admin switch, decisions 46, 47) and a separate MUNDA/MUSD swap pool. Not for the demo: the swap pool is its own build and must stay apart from the house pool.
- Insurance-fund loan terms beyond "pay instant cash-outs and collect later" (still loosely open in decision 4).

---

## Suggested 7-day shape

This is a guide, not a hard timetable.

- **Day 1:** steps 1–3 (scaffold, MUSD, pool + NAV).
- **Day 2:** steps 4–6 (round, VRF, settle + fee).
- **Day 3:** steps 7–8 (claims, FIFO queue).
- **Day 4:** steps 9–10 (staking, 24h lock) and start step 11 (ER).
- **Day 5:** finish ER, session keys, fairness page (steps 11, 12, 14).
- **Day 6:** UI reveal + daily snapshots (steps 15, 13) and start the 50-round sim.
- **Day 7:** finish the sim, record Explorer links, only then try private ER if the must-have list is solid.

If a day slips, drop stretch and protect the six demo bullets at the top.
