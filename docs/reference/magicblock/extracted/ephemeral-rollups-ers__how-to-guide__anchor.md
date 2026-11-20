

# Ephemeral Rollups with Anchor
Source: https://docs.magicblock.gg/pages/ephemeral-rollups-ers/how-to-guide/anchor

Learn how to write a simple Anchor program that increments a counter on Solana

> [!TIP]
>
> **Building with an AI coding agent?** Install the MagicBlock Dev Skill to give your agent MagicBlock-specific patterns — delegation flows, Magic Actions, cranks, VRF, and more.
>
> **Hit an error?** Ask your coding agent with the skill installed, not the docs assistant. The assistant only sees the docs, so it cannot debug your code.
>
> Install it for Claude Code, Cursor, Codex, and other agents:
>
> ```bash theme={null}
> npx skills add https://github.com/magicblock-labs/magicblock-dev-skill
> ```
>
> See the [AI Dev Skill](/pages/overview/additional-information/ai-dev-skill) page for manual and project-scoped installs.

[Build a real-time Anchor Counter on Solana with MagicBlock's Ephemeral Rollup | Tutorial](https://www.youtube.com/embed/qwu2RBKyFiw?si=PMg-3UbRfvvbrs7C\&list=PLWR_ZQiGMS8mIe1kPZe8OfHIbhvZqaM8V)

This guide will walk you through the process of writing a simple Anchor program that increments a counter. You'll learn how to deploy this program on Solana and interact with it using a React client.

## Software Packages

This program is developed and tested with the following software packages. Other versions may also be compatible.

| Software | Version | Installation Guide |
| - | - | - |
| **Solana** | 3.1.9 | [Install Solana](https://docs.anza.xyz/cli/install) |
| **Rust** | 1.89.0 | [Install Rust](https://www.rust-lang.org/tools/install) |
| **Anchor** | 1.0.2 | [Install Anchor](https://www.anchor-lang.com/docs/installation) |

## Quick Access to Source Code

If you prefer to dive straight into the code:

> [card] [Source Code: Anchor Counter Program](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/anchor-counter)

> [card] [Live Example App](https://counter-example.magicblock.app/)

## Writing the Anchor Program

Let's break down the key components of our counter program:

## Core Functionality

The program implements two main instructions:

1. `initialize`: Sets the counter to 0
2. `increment`: Increments the counter by 1

Here's the core structure of our program:

```rust theme={null}
#[ephemeral]
#[program]
pub mod public_counter {
    use super::*;

    /// Initialize the counter.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count = 0;
        Ok(())
    }

    /// Increment the counter.
    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.count += 1;
        Ok(())
    }

    // ... Additional instructions will be added here
}
```

Nothing special here, just a simple Anchor program that increments a counter. The only difference is that we're adding the `delegate` macro to inject some useful logic to interact with the delegation program.

### Delegating the Counter PDA

In order to delegate the counter PDA, and make it writable in an Ephemeral Rollup session, we need to add an instruction which
internally calls the `delegate_account` function. `delegate_account` will CPI to the delegation program, which upon validation will gain ownership of the account.
After this step, an ephemeral validator can start processing transactions on the counter PDA and propose state diff trough the delegation program.

> [card] [Transaction (Base Layer): Delegate](https://solscan.io/tx/5tyEk8gkdP88xe8GL7X12f9WGZfYZtmDbuXozvqkpea4rphXPiNp91vAzmSjsyUR399wimBYazEszKSH35sidPRH?cluster=devnet)
>
> Inspect transactions details on Solana Explorer

```rust theme={null}
/// Add delegate function to the context
#[delegate]
#[derive(Accounts)]
pub struct DelegateInput<'info> {
    pub payer: Signer<'info>,
    /// CHECK The pda to delegate
    #[account(mut, del)]
    pub pda: AccountInfo<'info>,
}
```

```rust theme={null}
/// Delegate the account to the delegation program
/// Set specific validator based on ER, see https://docs.magicblock.gg/pages/ephemeral-rollups-ers/how-to-guide/local-development
pub fn delegate(ctx: Context<DelegateInput>) -> Result<()> {
    ctx.accounts.delegate_pda(
        &ctx.accounts.payer,
        &[COUNTER_SEED],
        DelegateConfig {
            // Optionally set a specific validator from the first remaining account
            validator: ctx.remaining_accounts.first().map(|acc| acc.key()),
            ..Default::default()
        },
    )?;
    Ok(())
}
```

### Committing while the PDA is delegated

The ephemeral runtime allows committing the state of the PDA while it is delegated. This is done by building a `MagicIntentBundleBuilder` with the `commit` intent and invoking it.

> [card] [Transaction (ER): Commit](https://solscan.io/tx/335F5UcgAW3P8wSaVoEPiBoxeykw9wR1kcJLWUnd9sHmxymCgGvLtxaGDy6ZKDgJw2ku9jtRqwVNY85DuAHvs9GT?cluster=custom\&customUrl=https://devnet.magicblock.app)
>
> Inspect transaction details on Solana Explorer

> [card] [Transaction (Base layer): Commit](https://solscan.io/tx/3xzMGbj2fwFCnGp6r7wyMv5LExmL6SXaEbt52DmxA3Wx1LyksuXpPJS4AAPtqoSeJe3aF7oY3jFNiEhBQKwhzRGN?cluster=devnet)
>
> Inspect transaction details on Solana Explorer

```rust theme={null}
use ephemeral_rollups_sdk::ephem::MagicIntentBundleBuilder;

/// Increment the counter and manually commit the account in the Ephemeral Rollup session.
pub fn increment_and_commit(ctx: Context<IncrementAndCommit>) -> Result<()> {
    let counter = &mut ctx.accounts.counter;
    counter.count += 1;
    // Serialize the Anchor account before the CPI sees it
    counter.exit(&crate::ID)?;
    MagicIntentBundleBuilder::new(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.magic_context.to_account_info(),
        ctx.accounts.magic_program.to_account_info(),
    )
    .commit(&[ctx.accounts.counter.to_account_info()])
    .build_and_invoke()?;
    Ok(())
}
```

### Undelegating the PDA

Undelegating the PDA is done by building a `MagicIntentBundleBuilder` with the `commit_and_undelegate` intent.
This commits the latest state and returns ownership of the PDA to the owner program.

> [card] [Transaction (ER): Undelegate](https://solscan.io/tx/3MN6Avygk3FwsuQPoKHzpLo9kBBommHwRHTdgxBooDwsaNHgSYr1yaDXYEe63Eii5DCVbtaK89Yyrhy3dSGhHmvw?cluster=custom\&customUrl=https://devnet.magicblock.app)
>
> Inspect transaction details on Solana Explorer

> [card] [Transaction (Base layer): Undelegate](https://solscan.io/tx/2YwzNQh632bJu8TSP9GPsXmh7o78KgMMM9QPNbdaAzamtjGZfmT4o7bkG2PtpCWp7aJmmxpxTXr4Qo36k5EjgPab?cluster=devnet)
>
> Inspect transaction details on Solana Explorer

```rust theme={null}
use ephemeral_rollups_sdk::ephem::MagicIntentBundleBuilder;

/// Undelegate the account from the delegation program
pub fn undelegate(ctx: Context<IncrementAndCommit>) -> Result<()> {
    MagicIntentBundleBuilder::new(
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.magic_context.to_account_info(),
        ctx.accounts.magic_program.to_account_info(),
    )
    .commit_and_undelegate(&[ctx.accounts.counter.to_account_info()])
    .build_and_invoke()?;
    Ok(())
}
```

## Connecting the React Client

The React client is a simple interface that allows you to interact with the Anchor program.
It uses the Anchor bindings to interact with the program and the MagicBlock SDK to interact with the Ephemeral Rollup session. Source lives alongside the program at [`anchor-counter/app`](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/anchor-counter/app).

&#x20;

> [card] [Live Example App](https://counter-example.magicblock.app/)

[https://counter-example.magicblock.app/](https://counter-example.magicblock.app/)

> [!WARNING]
>
> Iframes only work with some wallets (e.g. Backpack). Alternatively, try the
> deployed demo here: [https://counter-example.magicblock.app/](https://counter-example.magicblock.app/)

### Ephemeral Endpoint Configuration

To interact with the Ephemeral Rollup session, you need to configure the appropriate endpoint:

* For devnet, use the following ephemeral endpoint:

  [https://devnet.magicblock.app](https://devnet.magicblock.app)

* For mainnet, use a public ER endpoint such as `https://as.magicblock.app`, `https://eu.magicblock.app` or `https://us.magicblock.app`, or send through the Magic Router at `https://router.magicblock.app`. See the [RPC API](/pages/ephemeral-rollups-ers/api-reference/rpc/introduction) page for the full list.

* For [localhost](/pages/ephemeral-rollups-ers/how-to-guide/local-development), download, install, and run the ephemeral validator locally with the appropriate environment variables.

Make sure to update your client configuration to use the correct endpoint based on your development or production environment.

> [!NOTE]
>
> These public RPC endpoints are currently free and supported for development:
> \
> &#x20;Magic Router Devnet: [https://devnet-router.magicblock.app](https://devnet-router.magicblock.app) \
> &#xA;Solana Devnet: [https://api.devnet.solana.com](https://api.devnet.solana.com) \
> &#xA;ER Devnet (Asia): [https://devnet-as.magicblock.app](https://devnet-as.magicblock.app) \
> &#xA;ER Devnet (EU): [https://devnet-eu.magicblock.app](https://devnet-eu.magicblock.app) \
> &#xA;ER Devnet (US, validator 2.0): [https://devnet-us.magicblock.app](https://devnet-us.magicblock.app) \
> &#xA;TEE Devnet: [https://devnet-tee.magicblock.app/](https://devnet-tee.magicblock.app/) \
> &#xA;Find out more details&#x20;
> [here](/pages/ephemeral-rollups-ers/how-to-guide/local-development)
> .

> [card] [Source Code: Anchor Counter Program](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/anchor-counter)

> [card] [Live Example App](https://counter-example.magicblock.app/)


# Run Ephemeral Rollups Locally
