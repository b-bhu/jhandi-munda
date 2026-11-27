# Devnet deploys

Devnet only. Program `4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH`, upgrade authority and config admin `5qPRTWcUZw1XYGnXqUCCF65TGBgMjk9BfMyz2VBHacm7` (`keys/dev-wallet.json`, gitignored).

## 27 Nov 2026, 16:10 IST: Step 18 + #127 + #128
- Code: `main` at `f9952aa` (program code as of `2525fee`, #128). Built with `anchor build` (anchor-cli 1.0.2, solana-cli 3.1.9). `cargo test -p jhandi_munda`: 94 passed, 0 failed, 1 ignored.
- `.so` 1,247,672 bytes; program data was 1,187,653, so it was extended by 70,000 bytes first (now 1,257,653).
  - extend: [https://explorer.solana.com/tx/5kQUG4S71HAW4LLfodXJBLG4kw3TF2rR9amMi1CXRnzgSrxdn4oDAbramMubV1V3h2HNQj9GAGpvp7b1zuh4wAf6](https://explorer.solana.com/tx/5kQUG4S71HAW4LLfodXJBLG4kw3TF2rR9amMi1CXRnzgSrxdn4oDAbramMubV1V3h2HNQj9GAGpvp7b1zuh4wAf6?cluster=devnet)
  - upgrade (slot 526841116): [xCb37j2D…](https://explorer.solana.com/tx/xCb37j2DoQrEP5uJMdJWchRF9FtCJuft89DhLhV6gA3XdmhMHPFxJZRdxwskJFbvuXaZatTaktkMGpNhSDY41pt?cluster=devnet)
- `migrate_config` right after: [4udnHt1F…](https://explorer.solana.com/tx/4udnHt1FXXriZB4o2V6c9swRvVasPYA5843gL1eeSi6NmiDEanUind6PnPzAUHc3pGfJFdPF1ENkuekFLUFKLUPg?cluster=devnet)
  - config `7qArYyj2GbkBBPmXfipkiRFTbC8kDSML4WnWwmY59jy6`: 74 -> 91 bytes. The first 74 bytes are unchanged (admin `5qPR…acm7`, MUSD mint `9ixsxMHJHJJUSNCzQrpP511143s9xksBmYxFAomwjr56`, bumps 255/253). New: `transfers_enabled` false, `overflow_target` 10,000 MUSD, `overflow_mark` 0.
- Round proof (`node scripts/devnet-checks-112.mjs --queue-only`, checks 2, 3, 6 PASS), round 12, faces [2, 2, 4, 6, 4, 5], a 2-match chip win of 2.96 MUSD queued and paid:
  - place_bet_chips: [45KEdeQT…](https://explorer.solana.com/tx/45KEdeQTrCqz9yV6smK8U6dYwg5Xw1s19VWgTexnDSDPRwovZqdhrRNnD15y6pMdXGHiBZqBBWuHCnrDVqRYeqob?cluster=devnet)
  - request_roll: [4hSyiicb…](https://explorer.solana.com/tx/4hSyiicby5eaYtM8hBzGH5euGPcpyiSQWdqvHht6yZvQ7ZkrGDd27mgy8z8v5iKoo2ZQQzR9SwNwRnqbBFihfcRa?cluster=devnet)
  - VRF callback: [5aQ75QwP…](https://explorer.solana.com/tx/5aQ75QwPrQafMvV8sbHgZW69afP1fH4N3HaB2tm9jJAZBH83Turikxv1bmL3iq2s4CvmJ8V2Ma8iMap1aXhyAZtF?cluster=devnet)
  - settle_chip_bets: [5XFUaLXU…](https://explorer.solana.com/tx/5XFUaLXUZQcRkQy1BFj4ksjhwYn7eBMm5umXBn9AdZyBfiASyP2ChHZhc6mTaPqDUw9nUhNSP4fde7Hijx4J1hSN?cluster=devnet)
  - deposit (new `config` account, #127): [3AtDWmm9…](https://explorer.solana.com/tx/3AtDWmm9UzmdePSengkQyadbku3gEovGUJtMkiBohcMVVCT87PUuAFafhqKJ7e83Z3NNszwZc3qn1G21aXNNAaM9?cluster=devnet)
  - pay_queue: [5yX32eVC…](https://explorer.solana.com/tx/5yX32eVCaGaYvXV2nWPxcjM4wAP7egB7d5GAzq8dgxafh9pm64DpkQasJHe1ouuVxrEnMHaNMuMUCsyiqVrUfJLS?cluster=devnet)
  - finish_round: [5bWST8rV…](https://explorer.solana.com/tx/5bWST8rVjWo1LLv3WgmWn3x2uJEC7s4bmpbo8KUYTAJ7xr2Gort3vHDpe3rb2FR3w6B9EhpeJ5UxSvfLbpRicSfd?cluster=devnet)
  - withdraw (new `config` account, first full run): [4oCsLJHe…](https://explorer.solana.com/tx/4oCsLJHepsKmSkMv5hiCgXGjNep4BGWHo3DjoEY4n6Vyjx6U9RjRtXKfrY6t1btvnKDtbatQ5iwEBb6z8ud36ffg?cluster=devnet); take_snapshot: [2QNPYtJn…](https://explorer.solana.com/tx/2QNPYtJnJR6VgYq6Z9PhSvEQi7gxbr6UqPt9kwhSwPHn9MvzhtrkzjTzjiC1GPgcAWDcgDfSXpiSFnDG2fDVj3qB?cluster=devnet)
- Known script gap: the full run (without `--queue-only`) fails at check 4, `place_bet_chips_session` -> `InvalidSession` (6057). The wallet's `SessionCap` is init-only and still names an earlier run's session signer, while the script makes a new signer every run. Not caused by the upgrade. Fix: reuse the stored signer, or let `grant_session` update an existing cap.
- Not covered on devnet in this run: normal (MUSD) bets with `claim_open` / `claim_redeem`, and loss MUNDA minting. Those are covered by the unit tests.
- Side effect: the script empties the pool on purpose (withdraws all LP shares). After the run the pool holds what the round left in it.
