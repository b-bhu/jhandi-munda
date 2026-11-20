

# Verifiable Randomness (VRF) on Solana
Source: https://docs.magicblock.gg/pages/verifiable-randomness-functions-vrfs/introduction/solana-vrf

Get verifiable, unpredictable randomness in a Solana program. Request it onchain and receive the value in a callback with a proof, for games, raffles, and loot drops.

MagicBlock's Solana VRF gives Solana programs a verifiable randomness primitive for gameplay, lotteries, matchmaking, and other real-time flows that need fair outcomes without trusting blockhashes or app servers.

> [!NOTE]
>
> Solana VRF is available on both Solana mainnet and MagicBlock Ephemeral Rollups.

> [card] [Start Building](/pages/verifiable-randomness-functions-vrfs/how-to-guide/quickstart)
>
> Build a Solana VRF request and callback flow with the MagicBlock VRF SDK.

> [card] [Technical Details](/pages/verifiable-randomness-functions-vrfs/introduction/technical-details)
>
> See how requests, proofs, oracle queues, and callbacks work.

> [card] [Pricing](/pages/verifiable-randomness-functions-vrfs/introduction/pricing)
>
> Review per-request pricing and estimate monthly VRF costs.

> [card] [Security & Audits](/pages/overview/additional-information/security-and-audits)
>
> Read the audit report and security notes.

> [card] [Open Source VRF](https://github.com/magicblock-labs/solana-vrf)
>
> Inspect the program, SDK, proof implementation, and examples on GitHub.

## What is Solana VRF?

Solana VRF is a verifiable random function implementation for Solana programs. Your program requests randomness, an oracle computes a random value with a cryptographic proof, and the MagicBlock VRF program verifies that proof before calling back into your program.

That proof makes the result auditable. Users and programs can verify that a random outcome was produced from the committed request instead of being chosen after the fact by a validator, server, or game operator.

## Why use MagicBlock Solana VRF?

Traditional randomness sources fall short on-chain: users cannot verify how off-chain numbers were generated, validators or operators can influence blockhash-based outcomes, and off-chain delivery does not always align with on-chain execution timing. Solana VRF addresses this:

* **Built for Solana programs**: request randomness through the `ephemeral-rollups-sdk` VRF module and consume the result in your own callback instruction.
* **Designed for real-time apps**: MagicBlock's ephemeral rollup execution model keeps randomness delivery low-latency for games and interactive flows.
* **Verifiable by design**: proofs are validated on-chain before your callback logic runs.
* **Open source and audited**: the VRF program is public, with audit coverage linked from the security docs.

## How Solana VRF works

1. Your program submits a randomness request with a `caller_seed`, callback discriminator, and callback accounts.
2. The request is added to an oracle queue for fulfillment.
3. A verified oracle computes the random value and proof.
4. The MagicBlock VRF program verifies the proof.
5. Your callback receives the random bytes and converts them into game or app logic, such as dice rolls, loot drops, or raffle winners.

## Common use cases

* Loot drops and gacha
* Matchmaking and shuffling
* Raffles and giveaways
* Randomized resource generation
* Random character attributes or NFT traits
* Any Solana app that needs auditable fairness

## Start with the right guide

Use the [Solana VRF quickstart](/pages/verifiable-randomness-functions-vrfs/how-to-guide/quickstart) when you are ready to implement request and callback instructions. Use the [security guide](/pages/verifiable-randomness-functions-vrfs/introduction/security) to review callback validation, signer checks, and audit links before going live.


# VRF Technical Details
