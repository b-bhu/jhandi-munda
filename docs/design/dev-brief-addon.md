ADD-ON TO THE DEV BRIEF: APP PAGES AS UI-ONLY SHELLS

Goal: build /play, /pool, /fairness/[round], /replay now, with MOCK data, in the same look as the landing page. Do NOT call the program yet. Later we swap the mock for the real program in ONE file.
Pictures to copy: play.png, play-mobile.png, pool.png, fairness.png, replay.png (+ the HTML in app-mock/ for the CSS: base.css = tokens, app.css = page parts).

RULES FOR SHELLS
- Every mock number must be marked: a small "EXAMPLE" tag on the panel, and a top strip "UI SHELL · MOCK DATA" while the mock client is on. The strip disappears when the real client is on (one flag: NEXT_PUBLIC_DATA=mock|program, default mock).
- Pages never import @solana/web3.js or the IDL. They only call the data layer (below). That is what makes the swap one file.
- Devnet only. Test MUSD. Never promise returns. Loss cards say "not a refund". Loss claims are Wait only (D50). Chip-bet losers get MUNDA too (D51).
- Mobile: all pages work at 390px (single column). /play is checked at 390px in the PR.

THE DATA LAYER (3 files)
app/lib/types.ts        plain types: Symbol (1..6), Round, Bet, Claim, PoolView, Position, StakeView, QueueItem, FairnessProof, SimRound
app/lib/game-client.ts  the interface + getClient(): returns mockClient or programClient by the env flag
app/lib/mock-data.ts    the fixtures and the mockClient implementation (all example numbers live here, nowhere else)
(later) app/lib/program-client.ts   real implementation, reads accounts / sends instructions. This is the ONLY file that changes later.

interface GameClient {
  // table
  getRound(): Promise<Round>               // id, phase 'open'|'rolling'|'result', secsLeft, totals per symbol, faces?
  subscribeRound(cb: (r: Round) => void): () => void
  placeBet(symbol: Symbol, amountMusd: number): Promise<Bet>
  getMyBets(roundId: number): Promise<Bet[]>        // symbol, amount, matches?, result?
  getMyClaims(): Promise<Claim[]>                   // kind 'win'|'loss', amount, unlockAt, canTakeNow (false for loss)
  redeemClaim(id: string, mode: 'wait' | 'takeNow'): Promise<void>
  // session
  getSession(): Promise<{ on: boolean; minsLeft: number; capMusd: number }>
  startSession(): Promise<void>
  revokeSession(): Promise<void>
  // pool
  getPool(): Promise<PoolView>              // vault, owedToQueue, shares; nav = (vault - owed) / shares (computed in ONE helper)
  getMyPosition(): Promise<Position>        // shares, boughtAt, worthNow, profitPct
  deposit(amountMusd: number): Promise<void>
  withdraw(shares: number): Promise<void>
  getQueue(): Promise<QueueItem[]>
  getStake(): Promise<StakeView>            // lots, locked/earning, worthThen, worthNow, transfersEnabled
  getSnapshots(): Promise<{ day: string; nav: number; poolSize: number }[]>
  // proof and replay
  getFairness(roundId: number): Promise<FairnessProof>  // faces, steps [{label, signature}], explorer links
  getReplay(): Promise<SimRound[]>          // 50 rounds: poolCash, owedInQueue, jackpot flag, bets
  faucet(): Promise<void>
}
Money is stored as integer MUSD base units (6 decimals) inside the client and formatted ONLY by lib/format.ts.
Payout multipliers live in lib/payouts.ts (one file, shared with the landing page): matches 2/3/4/5/6 = 3/5/9/26/101x back (2/4/8/25/100x profit). 2% fee on a winner's profit. Loss claim: 24h, MUNDA into the stake. Win claim: Wait 100% after 90 s, Take now 95%.

PAGES, COMPONENTS, FILES (App Router, under app/app/ and app/components/)
/play   app/app/play/page.tsx
  components/table/: RoundTimer (90 s ring or bar), Board (cloth, 6 Tiles, cloth totals, "You" tags), Tile, SymbolIcon (6 SVGs: heart spade diamond club crown flag), BetPanel (symbol picker, amount with $1 min and chips, payout lines, Place bet), SessionBadge, MyBets, DiceRow (6 dice, '?' when covered), Bowl (bowl-covered.jpg and bowl-open.jpg stills), RoundMath, ClaimCard (variant 'win' | 'loss')
  States (a prop on the page driven by round.phase): open -> rolling -> result. Add a "?state=open|rolling|result" query in the mock mode so we can screenshot each.
  Issues: #56 layout, #57 join and bet, #58 bowl and reveal, #59 claim card, #47/#48 session (SessionBadge + start/revoke), #122 loss wording.
/pool   app/app/pool/page.tsx
  components/pool/: NavCard (big NAV + formula box), DepositWithdraw (tabs), MyPosition (bought at, worth now, profit %), QueueTable, StakeCard (lots with "earning" or "N h lock left", worth then/now, locked note), UnstakeButton (disabled when transfersEnabled is false, with the D46/D47 text), SnapshotChart (inline SVG, no chart lib)
  Issues: #10 NAV readout, #36 MUNDA worth then/now, #51 snapshot chart, #11 (epic).
/fairness/[round]  app/app/fairness/[round]/page.tsx
  components/fairness/: FacesRow (reuses DiceRow), RoundSummary, ProofSteps (list: label, what happened, short signature, Explorer link ?cluster=devnet), MatchesTable
  Mock data: use REAL round 12 (faces 2,2,4,6,4,5; signatures from docs/deploys.md: place_bet_chips, request_roll, VRF callback, settle_chip_bets, deposit, pay_queue, finish_round). This page can show real links even in mock mode.
  Issue: #54.
/replay app/app/replay/page.tsx
  components/replay/: StatStrip, PoolQueueChart (inline SVG, jackpot marker), Scrubber (play, pause, drag to a round), RoundCard, AfterJackpot
  Data: a fixture of 50 rounds now (marked SIMULATION), the real sim output later.
  Issue: #63 (needs the 50-round sim step, epic #64).
Shared: components/shell/ (TopBar with Play, Pool, Fairness, Replay, Docs, GitHub + wallet pill; Footer; MockStrip; ExampleTag), components/ui/ (CornerBox, SectionRule, KV row, Pill), styles/tokens.css (from base.css).

ORDER OF WORK
1 lib/types.ts + game-client.ts + mock-data.ts + shell components (MockStrip, ExampleTag)
2 /play state A (#56) -> state B, C (#58) -> claim cards (#59, #122) -> bet wiring against the interface (#57)
3 /pool (#10, then #36, #51)   4 /fairness (#54)   5 /replay (#63)
Each PR small (about 150 lines), one issue, "Closes #N" (use "Refs #N" if the page stays on mock data, so the issue stays open until the real client lands).

GAPS (no issue today; propose these as new)
G1 Data layer: types.ts, game-client.ts, mock-data.ts, format.ts, payouts.ts, env flag, MockStrip + ExampleTag. Done when pages only import lib/game-client and a switch of the flag changes the strip.
G2 /play states and query switch: bowl-covered.jpg, bowl-open.jpg stills, a ?state= mock switch, "late joiner sees the result" case. (Part of #58 if the dev prefers.)
G3 Real client (program-client.ts): implement each method against the deployed program (accounts, instructions, ER vs Solana routing). Done when flag=program works on devnet and the mock strip is gone. Owner: after the program side is done.
G4 Replay data source: a script that writes the 50-round fixture to app/public/sim.json (so /replay can load it), plus a jump-to-round control.
G5 Docs page (/docs) is in the nav but has no issue. Either remove the link for now or make a short page from the README (#68).
Also: the first mock links "Docs" and "GitHub" in the nav. GitHub = the repo URL; Docs = README on GitHub until a page exists.
