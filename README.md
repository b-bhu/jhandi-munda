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
- **Test dollar:** MUSD (Munda USD), 6 decimals like USDC (so 1 MUSD = 1_000_000 units). Our own devnet stablecoin at $1, created and controlled by the program as PDAs (D21, D26). No real USDC. The `musd_faucet` instruction mints 100 MUSD per call to the caller's associated token account (created if needed), no cap.

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
keys/devnet-wallet.json` calls the same instruction (see the script header).

Devnet and local only. Keypairs go in `keys/` or `~/.config/solana` and are never committed.
