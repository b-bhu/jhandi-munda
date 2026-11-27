# Submission notes (MagicBlock Solana Blitz v10)

For judges and for whoever submits. Everything here is checked against the repo and `docs/deploys.md`. Items marked **TODO** are not done and are not ours to invent.

## One paragraph

Jhandi Munda is the Indian street dice game (Crown and Anchor) as an onchain casino on Solana devnet. Six symbol dice are rolled by MagicBlock VRF after a 90-second table, so the roll can be checked by anyone. Players bet test MUSD against one shared house pool that anyone can fund. Wins pay in MUSD (Wait, or Take now at 95%); losses become MUNDA, a house token that goes straight into the player's stake after 24 hours, and is not a refund. Fees go to MUNDA stakers, an insurance fund and the developers. When the pool cannot pay a win, the win waits in a strict first-in, first-out queue. Devnet only, test money only.

## Links

| What | Link |
|---|---|
| Repo | https://github.com/b-bhu/jhandi-munda |
| Program on devnet | https://explorer.solana.com/address/4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH?cluster=devnet |
| MUSD test mint | https://explorer.solana.com/address/9ixsxMHJHJJUSNCzQrpP511143s9xksBmYxFAomwjr56?cluster=devnet |
| A real round with proof | `/fairness/12`, and the transactions in `docs/deploys.md` |
| Live app URL | **TODO** (hosting is issue #151, not done) |
| Pitch video | **TODO** (script: issues #65 to #67) |
| Event registration | **TODO** (Bibhu) |

## What to check in five minutes

1. **Open the program** on Explorer (link above). It is deployed and upgradeable on devnet.
2. **Open round 12.** `docs/deploys.md` lists its transactions: place bet, request roll, VRF callback, settle, deposit, pay queue, finish. The callback transaction shows the six faces `[2, 2, 4, 6, 4, 5]` written onchain.
3. **Run the tests:** `source scripts/env.sh && anchor build && cargo test -p jhandi_munda`. Expect 96 passed, 0 failed, 1 ignored. They run the built program in LiteSVM, with no network and no keys.
4. **Run the app:** `cd app && pnpm install --frozen-lockfile && pnpm dev`, then open `/play?state=result`, `/pool`, `/fairness/12`, `/replay`. Every example number is tagged EXAMPLE or SIMULATION.
5. **Read the decisions:** `docs/decisions.md` has D1 to D51, each with its reason.

## Demo script (about 3 minutes)

| Time | Show | Say |
|---|---|---|
| 0:00 | Landing page | "The street dice game, onchain. Six dice, one bowl, a 90-second table. Devnet and test money only." |
| 0:20 | `/play?state=open` | "Pick a symbol, bet $1 or more. A session key lets you bet without a popup." (This page is a UI shell on example data.) |
| 0:50 | `/play?state=rolling`, then `?state=result` | "The dealer covers the bowl while MagicBlock VRF rolls six dice. Two or more matching dice win: 3x to 101x back." |
| 1:20 | The two claim cards | "A win pays by Wait or Take now. A loss turns into MUNDA after 24 hours. It is not a refund, and a loss has no Take now." |
| 1:50 | `/fairness/12` | "This one is real. Open the VRF callback on Explorer: these are the faces the program stored." |
| 2:20 | `/pool` | "Anyone can be the house. NAV is (vault minus what the queue owes) divided by shares. It can go down." |
| 2:40 | `/replay` | "A simulation of the program's own code: one round hits all six dice, the pool cannot pay at once, so the wins wait in the queue and the table keeps running." |

## Verified, and not verified

| Claim | Evidence |
|---|---|
| Program logic: rounds, settle, claims, queue, pool, staking, fees, overflow, session keys | 96 LiteSVM tests (`cargo test -p jhandi_munda`) |
| The program runs on devnet | Deploys, upgrades and proof runs in `docs/deploys.md` (rounds 12 to 16) |
| Loser claims are Wait only; chip losers get MUNDA | Unit tests, and devnet rounds 13 and 14 |
| Session key re-grant works | Full script run twice in a row on devnet, no `InvalidSession` |
| Dice come from VRF and cannot be replayed | `callback_roll.rs` rejects unexpected callbacks; round 12 callback on Explorer |
| Web app pages | Built on **mock data**; axe-core accessibility check: 0 violations at 390px and 1440px |
| The app talks to the program | **Not yet** (issue #152). Only `/get-musd` and wallet connect touch devnet. |
| Ephemeral Rollup for session token and cap | **Unverified** (`docs/deploys.md`, `docs/er-design.md`) |
| Redeeming a loss claim by Wait on devnet | **Not run** (needs 24 hours). Covered by unit tests only. |
| Private Ephemeral Rollup (hidden bets) | **Not built** (stretch, Step 17) |
| Security audit | **None.** Hackathon code. |

## Submission checklist

- [x] Program deployed to devnet, upgradeable, proof transactions logged (`docs/deploys.md`)
- [x] Tests pass: `cargo test -p jhandi_munda` (96)
- [x] README explains the game, the money flow, the fairness model, what is live and what is not
- [x] Decisions recorded (D1 to D51) and the build plan kept (`docs/build-plan.md`)
- [x] No keys, `.env` files or wallet files in the repo (`keys/` and `*-keypair.json` are gitignored)
- [x] Devnet and test money only; risk and "not a refund" wording on the app pages
- [x] Accessible at 390px and 1440px (axe-core, 0 violations)
- [ ] **TODO** Host the app and put the URL above (#151, Bibhu)
- [ ] **TODO** Wire the pages to the program (`program-client.ts`, #152): sending a transaction must be blocked unless the devnet guard says devnet
- [ ] **TODO** Record the pitch video (#65 to #67) and add the link above
- [ ] **TODO** Register the submission (Bibhu)
- [ ] **TODO** Redeem a loss claim by Wait on devnet once 24 hours have passed, and add the signatures to `docs/deploys.md`
- [ ] **TODO** Add a licence file if the repo should be open source

## Known limits and risks

- Not audited; for a hackathon, not for real money. Devnet only.
- MUNDA cannot be unstaked until an admin switch is turned on, so it has no market.
- On devnet, scripts can farm the 5x early MUNDA bonus with many wallets through the faucet. MUNDA has no value there, so we accepted it (D49).
- Max bet and exposure caps exist but are off by default on devnet (D6).
- The payout table leans to the house by about 3.8% before fees (published in `docs/build-plan.md`). The 2% fee applies to winning profit and, from the pool's side, to losing bets.
- A pool that cannot pay a big win queues it. A deep queue can wait a long time, and LP withdraws are closed while wins are queued (D30).

## Stack and credits

Anchor 1.0.2, Rust 1.89, Solana/Agave 3.1.9; MagicBlock `ephemeral-rollups-sdk` 0.18 (rollup and VRF) and `session-keys` 3.1; Next.js 16, React 19, TypeScript. Versions and the gotchas found while pinning them are in the README.
