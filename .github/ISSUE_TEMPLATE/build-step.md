---
name: Build sub-issue
about: One small job (one instruction, account type, or UI piece) for Hermes to pick up.
title: "Step N.k: "
labels:
  - good-for-agent
---

Read first: `docs/build-plan.md`, `docs/decisions.md`, `docs/reference/magicblock/notes.md`, and the pinned versions in `README.md`.

## What

<!-- One job, in plain English. One new concept. Aim for under ~150 lines of change. -->

## Why

<!-- Why this sub-issue exists. -->

## Done when

<!-- Observable proof that this one job is finished and testable on its own. -->

## Decisions

<!-- D-numbers from docs/decisions.md that this sub-issue implements, e.g. D21, D26. -->

## Acceptance checks

<!-- Exact commands to run and what output proves the Done-when. Include any Solana Explorer link that is expected (devnet). -->

```bash
source scripts/env.sh
anchor build
anchor test
```

## Constraints

- Devnet and local only. No mainnet, no real funds.
- Never commit keys or print private keys. Keys live in `keys/` (gitignored) or `~/.config/solana`.
- Pinned versions: Anchor 1.0.2, `ephemeral-rollups-sdk` 0.18.x (do not move to 0.20), Rust 1.89, Solana/Agave 3.1.9. Keep the `Cargo.lock` macro-crate pin at 1.0.2.
- Keep the PR scoped to this one issue. Do not start a sibling sub-issue here. Aim for under ~150 lines of change.
- Open a PR against `main` whose body includes `Closes #N` (this issue's number).

## Out of scope

<!-- What must wait for a later sub-issue. -->

## Depends on

<!-- e.g. #N must be merged first. Depend only on earlier issues. -->
