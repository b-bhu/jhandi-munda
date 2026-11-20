Solana JSON-RPC method getVersion.

# RPC API
Source: https://docs.magicblock.gg/pages/ephemeral-rollups-ers/api-reference/rpc/introduction

Solana JSON-RPC HTTP methods on MagicBlock devnet RPC endpoints.

This section documents the Solana JSON-RPC HTTP method catalog on MagicBlock endpoints:

**Devnet:**

* `https://devnet-as.magicblock.app/`
* `https://devnet-us.magicblock.app/` (runs validator 2.0; the other public ERs run 1.1.1)
* `https://devnet-eu.magicblock.app/`
* `https://devnet-tee.magicblock.app/`

**Mainnet:**

* `https://as.magicblock.app/`
* `https://us.magicblock.app/`
* `https://eu.magicblock.app/`
* `https://as-tyo.magicblock.app/`
* `https://sa.magicblock.app/`
* `https://mainnet-tee.magicblock.app/`

`devnet-us` runs validator 2.0, which reports commits through settlement outbox logs instead of the `ScheduledCommitSent` logs of 1.1.1. SDK `0.18.x` reads both log formats. SDK `0.20.x` reads only the 2.0 format, so use it only against 2.0 endpoints such as `devnet-us`.

The method catalog is aligned with Solana RPC HTTP methods:

* [Solana RPC HTTP Methods](https://solana.com/docs/rpc/http)

> [!NOTE]
>
> TEE RPC endpoints may require an authentication token for certain methods. See
> [Authorization
> guide](/pages/private-ephemeral-rollups-pers/how-to-guide/quickstart#4-authorize)
> for details.


# isBlockhashValid
