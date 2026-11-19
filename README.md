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

## Open for brainstorming

- Payout table and house cut
- Pool risk limits (max bet, exposure per round)
- How rounds run (timing, multiplayer)
- Tech choices (not decided yet)

All ideas go in `docs/ideas.md`.
