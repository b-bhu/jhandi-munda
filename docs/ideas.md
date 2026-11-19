# Ideas

A running dump of ideas. Add freely; nothing here is decided unless it's in the README's "Decided so far" section.

- House pool as counterparty: depositors own the house and earn its cut. (Decided)
- Publish the house cut openly, since every roll is provably fair. Street games hide theirs.
- The bowl reveal is the big moment in the UI.
- Demo idea: show the pool earning over 50 rounds.
- Reference: SatRush (https://satrush.io/docs/). It's a player-vs-player fee game dressed as "mining": 21 blocks, one wins every minute, losing stakes come back minus a fee, and winners share the fees. A rolling "Sat Strike" jackpot built from fee slices hits about 1 in 1,097 rounds. There are points ("hashrate") for every play, used as draw tickets, plus a token.
  - Possible borrow: a rolling "Munda Strike" jackpot funded by a slice of each round's fees, on top of the six-crowns payout.
  - Possible borrow: points for every bet, spent on weekly draws.
  - Possible borrow: shared one-minute rounds and a public check-any-round fairness page.
- Reference: Papertrade (https://docs.papertrade.xyz/). A 1000x perps venue on Hyperliquid where every trade faces one LP. The LP is not seeded: it starts at $0 and grows only from user losses (the "Martingaler" mechanism). If the LP can't pay a win, the win goes into a first-in-first-out payout queue, paid as future losses come in. Losers are minted PAPER, a token that is the only way to own the LP's economics (it earns 1% of PnL plus overflow above $5M). The house takes its cut from winners only, through a haircut plus a 2% win fee.
  - Possible borrow: Martingaler-style pool, so there's no seed capital and no hard bet cap, and the queue absorbs variance.
  - Possible borrow: "lose and you own the house" tokens, minted to losing bettors.
  - Possible borrow: take the fee from wins only.
