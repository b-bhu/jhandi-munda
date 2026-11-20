# Source this before building: `source scripts/env.sh`
# Puts the pinned toolchain first on PATH for this shell only, so the other
# Solana/Anchor versions installed globally on the shared box are untouched.
export PATH="$HOME/.local/share/anchor-pinned/bin:$HOME/.local/share/solana-pinned/3.1.9/solana-release/bin:$HOME/.cargo/bin:$PATH"
# RUSTUP_TOOLCHAIN is not set here: rust-toolchain.toml already pins 1.89.0, and
# exporting it breaks the anchor 1.1.x CLI IDL step ("toolchain '{toolchain}' is not installed").
