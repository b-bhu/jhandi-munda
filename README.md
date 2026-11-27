# Jhandi Munda Casino

**The Indian street dice game Jhandi Munda (Crown and Anchor), rebuilt as an onchain casino on Solana devnet.**
Six symbol dice, one covered bowl, a 90-second table. The roll comes from MagicBlock's random number service, so anyone can check it was not rigged.

Built for **MagicBlock Solana Blitz v10** (20 to 27 Nov 2026). **Devnet only. MUSD is a test token with no value. No real money, and nothing here is a promise of returns.**

| | |
|---|---|
| Program (devnet) | [`4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH`](https://explorer.solana.com/address/4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH?cluster=devnet) |
| A real round, start to finish | [round 12 on the fairness page](#proof-on-devnet) (faces 2, 2, 4, 6, 4, 5) |
| Submission notes | [`docs/submission.md`](docs/submission.md) |
| All decisions (D1 to D51) | [`docs/decisions.md`](docs/decisions.md) |

## What is live and what is not

Read this first. We would rather be exact than impressive.

| Part | Status |
|---|---|
| Solana program (46 instructions) | **Live on devnet.** `cargo test -p jhandi_munda`: 96 pass, 1 ignored (the simulation harness). |
| Rounds, VRF dice, settle, claims, payout queue, pool, MUNDA staking, fees, overflow, session keys | **Built and run on devnet** by scripts. Proof transactions are in [`docs/deploys.md`](docs/deploys.md). |
| Ephemeral Rollup | A round was delegated to the rollup on devnet during development (a `request_roll` there worked once, in a one-off test). The latest full proof runs (rounds 12 to 16) ran on the Solana base layer. Delegating the session token and session cap is **not verified**. See [`docs/er-design.md`](docs/er-design.md). |
| Web app (`app/`) | **A UI shell on mock data.** Every page shows a "UI SHELL · MOCK DATA" strip and EXAMPLE tags. Only `/get-musd` (the faucet) and the wallet connect talk to devnet. Wiring the pages to the program is the next piece of work (`GameClient`, issue #152). |
| `/fairness/12` | Real devnet data: the six faces and the transaction links. |
| `/replay` | A **simulation**: the program's own code run offline with made-up players. Not real rounds. |
| Private Ephemeral Rollup (hidden bets) | Not built. Stretch goal. |
| Mainnet | Never. Devnet and local only. |

## The game

- A board of six symbols: **heart, spade, diamond, club, crown (munda), flag (jhandi)**, on a 2 x 3 cloth.
- Six dice, each carrying those six symbols.
- Pick a symbol and bet at least **$1** of test MUSD. The first bet opens a **90-second** table; others can join.
- When the window ends the dealer covers the bowl and MagicBlock VRF rolls all six dice at once. Then the bowl opens.
- Count the dice that show your symbol. **Two or more wins. Zero or one loses your stake.**

| Dice matching | Profit | You get back (stake included) |
|---|---|---|
| 0 or 1 | none | 0 |
| 2 | 2x | 3x |
| 3 | 4x | 5x |
| 4 | 8x | 9x |
| 5 | 25x | 26x |
| 6 | 100x | 101x |

A **2% fee** is taken from a winner's profit (the stake comes back untouched). The dice table leans to the house by about **3.8%** before fees. We publish the table and the math ([`docs/build-plan.md`](docs/build-plan.md)) instead of hiding it.

## Where the money goes

```text
 player ──bet──▶  house pool (shared vault)  ◀──deposit── LPs (get pool shares at NAV)
                     │
        win  ◀───────┤  paid in MUSD: Wait (100% after 90 s) or Take now (95%, from the insurance fund)
        queue ◀──────┤  if the pool is short, wins wait first-in, first-out (D30, D44)
        loss ────────┘  becomes MUNDA after 24 h, straight into the player's stake. Not a refund.
```

- **Wins** become a claim. **Wait** pays the full amount after 90 seconds. **Take now** pays 95% at once from the insurance fund, which the pool repays later.
- **Losses** become **MUNDA**, a house token. It goes straight into your stake after 24 hours. **Losses have Wait only: there is no Take now for a loss (D50).** The first $1,000 of losses across all players earn 5 MUNDA per $1, then 1 per $1 (D49). Chip bets earn MUNDA too (D51).
- **The pool.** Anyone can deposit test MUSD and get shares at NAV = (vault − owed to the queue) ÷ shares. Pool value can go down as well as up. LPs can lose money.
- **Fees.** The 2% fee also applies to losing bets, taken from the pool's side (D45). It splits 70% to MUNDA stakers, 20% to the insurance fund and 10% to the developers.
- **Overflow.** Above $10,000 of pool assets, half of the gains go to eligible stakers and half stay in the pool (D48). Only house gains count, not LP deposits.
- **Staking.** A stake lot earns after 24 hours. Unstake is off until an admin "transfers" switch is turned on (D46, D47).
- **Session keys.** Approve once, then bet with no wallet popup, up to a cap and for up to 60 minutes. Revoke any time.

## How the dice stay fair

1. After the 90-second window, anyone can call `request_roll`. The program asks MagicBlock's VRF for 32 random bytes and stores the request id.
2. The VRF answers with a callback. The program rejects any callback it did not ask for, then keeps only bytes below 252 (so no face is likelier than another) and turns each into a face with `byte % 6 + 1`.
3. The six faces are written onchain. Settlement reads them from there.

Every settled round has a public page, `/fairness/<round>`, with the faces and a link to each transaction on Solana Explorer (devnet).

## How it is built

```text
 Next.js app (app/)  ──▶  lib/game-client.ts  ──▶  mock data today · program-client.ts later
        │                                                    │
        └── wallet pill + devnet guard                       ▼
                                          Anchor program (programs/jhandi_munda, Rust)
                                          ├─ MUSD test mint and faucet
                                          ├─ house pool, shares, NAV, payout queue
                                          ├─ table, rounds, bets, chips (the "cage")
                                          ├─ VRF request and callback  ── MagicBlock VRF
                                          ├─ settle, claims, insurance, fee pots
                                          ├─ MUNDA mint, stake, rewards, overflow
                                          └─ session keys, Ephemeral Rollup delegate/undelegate
```

- **Program:** Anchor 1.0.2 on Solana, Rust 1.89. LiteSVM tests load the built `.so`.
- **MagicBlock:** `ephemeral-rollups-sdk` 0.18 (rollup and VRF) and `session-keys` 3.1.
- **App:** Next.js 16, React 19, TypeScript. Pages only talk to `lib/game-client.ts`; they never import `@solana/web3.js` or the IDL (the standalone faucet page is the one exception). `NEXT_PUBLIC_DATA=mock|program` picks the data source (default mock). The wallet connect refuses to connect unless the RPC reports the devnet genesis hash.

## Proof on devnet

Everything below is a real devnet transaction. The full log, with every signature, is [`docs/deploys.md`](docs/deploys.md).

- **Round 12** (a chip bet, 2 matching dice, a 2.96 MUSD win that was queued and then paid): [bet](https://explorer.solana.com/tx/45KEdeQTrCqz9yV6smK8U6dYwg5Xw1s19VWgTexnDSDPRwovZqdhrRNnD15y6pMdXGHiBZqBBWuHCnrDVqRYeqob?cluster=devnet) · [roll requested](https://explorer.solana.com/tx/4hSyiicby5eaYtM8hBzGH5euGPcpyiSQWdqvHht6yZvQ7ZkrGDd27mgy8z8v5iKoo2ZQQzR9SwNwRnqbBFihfcRa?cluster=devnet) · [VRF callback](https://explorer.solana.com/tx/5aQ75QwPrQafMvV8sbHgZW69afP1fH4N3HaB2tm9jJAZBH83Turikxv1bmL3iq2s4CvmJ8V2Ma8iMap1aXhyAZtF?cluster=devnet) · [settled](https://explorer.solana.com/tx/5XFUaLXUZQcRkQy1BFj4ksjhwYn7eBMm5umXBn9AdZyBfiASyP2ChHZhc6mTaPqDUw9nUhNSP4fde7Hijx4J1hSN?cluster=devnet) · [queue paid](https://explorer.solana.com/tx/5yX32eVCaGaYvXV2nWPxcjM4wAP7egB7d5GAzq8dgxafh9pm64DpkQasJHe1ouuVxrEnMHaNMuMUCsyiqVrUfJLS?cluster=devnet)
- **Loss claims** (rounds 13 and 14): a wallet loss and a chip loss each opened a 5 MUNDA claim, Take now was refused on both, and a winning chip bet correctly had no claim. See the 27 Nov 17:00 IST entry in `docs/deploys.md`.
- **Session keys** (rounds 15 and 16): the full script ran twice in a row with a fresh session key each time and no `InvalidSession` error.
- **Upgrades:** the program upgrade and the config migration (74 to 91 bytes) are in `docs/deploys.md`.

Not tested on devnet: redeeming a loss claim by Wait (it needs 24 hours). That path is covered by unit tests.

## Try it

**Run the app** (mock data, no wallet needed):

```bash
cd app
pnpm install --frozen-lockfile
pnpm dev                      # http://localhost:3000
```

Open `/play` (in mock mode add `?state=open`, `rolling`, `win`, `loss` or `result` to see each phase), `/pool`, `/fairness/12`, `/replay`, `/docs`. Get test MUSD with the one-button faucet on `/get-musd` (Phantom set to devnet; 100 MUSD per click, once an hour per wallet).

**Run the program tests** (no network, no keys):

```bash
source scripts/env.sh          # the pinned Rust / Solana / Anchor first on PATH
anchor build                   # builds target/deploy/jhandi_munda.so + the IDL
cargo test -p jhandi_munda     # LiteSVM runs the built program
```

**Replay the simulation** (the program's own code, offline): `./scripts/sim-rounds.sh --rounds 50 --seed 1`, then `node scripts/gen-sim.mjs --from-file <output>` writes `app/public/sim.json` for `/replay`.

**Devnet scripts** (need your own keypair in `keys/`, which is gitignored): `scripts/devnet-checks-112.mjs`, `scripts/devnet-er-round.mjs`, `scripts/devnet-loss-claims-126.mjs`. Each has its usage in its header.

## Repo map

```text
programs/jhandi_munda/     the Anchor program (src/instructions/*) and its LiteSVM tests (tests/*)
app/                       Next.js app: app/app (pages), components, lib (data layer, payouts, wallet)
scripts/                   devnet scripts, the simulation harness, the sim fixture generator
docs/decisions.md          D1 to D51, each with its reason
docs/build-plan.md         the plan, the payout math and a "done when" for every step
docs/er-design.md          which accounts run on the rollup, and what is unverified
docs/deploys.md            every devnet deploy and proof run, with Explorer links
docs/design/               the design mocks (png) and their HTML
docs/characters/           the dealer, the board, the dice and bowl art
docs/submission.md         the submission checklist and a demo script
```

## Honest limits

- **Hackathon demo, not audited, not for real money.** Devnet only.
- The web app is not wired to the program yet; the table, pool and claim pages run on example data and say so.
- MUNDA cannot leave the stake until an admin switch is turned on. There is no MUNDA market.
- On devnet the faucet lets scripts farm the 5x early MUNDA bonus with many wallets. MUNDA has no value there, so we accepted that (D49).
- Risk caps (max bet, exposure per round) are built in but off by default on devnet (D6).
- Ephemeral Rollup delegation of session keys, and the rollup's behaviour for every call, are not verified end to end.

Gambling games carry risk and may be restricted where you live. Nothing here is financial advice.

## Toolchain and versions


These are the versions actually installed and used for the scaffold (20 Nov 2026).

| Tool / crate | Version | Notes |
| --- | --- | --- |
| Rust (host) | 1.89.0 | `rust-toolchain.toml`, via rustup |
| Solana / Agave CLI | 3.1.9 | `cargo-build-sbf` uses platform-tools v1.52 |
| Anchor CLI | 1.0.2 | official release binary (otter-sec/anchor) |
| `anchor-lang` | =1.0.2 | macro crates (`anchor-syn`, `anchor-derive-*`, `anchor-attribute-*`) locked to 1.0.2 in `Cargo.lock` |
| `ephemeral-rollups-sdk` | =0.18.0 | features `anchor`, `vrf` (pulls `ephemeral-vrf-sdk` 0.18.0). Do not move to 0.20. |
| `session-keys` | =3.1.1 | feature `no-entrypoint`; compiles with Anchor 1.0.2 |
| `litesvm` (dev) | 0.10.0 | default Anchor 1.0 test template |
| Node.js | 24.22.1 | |
| pnpm | 10.34.6 | |
| Next.js | 16.4.6 | React 19.3.0, TypeScript 5.9.3 |

Gotchas found while pinning:

- `anchor-lang 1.0.2` asks for its macro crates as `^1.0.2`, so a fresh lockfile pulls the 1.2.2 macros. Those generate code 1.0.2 does not have (`create_account_allow_prefund`), and `session-keys` then fails to compile. Keep `Cargo.lock` committed. If you regenerate it, run `cargo update -p <crate> --precise 1.0.2` for each `anchor-*` macro crate.
- Anchor 1.0.2 asks rustup for `stable` during the IDL build unless `RUSTUP_TOOLCHAIN` is set. `scripts/env.sh` sets it to `1.89.0`.
- The shared box also has Solana 4.3.0 and Anchor 0.31.1 on the global PATH, used by other projects. The pinned tools live in `~/.local/share/solana-pinned/3.1.9` and `~/.local/share/anchor-pinned`, and `scripts/env.sh` puts them first. Nothing global was switched.

### Build and run

```bash
source scripts/env.sh          # pinned Rust / Solana / Anchor on PATH
anchor build                   # builds target/deploy/jhandi_munda.so + IDL
cargo test                     # LiteSVM test (same as `anchor test --validator legacy`)

cd app
pnpm install
pnpm dev                       # http://localhost:3000
pnpm build                     # production build check
```

Get MUSD on devnet: the app's `/get-musd` page has the one-button faucet
(100 MUSD per click) plus the mint address and an Explorer link
(`?cluster=devnet`). Without a browser, `node scripts/devnet-musd.mjs faucet
keys/dev-wallet.json` calls the same instruction (see the script header).

## Credits

Made for MagicBlock Solana Blitz v10. Uses MagicBlock's Ephemeral Rollups SDK, VRF and session keys. The art in `docs/characters/` and `docs/reference/` was made for this project. There is no licence file yet: all rights reserved until one is added.
