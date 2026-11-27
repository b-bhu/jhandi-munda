# Devnet deploys

Devnet only. Program `4nsQ6ntZgpBhsDJt4bN5nu4bBvyqXtD4ReQdhAdSRhtH`, upgrade authority and config admin `5qPRTWcUZw1XYGnXqUCCF65TGBgMjk9BfMyz2VBHacm7` (`keys/dev-wallet.json`, gitignored).

## 27 Nov 2026, evening IST: #130 (grant_session replaces an existing cap)
- Code: `main` at `906f922` plus #130 (squash-merged as `bc9193d`). `anchor build`, `cargo test -p jhandi_munda`: 96 passed, 0 failed, 1 ignored.
- `.so` 1,250,568 bytes, program data 1,257,653, so no extend was needed. Upgrade (slot 526874437): [38L9Yru9…](https://explorer.solana.com/tx/38L9Yru9MXvkXNF9G63kbCc46zPC4R6tiQGCt62J6K6PtuU3csQG5VKDAAHhpBM2a4dKz4RcuJhLsy7LfMAeuw6x?cluster=devnet)
- Review: `GrantSession.cap` goes from `init` to `init_if_needed`, same seeds `["session_cap", owner]` with `owner` a `Signer`. Only the wallet can sign for its own PDA, so nobody else can replace its cap. The handler overwrites owner, signer, cap, spent, expiry and bump. There is no realloc or close, and `init-if-needed` was already on.
- Full script `node scripts/devnet-checks-112.mjs`, run twice in a row, both with the existing cap replaced and no `InvalidSession` (6057). Checks 2, 3, 4, 5, 6 PASS each time (the "second take_snapshot should fail" line in check 2 is the expected negative test).
  - Run 1, round 15, faces [2, 2, 4, 5, 5, 2], a 3-match chip win of 4.92 MUSD queued and paid:
  - grant_session (replaces the old cap): [4z6gVWfi…](https://explorer.solana.com/tx/4z6gVWfixafuY7jteNdHHjYihBLMtVcZrpXdrZ1JYRtGe8Pe5KtpQE54FfbBeH2M9mxe5Cbg1NeisjfCkqdYHY6X?cluster=devnet)
  - create_session: [TcGWFQ6U…](https://explorer.solana.com/tx/TcGWFQ6U4xJgyTfZJczvJ7MzQKXLyin1sHPSNHmziSkzUbRLYLNEUmeH6XsGKMYK6pGb5YAxTJGfYGwZ6rqHbbi?cluster=devnet)
  - open_chip_bet: [4VuhrcHP…](https://explorer.solana.com/tx/4VuhrcHPWttvrZ29JwiADrixbHbXE85k6q5p3HFtmzLJ1qsjMDCnqPkNAFxduH1w65ejXMoWXpixMBTcnKg5ZcBw?cluster=devnet)
  - place_bet_chips_session: [4pQYm7AY…](https://explorer.solana.com/tx/4pQYm7AY7bsa7oLFW5sHG9GpzYSnmtei6egjsErSBkKTrtwe8TqWSJD1HARbjFgPYTLj32KjPkmV8ERG4DjTPZpD?cluster=devnet)
  - request_roll: [5bFXqWdR…](https://explorer.solana.com/tx/5bFXqWdRBeiUpZSVvMScqMFSWcvw8XGG9x6EaqkrHDEE9RqCA2C1xZvLqcjAw73scc7zcuaZEJLQdjvrfcQPjVP5?cluster=devnet)
  - settle_chip_bets: [5UzRfFR4…](https://explorer.solana.com/tx/5UzRfFR477hgXN9dexXMZx1FZ9F8RPKJ2yGvCPqRAgJEJsiLEq959HBHgBiZMHchQRhrKkZFYHc9vfkjARNojczd?cluster=devnet)
  - pay_queue: [4e9e5vuz…](https://explorer.solana.com/tx/4e9e5vuzP1BJrSGajoQGxZ3vjrsHUiLWV22vH5wdjiqKipUEPb9gxLxS9bJ7S6BMja1JxBAaDGzH1x1mbMK3Sjg9?cluster=devnet)
  - Run 2, round 16, faces [4, 3, 5, 2, 1, 2]:
  - grant_session (replaces the run 1 cap): [3wtjMmKw…](https://explorer.solana.com/tx/3wtjMmKwJxX3gXewbhmbnShBu3ZyYpnxkn776Auop997t1webeYMzB3j6fbovPTkaBL9K8592phnfXaCpiGQmyS7?cluster=devnet)
  - create_session: [4QJKfvwy…](https://explorer.solana.com/tx/4QJKfvwyKvQVxpzXtBBhGuv58Ki8bGYbP3vHBZPrvT1mbJ4ropiVTapWGwfKTtE5jgh6cjuGMNTtryk6p3goLK28?cluster=devnet)
  - open_chip_bet: [WEn7X1s2…](https://explorer.solana.com/tx/WEn7X1s25FtsQeoVm561SXixuUezgvBejwRJfY1h7cTVvGnb88V6FsxR9HBVrEFgM6sEQVU9sDDTWqo1zFNfrgL?cluster=devnet)
  - place_bet_chips_session: [8FwAXsZ9…](https://explorer.solana.com/tx/8FwAXsZ9WLxkfiqnMvfEekEyvS5LYVf32WQeTi3p52fBsHJepNKDRvbo3c8zThinmrM4R6CekDM6qZsUXZFmXA3?cluster=devnet)
  - request_roll: [2MNQZRem…](https://explorer.solana.com/tx/2MNQZRemf2qHRbx5VB6mZK8bQUdq6rv6vDAUpFQvsWPVs1Ny8Pod6hX85aPqj6cbwUaVZCS7AQxPZu1Vw3e13vMZ?cluster=devnet)
  - settle_chip_bets: [fbvBfLuc…](https://explorer.solana.com/tx/fbvBfLucsXMwFybJ3huV8H92QSyHqePSmKKd8ijR2qtEDCiRpfeZzNJFXVyJtHBpQ2nKNWdyD5M66XrvGyATZRr?cluster=devnet)
  - pay_queue: [LKgsYz1z…](https://explorer.solana.com/tx/LKgsYz1zLX9aeWjW344fyg1FPsRrouukSQVevJsmLSnoaKuwiocwdX5FqCqRF2jbKTLG1oFTGcePDHHE5cBpQBh?cluster=devnet)
- Not covered here: the ER delegation of the session token and `SessionCap` (still unverified).
- Side effect: the script withdraws all LP shares each run, so the pool holds only what the round left.

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

## 27 Nov 2026, 17:00 IST: #124 (D50) + #126 (D51), program upgrade only
- Code: `main` at `ccceda6` (#131 loser claims Wait only, #133 chip-road losers get MUNDA). `cargo test -p jhandi_munda`: 95 passed, 0 failed, 1 ignored.
- `.so` 1,247,832 bytes; program data is 1,257,653, so no extend. Upgrade (slot 526850978): [2cUKcEDc…](https://explorer.solana.com/tx/2cUKcEDcuZFpbyJEPZNMXxKLx3VxpuhoBpkmFKPp3LH9foK26A4LoRke98bFSz3Dw157qqLSCzw34LFUxedfvxdw?cluster=devnet)
- No account layout change in these PRs (only `claim_open.rs`, `claim_redeem.rs`, `error.rs`, `constants.rs`), so config stays 91 bytes and no migration was run.
- Proof run: `node scripts/devnet-loss-claims-126.mjs` (2 rounds; pool topped up with 40 MUSD first: [63Z3mSCj…](https://explorer.solana.com/tx/63Z3mSCjXvB3gB333NEc8qzL1u8ReSdusx3WQWZReTxyNUtE7jyUSa4ekxi45eDs8a74z26bhPnY2BadXk9yWgWx?cluster=devnet)). Each round: $1 MUSD bets on faces 1-3, $1 chip bets on faces 4-6.
  - Round 13, faces [4, 6, 2, 5, 4, 3]: request_roll [56Ehc2EA…](https://explorer.solana.com/tx/56Ehc2EAAoczqjifmic57n4MVZgo2caDLLw7zp58LzP4hExFaBrgbE7EAXTK4UMmRUXEdBwXXUZHbxmJYq9LzpJ2?cluster=devnet), settle_chip_bets [5fr6NR1R…](https://explorer.solana.com/tx/5fr6NR1RsEAiCKWjbn834J9M8f3kxr3kywKy2zp3t8QgbR442599NkzjP4bwSqWMgYLB7tCx7QdbWJ6jKbtTqzaG?cluster=devnet), settle_bets [3XsZkxBY…](https://explorer.solana.com/tx/3XsZkxBYC4d91X3UXCemRmkbAyy8T44Qye53kDsKL3QjCCQxAyGTW49dA8NDKo9MrLiY2fXxRjZhF7RghCTsGxpt?cluster=devnet), finish_round [2QDmJKQJ…](https://explorer.solana.com/tx/2QDmJKQJkBeNWaidAvv9qFdRBfgXCoZtJgeScbQyNzy7Y8FBunvAt9tMi2mTp4RyvjuqYhuSaKevGx6uM1uaKVHJ?cluster=devnet).
  - Wallet (MUSD) loss, face 1 (0 matches): open_claim [4Dh1RzfW…](https://explorer.solana.com/tx/4Dh1RzfW8R8pVVe2qrgW3cSSouaCmLNiK8Qgax7Y9jznakPyzLTerp34zud8WiJQqMA6SifSLeibu8yZxxrbtq2J?cluster=devnet) opened a MUNDA claim of 5.000000 (5 x $1, loss tally 0 -> 1.000000), unlock +24h, early_exit_bps 0. Take now refused with `LossClaimWaitOnly` (6065): [5nCntAQH…](https://explorer.solana.com/tx/5nCntAQHwm5Qkk3CJ72Pp1eSmAj2XTEodo8cNxD1Y6uBoguYiV7hAJYaakodC3SdV7xkpuBKRmSjjq7LAY7Mq3uv?cluster=devnet)
  - Chip-bet loss, face 5 (1 match): open_claim [4GHFbS7f…](https://explorer.solana.com/tx/4GHFbS7f5rUdwmrtiX7sXN6HT5zYeBMiygqYXf91a3V1nk6doFy5EQ1vgtU4fwa5uTd3mYzdEQVrDupWiB8bCJQq?cluster=devnet) opened a MUNDA claim of 5.000000 (5 x $1, loss tally 1.000000 -> 2.000000), unlock +24h, early_exit_bps 0. Take now refused with `LossClaimWaitOnly` (6065): [2ib6c9Rr…](https://explorer.solana.com/tx/2ib6c9RrFFpy1bBMe7Z1rU8nCG1usSmEce6aU3zgjDf46eZuUj6oTDeYotTXBGugztEtGifTutn4ctytdF3PwpuK?cluster=devnet)
  - A winning chip bet (face 4, 2 matches, 2.96 MUSD paid into chips at settle) still has no claim: open_claim refused with `ChipBetSettledToChips` (6044): [5xgC2L5h…](https://explorer.solana.com/tx/5xgC2L5h9Yq4wwSrWpMRqZKLeFAyDpx9W3WaTmt3x6yogHQTNvVHNZVii3pd76yxWLfRaNZyWYgF7jhAY4zWEhy5?cluster=devnet)
  - Round 14, faces [2, 5, 5, 1, 3, 1]: request_roll [5DuBfq6J…](https://explorer.solana.com/tx/5DuBfq6JzUr6npQrQgC3qrfLGigEHQ3JiQhGjjLteUDQQ81iW7HanrV6JMiJ4dWv7h55em5BMyk3RafhwRpLN1Aj?cluster=devnet), settle_chip_bets [5Rqjdspf…](https://explorer.solana.com/tx/5RqjdspfbtW2YbjusKAUju3GBsmEcBMHH1gZSk3H5HBE6q7CkLvFL9Ry5FQT2C9Y5sJx5jZE637vdL6FbHwftCLS?cluster=devnet), settle_bets [55ynF3oz…](https://explorer.solana.com/tx/55ynF3ozbJB8gQSbeLPiSJokYtg2KinSYCVsNgxNu8j9P2jB3rD26y781etU44YqfN7ZJRWZGU78VnUcRjXbDHC6?cluster=devnet), finish_round [4Wu4rYus…](https://explorer.solana.com/tx/4Wu4rYusk2ZFfkMom3b5oApu9ogyYtPLuxoyywKYTF5ACnymvZmdnmUBJgyVXWoc7YvzvQfFTPeeD2a8dC1ca1z1?cluster=devnet).
  - Wallet (MUSD) win, face 1 (2 matches, owed 2.960000): open_claim [5mwcJntb…](https://explorer.solana.com/tx/5mwcJntb3Sm98vyvZpyUVJ856wmog57DD4cpvQwi7mpTjb8d4cJJLa8b4QpV6NXkiQwipwKcAV1FCeZXs1ZizFaR?cluster=devnet) (early_exit_bps 500), then Take now [2fD9emSZ…](https://explorer.solana.com/tx/2fD9emSZFUVfATSQEsjPTMUqKv5anWLBHM4iXiYSXqAudH4A8aGufiyyS7AJfXavcDHx1v4xSjpeXBabKaW1m7TG?cluster=devnet) paid 2.812000 (95%).
- Not tested: Wait redeem of a loss claim (needs 24h; claims unlock at +24h from settle, ~28 Nov 2026 ~17:00 IST). The loss MUNDA stake credit and the stake account creation therefore are covered by unit tests only so far.
- State after: loss tally 2.000000 MUSD; pool vault 47.08 MUSD; this wallet holds the 2 loss claims (bets `4Vh99mf8…`, `8dKM68VD…`) to redeem by Wait after 24h.
- Script note: `scripts/devnet-loss-claims-126.mjs` is a standalone proof script (does not use session keys, so the #130 rerun bug does not affect it).
