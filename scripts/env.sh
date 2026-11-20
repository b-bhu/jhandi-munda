# Source this before building: `source scripts/env.sh`
# Puts the pinned toolchain first on PATH for this shell only, so the other
# Solana/Anchor versions installed globally on the shared box are untouched.
export PATH="$HOME/.local/share/anchor-pinned/bin:$HOME/.local/share/solana-pinned/3.1.9/solana-release/bin:$HOME/.cargo/bin:$PATH"
# Anchor 1.0.2 otherwise asks rustup for "stable" during the IDL build; pin it to ours.
export RUSTUP_TOOLCHAIN=1.89.0
