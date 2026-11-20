# MagicBlock docs snapshot

Fetched on **20 Nov 2026, 11:20 AM IST (Asia/Calcutta)**. Devnet / local planning only. Do not treat this folder as live docs; always re-check if a page may have changed.

## Source URLs

| File in this folder | Source | Size on fetch |
| --- | --- | --- |
| `llms.txt` | https://docs.magicblock.gg/llms.txt | 22,221 bytes, 215 lines |
| `llms-full.txt` | https://docs.magicblock.gg/llms-full.txt | 493,083 bytes, 13,417 lines |
| `products-index.txt` | https://docs.magicblock.gg/_llms/en/products.txt | 27,227 bytes, 246 lines |

The official `llms-full.txt` header says the file may be truncated. The copy above ends cleanly after the VRF technical-details page, and it includes the VRF, Ephemeral Rollup, Private Ephemeral Rollup, and Session Keys pages listed in the products index.

`extracted/` holds sliced copies of those key pages, taken from `llms-full.txt` on the same day, so we can read one topic without opening the whole dump.

Human-readable notes for this project are in [`notes.md`](notes.md).

## Anchor version (fetched the same day)

Checked on crates.io (`https://crates.io/crates/anchor-lang`) and GitHub (`https://github.com/otter-sec/anchor/releases`):

- **Latest stable `anchor-lang`:** `1.2.2`, published 28 Oct 2026. Docs: https://v2.anchor-lang.com/docs/updates/release-notes/
- **Latest prerelease:** `2.0.0-rc.4`, published 14 Nov 2026. Do not use this for the hackathon.
- **Older still-published lines:** `0.32.3` and `0.31.3` (3 Nov 2026).

MagicBlock's own current guides (ER, Private ER, and VRF quickstarts in this snapshot) list this tested combo:

- Solana `3.1.9`
- Rust `1.89.0`
- **Anchor `1.0.2`** (published 2 May 2026)
- Node `24.10.0`
- `ephemeral-rollups-sdk` **`0.18.x`** (Rust crate; npm package `@magicblock-labs/ephemeral-rollups-sdk`, dist-tag `legacy`)

An unpinned `cargo add ephemeral-rollups-sdk` or `npm i @magicblock-labs/ephemeral-rollups-sdk` installs **`0.20.x`**, which targets validator 2.0 (`devnet-us` today), needs `anchor-lang` 1.2.2 or later, and has a broken `#[vrf]` macro on `0.20.0`. Public Ephemeral Rollups (including the TEE) run validator **1.1.1**. Pin `0.18.x`.

**Recommendation for this repo:** follow the MagicBlock example combo (Anchor `1.0.2` + SDK `0.18.x`). Whether SDK `0.18.x` compiles cleanly against Anchor `1.2.2` is **UNVERIFIED**.
