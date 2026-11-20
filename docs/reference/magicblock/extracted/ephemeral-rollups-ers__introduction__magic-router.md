

# Magic Router
Source: https://docs.magicblock.gg/pages/ephemeral-rollups-ers/introduction/magic-router

Learn how Magicblock's Magic Router accelerates transactions by routing them to the right execution environment.

[Why Build with MagicBlock? Solve Scaling & UX Pain Points with Ephemeral Rollups](https://www.youtube.com/embed/Wc2XMAKtEug?si=bjitIDG7w5fY5zmK\&list=PLWR_ZQiGMS8mIe1kPZe8OfHIbhvZqaM8V)

***

## Dynamic Transaction Routing

**Magicblock's Magic Router** is an dynamic transaction routing engine that accelerates transactions by intelligently deciding where they should be executed — either on **Ephemeral Rollups** or **Solana** — based on transaction metadata.

This eliminates the need for manual routing logic from the developer, providing significant benefits in transaction speed and development experience.

* ✅ **Simple Integration with Single Endpoint**: Send transactions and read accounts through one RPC endpoint. The router does not serve every Solana RPC method (for example `simulateTransaction`, `getSlot`, and `getProgramAccounts`); see the [Router API](/pages/ephemeral-rollups-ers/api-reference/er/introduction).
* ✅ **Seamless Wallet Experience**: Connect, sign, and submit — no need to know what’s happening behind the scenes.
* ✅ **Faster Confirmations**: Magic Router routes to the fastest available endpoint.

![](https://mintcdn.com/magicblock-42/5iyVpKJBt1PkwHw4/images/magic-router.png?fit=max\&auto=format\&n=5iyVpKJBt1PkwHw4\&q=85\&s=89797c78d3081846949e7e8ad36fd3fb)

***

## Quick Access

> [card] [Anchor](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/counter/anchor)
>
> Integrate with an Anchor program

> [card] [Native Rust](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/counter/native-rust)
>
> Integrate with a Native Rust program

> [card] [API](/pages/ephemeral-rollups-ers/api-reference/er/getRoutes)
>
> Experiment with Magic Router API

***

## Code Snippets

Initialize connection with Magic Router before you send transactions dynamically.

> [!NOTE]
>
> These public RPC endpoints are currently free and supported for development:
> \
> &#x20;Magic Router Devnet: [https://devnet-router.magicblock.app](https://devnet-router.magicblock.app) \\

Choose your preferred SDK to initialize, send and confirm transactions:

* `ephemeral-rollups-kit` for `@solana/kit`
* `ephemeral-rollups-sdk` for `@solana/web.js`

```typescript Kit theme={null}
import { Connection } from "@magicblock-labs/ephemeral-rollups-kit";

// Initialize connection
const connection = await Connection.create(
  "https://devnet-router.magicblock.app",
  "wss://devnet-router.magicblock.app"
);

// ... create transaction

// Send and confirm transaction
const txHash = await connection.sendAndConfirmTransaction(
  transactionMessage,
  [userKeypair],
  { commitment: "confirmed", skipPreflight: true }
);
```

```typescript Web3.js theme={null}
import { sendAndConfirmTransaction } from "@solana/web3.js";
import { ConnectionMagicRouter } from "@magicblock-labs/ephemeral-rollups-sdk";

// Initialize connection
const connection = new ConnectionMagicRouter(
  "https://devnet-router.magicblock.app/",
  { wsEndpoint: "wss://devnet-router.magicblock.app/" }
);

// ... create transaction

// Send and confirm transaction
const txHash = await sendAndConfirmTransaction(connection, tx, [payer], {
  skipPreflight: true,
  commitment: "confirmed",
});
```

The Magic Router analyzes each transaction’s metadata (e.g. writable accounts, owner, and signer) and automatically routes it to the closest dedicated endpoint:

![](https://mintcdn.com/magicblock-42/5iyVpKJBt1PkwHw4/images/magic-router-scenarios.png?fit=max\&auto=format\&n=5iyVpKJBt1PkwHw4\&q=85\&s=4ef8b83ef98061f9073bb11f834d7cc2)

1. **Client - Transaction Submission** The dApp or user sends a transaction to the
   Magic Router RPC endpoint.

2. **RPC - Metadata Inspection**
   The Magic Router inspects the transaction metadata and checks the owner of writable accounts.

3. **Validator - Smart Routing and Execution**
   Based on the metadata, the router determines whether to send it to:

   * **Ephemeral Rollup** for fast, low-latency, zero-cost execution
   * **Solana** for persistent, high-throughput execution

***


# Ephemeral Rollup Runtime Limits
