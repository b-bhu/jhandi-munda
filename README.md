# Jhandi Munda Casino

The Indian street dice game Jhandi Munda (also called Crown and Anchor), rebuilt as an onchain casino.

Built for MagicBlock Solana Blitz v10 (20–27 Nov 2026).

## The game

- A board of six symbols: heart, spade, diamond, club, crown (munda) and flag (jhandi).
- Six dice, each carrying those six symbols on its faces.
- Players bet on symbols. The dealer shakes the dice in a covered bowl and reveals them.
- A bet wins when its symbol shows on enough dice, and pays more the more dice show it. The exact payout table is still to be decided.

## The feel

It should be the mela street game: a flashy dealer, a worn cloth board, the bowl reveal, and a crowd standing around. See `docs/characters/`.

## Decided so far

- **Counterparty:** a shared house pool. Anyone can deposit into the pool and become part of the house. Players bet against the pool.
- **Test dollar:** MUSD (Munda USD), 6 decimals like USDC (so 1 MUSD = 1_000_000 units). Our own devnet stablecoin at $1, created and controlled by the program as PDAs (D21, D26). No real USDC. The `musd_faucet` instruction mints 100 MUSD per call to the caller's associated token account (created if needed), at most once per wallet per hour (D33 cooldown, wallet record at `["faucet", wallet]`).
- **House pool (Step 3.1, deposit comes later):** pool PDA at `["pool"]` with its MUSD vault at `["pool_vault"]` and the pool-share (LP) mint at `["pool_share_mint"]`. Initialized empty (0 MUSD vaulted, 0 shares, D4); NAV is $1.00 by definition until the first deposit, so the first shares mint 1:1 with MUSD (`nav_per_share`, D27).
- **Pool deposit (Step 3.2):** the `deposit` instruction moves MUSD from the depositor's canonical MUSD ATA into the vault (zero-amount deposits rejected) and mints pool shares at the current NAV: `amount × supply / vault`, which is exactly 1:1 at the empty pool's defined $1.00 NAV (100 MUSD in = 100 shares). Shares land in the depositor's share ATA, created on their first deposit (D27).
- **Pool withdraw (Step 3.3):** the `withdraw` instruction burns pool shares from the holder's canonical share ATA and pays MUSD out of the vault at the current NAV: `shares × vault / supply`, floored in the pool's favour — exactly 1:1 at the empty-pool $1.00 NAV (40 of 100 shares back = 40 MUSD; the full withdraw of the last 60 returns 60 MUSD and empties the holder's shares). Zero and over-balance withdraws are rejected (D27).
- **NAV after a mocked profit (Step 3.4):** both pool prices read the raw vault / share-supply token accounts, so MUSD transferred straight into the vault (mocked house profit, an ordinary transfer — no extra instruction and no new shares) moves the NAV up: $1.00 → $1.25 when 25 MUSD lands on a 100 MUSD / 100-share pool. The next 100 MUSD deposit then mints 80 shares pro rata (`amount × supply / vault`), and each withdraw pays `shares × vault / supply` exactly (125 and 100 MUSD); both formulas floor in the pool's favour, so any rounding residue stays with the remaining shareholders (D27).
- **Losing and MUNDA (D45 to D49, Step 18):** a lost bet opens a loss claim. After 24 hours "Wait" turns it into MUNDA (1 per 1 MUSD lost, 5 per 1 MUSD on the first $1,000 of all losses ever). "Take now" gives the MUNDA at once but 10% less. Either way the MUNDA goes straight into the player's stake and cannot be unstaked until an admin switch is turned on. The 2% fee is taken on losing bets too (from the pool side), and stakers who have held 24 hours get 70% of it, plus half of any pool gains above $10,000. Details in `docs/decisions.md`.

## Open for brainstorming

- Payout table and house cut
- Pool risk limits (max bet, exposure per round)
- How rounds run (timing, multiplayer)
- Tech choices (not decided yet)

All ideas go in `docs/ideas.md`.

## Plan (three phases)

1. **Plan (steps 2–5), no building:** a Q&A grilling to lock game design, tech choices, pool program logic and game round logic. Answers go to `docs/decisions.md`.
2. **Build and test (steps 6–7):** the UI and end-to-end devnet testing.
3. **Demo and submit:** website, repo, pitch video, Explorer links. Bibhu handles Luma registration.

## Folder layout

```
Anchor.toml, Cargo.toml, Cargo.lock   Anchor workspace (repo root)
rust-toolchain.toml                   Rust 1.89.0 for host/IDL builds
programs/jhandi_munda/                the onchain program (Anchor)
  tests/test_initialize.rs            LiteSVM test that runs the built .so
migrations/                           anchor deploy hook (unused for now)
app/                                  Next.js + TypeScript frontend (App Router)
scripts/env.sh                        puts the pinned toolchain on PATH
docs/                                 plan, decisions, MagicBlock reference
keys/                                 local keypairs, gitignored, never committed
```

The Anchor workspace sits at the repo root (not in an `onchain/` folder) so `anchor build`, `anchor test` and `target/idl` / `target/types` work with no extra paths, and `app/` can import the generated IDL from `../target`.

## Pinned versions

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

## Build and run

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


## Devnet

The program is live on Solana **devnet only** (never mainnet, no real funds).

- **Program id:** `4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH` — [Explorer](https://explorer.solana.com/address/4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH?cluster=devnet)
- **Deploy transaction:** [Explorer](https://explorer.solana.com/tx/3B8sEnN1QE5s2FbmYGN2DnLKF2g1etuyJDWsUzW89BRfwL7rpRpjFq1TJrnZRjcvAwqStP22pCUqRWfYBX9bVK37?cluster=devnet)
- **Program upgrade (table, bets, VRF):** [Explorer](https://explorer.solana.com/tx/2Kh1QbuG7JLa5GCtZRzVgtBmsPLwL99Xu1CHYyRwb3W15gLavWS71Szqjb75c6F6hvHcUviPgPJtRLd8mDVxUMh1?cluster=devnet)
- **Initialize (config + MUSD mint):** [Explorer](https://explorer.solana.com/tx/3gZp61mfMjELBaPBgYrKFzpkarLC2LYuSCgfmGu4ZQBo7QyRScCoa8CC9QiDnEmvtZEXndS3b9hzebc75Zkyjap?cluster=devnet)
- **Initialize pool (empty vault + share mint):** [Explorer](https://explorer.solana.com/tx/MeDrvQqcq1HuAPcYg8mnS4nw9rLBGpbAXKkbgbrsy2hSByF58qJQN3kE4xtkw2YtbZ3CTqEvijo2ovCqgvFZ2KD?cluster=devnet)
- **Faucet (100 MUSD to the dev wallet):** [Explorer](https://explorer.solana.com/tx/5ezoTc7Jk3d5oxid2eqz7cpYQE1fjNfg8AMoCLY6SA12eM4CfH4shJU71z9jYzVJ3Paj48CBLyNChdLxTJtFhZi5?cluster=devnet)
- **MUSD mint:** `9ixsxMHJHJJUSNCzQrpP511143s9xksBmYxFAomwjr56` — [Explorer](https://explorer.solana.com/address/9ixsxMHJHJJUSNCzQrpP511143s9xksBmYxFAomwjr56?cluster=devnet)
- **Dev wallet:** `5qPRTWcUZw1XYGnXqUCCF65TGBgMjk9BfMyz2VBHacm7`
- **Dev wallet MUSD token account (99 MUSD after the $1 Step 5.3 bet):** `5mwQw4GBc9KA77vH3hFkSSgjJWSF2zCUDmoXaLyFXUsu` — [Explorer](https://explorer.solana.com/address/5mwQw4GBc9KA77vH3hFkSSgjJWSF2zCUDmoXaLyFXUsu?cluster=devnet)
- **Round account:** `87yKp1eX7exH7gABS2zmTNizHrTUotuX5fAdTU7oK2Bb` — [Explorer](https://explorer.solana.com/address/87yKp1eX7exH7gABS2zmTNizHrTUotuX5fAdTU7oK2Bb?cluster=devnet)
- **VRF request:** [Explorer](https://explorer.solana.com/tx/5ucritoaKGe1XN2UdeV7dXc6vdUfp3GK7mVqjGMYXR9iBGcG4Fwh3k91FoiRx3APoo5PWhkrHVgxdUDU8iRrP8ya?cluster=devnet)
- **VRF callback (faces 4, 5, 3, 5, 2, 4):** [Explorer](https://explorer.solana.com/tx/463n6bMyvDsjmbaYPqvoHqaGQ4QcYN2tvmcKyHazoFVGx2evp7gshTS1ti9oqwNZMXFf6oJomE62E4kAa9ditewH?cluster=devnet)

Without a browser, the same initialize and faucet calls are:

```bash
node scripts/devnet-musd.mjs init keys/dev-wallet.json
node scripts/devnet-musd.mjs faucet keys/dev-wallet.json
node scripts/devnet-vrf-roll.mjs keys/dev-wallet.json
```

`devnet-vrf-roll.mjs` opens a round with a $1 bet, waits for the 90-second window on the on-chain Clock, requests the MagicBlock VRF roll, and prints Explorer links for the request and the callback. The current devnet round is already Rolled (faces 4, 5, 3, 5, 2, 4), so running it again reprints that proof. A new roll needs the Step 6 settle instruction first.


Devnet and local only. Keypairs go in `keys/` or `~/.config/solana` and are never committed.

## Ephemeral Rollup design

Which tokens move on the rollup and which stay on Solana, the account size budget, and the open points: see [docs/er-design.md](docs/er-design.md).
