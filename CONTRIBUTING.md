# Contributing

This repo is built one small GitHub issue at a time. Bibhu and his coach review each issue; an autonomous coding agent (Hermes, running on a VPS) implements the code, strictly one issue after another.

Read first: `docs/build-plan.md`, `docs/decisions.md`, `docs/reference/magicblock/notes.md`, and the pinned versions in `README.md`.

## How work is sliced

Each build-plan step is a parent **epic** (label `epic`) plus 2–5 **sub-issues** titled `Step N.k: ...`. An epic is a tracking issue with a task list; do not open a PR against an epic. Each sub-issue is one job, one new concept, and should touch few files — roughly under 150 lines of change (one instruction, one account type, or one UI piece). It must be finishable and testable on its own.

## Workflow

1. Hermes takes the **lowest-numbered open sub-issue labelled `next-up`**. There must be only one `next-up` issue at a time. Do not pick an epic, and do not pick a `stretch` issue until every must-have issue in the current milestone is done (or the coach says to start one).
2. Open a branch from `main` named `step-N-k-short-name` (for example `step-2-1-musd-mint`). For non-step issues, use a short kebab-case name (`pitch-outline`).
3. Stay inside that one issue. Do not start a sibling sub-issue or the next step in the same PR. If the change is growing past ~150 lines, stop and ask to split the issue.
4. Open a PR against `main` whose body includes `Closes #N` (the sub-issue you picked).
5. In the PR, paste the proof from the issue's **Acceptance checks**: commands you ran, the output that shows they passed, and any devnet Explorer links.
6. **Do not merge until Bibhu and his coach have verified the Done-when.** After they sign off and the PR is merged, the **reviewer moves the `next-up` label** from the closed issue to the next sub-issue in order (the next unchecked box on the epic, or the first sub-issue of the next epic). Never leave `next-up` on a closed issue, an epic, or a stretch issue while must-haves remain.

Phase 1 (plan) is already done: 32 decisions are locked in `docs/decisions.md`. Current work is Phase 2 (build on devnet), then Phase 3 (demo and submit).

## Constraints (every PR)

- Devnet and local only. No mainnet, no real funds.
- Never commit keypairs, `.env` secrets, or wallet files. Never print private keys in logs, tests, or PR text. Keys live in `keys/` (gitignored) or `~/.config/solana`.
- Keep the pinned toolchain: Anchor **1.0.2**, `ephemeral-rollups-sdk` **0.18.x** (do not move to 0.20), Rust **1.89**, Solana/Agave **3.1.9**. Leave `Cargo.lock` committed. If you regenerate it, pin every `anchor-*` macro crate back to 1.0.2 with `cargo update -p <crate> --precise 1.0.2`.
- Before building or testing, run `source scripts/env.sh` so the pinned tools are first on `PATH`. On a fresh machine, install the pinned versions listed in `README.md` first.

## Checking your work

```bash
source scripts/env.sh
anchor build
anchor test          # runs `cargo test` (LiteSVM); needs the .so from `anchor build`
```

For a change under `app/` (the same steps CI runs, in `.github/workflows/app-ci.yml`):

```bash
cd app
pnpm install --frozen-lockfile
pnpm lint && pnpm typecheck && pnpm test && pnpm build
node --test ../scripts/gen-sim.test.mjs   # the simulation fixture parser
```

`pnpm typecheck` runs `next typegen` first, because `PageProps` and `LayoutProps` are generated types.

A sub-issue is only done when its **Done when** and **Acceptance checks** all pass, not merely when the code compiles.
