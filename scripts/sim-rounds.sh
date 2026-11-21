#!/usr/bin/env bash
# Plays N rounds of the real program (LiteSVM, #61). Replayable: the same seed gives the same run.
#   ./scripts/sim-rounds.sh --rounds 5 --seed 1
# Needs no keys and no network. It builds the program first (the tests load target/deploy/*.so).
set -euo pipefail
ROUNDS=5
SEED=1
while [[ $# -gt 0 ]]; do
  case "$1" in
    --rounds) ROUNDS="$2"; shift 2 ;;
    --seed) SEED="$2"; shift 2 ;;
    *) echo "usage: $0 [--rounds N] [--seed S]" >&2; exit 2 ;;
  esac
done
cd "$(dirname "$0")/.."
source scripts/env.sh
anchor build > /dev/null 2>&1
SIM_ROUNDS="$ROUNDS" SIM_SEED="$SEED" \
  cargo test -p jhandi_munda --test test_first_bet sim_rounds_from_env -- --ignored --nocapture 2>&1 \
  | grep -E "^(simulation|round )|test result|panicked|error" 
