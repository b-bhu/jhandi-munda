

# Ephemeral Rollups Quickstart
Source: https://docs.magicblock.gg/pages/ephemeral-rollups-ers/how-to-guide/quickstart

Add Ephemeral Rollups to any Solana program. Delegate accounts, run gasless transactions on the rollup, then commit and undelegate back to Solana.

***

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

### Quick Access

Check out basic counter example:

> [card] [GitHub](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/counter/anchor)
>
> Anchor Implementation

> [card] [GitHub](https://github.com/GabrielePicco/ephemeral-counter-ui)
>
> React Implementation

***

[Build a real-time Anchor Counter on Solana with MagicBlock's Ephemeral Rollup | Tutorial](https://www.youtube.com/embed/qwu2RBKyFiw?si=PMg-3UbRfvvbrs7C\&list=PLWR_ZQiGMS8mIe1kPZe8OfHIbhvZqaM8V)

***

## Step-By-Step Guide

Build your program and upgrade it with delegation hooks with MagicBlock's Delegation Program `DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh`:

**1. [Write your program](#1-write-program)**

Write your Solana program as you normally would.

**2. [Add delegation and undelegation hooks in your program](#2-delegate)**

Add CPI hooks to delegate, commit and undelegate state accounts through
Ephemeral Rollup sessions.

> [!NOTE]
>
> These public validators are supported for development. Make sure to add the
> specific ER validator in your delegation instruction:
>
> **Mainnet**
>
> <ul>
>   <li>
>     Asia (as.magicblock.app):&#x20;
>     `MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57`
>   </li>
>
>   <li>
>     EU (eu.magicblock.app):&#x20;
>     `MEUGGrYPxKk17hCr7wpT6s8dtNokZj5U2L57vjYMS8e`
>   </li>
>
>   <li>
>     US (us.magicblock.app):&#x20;
>     `MUS3hc9TCw4cGC12vHNoYcCGzJG1txjgQLZWVoeNHNd`
>   </li>
>
>   <li>
>     TEE (mainnet-tee.magicblock.app):&#x20;
>     `MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo`
>   </li>
>
>   <li>
>     Asia Tokyo (as-tyo.magicblock.app):&#x20;
>     `MSHuBweMkNp4yPLDkPLA8bLRHbDmURWjYzsxrcBjix8`
>   </li>
>
>   <li>
>     South America (sa.magicblock.app):&#x20;
>     `MSARyz2yVTkqiYQWWgaEtUKug7oCF7PphE9GjEYHtDh`
>   </li>
> </ul>
>
> **Devnet**
>
> <ul>
>   <li>
>     Asia (devnet-as.magicblock.app):&#x20;
>     `MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57`
>   </li>
>
>   <li>
>     EU (devnet-eu.magicblock.app):&#x20;
>     `MEUGGrYPxKk17hCr7wpT6s8dtNokZj5U2L57vjYMS8e`
>   </li>
>
>   <li>
>     US (devnet-us.magicblock.app, validator 2.0):&#x20;
>     `MUS3hc9TCw4cGC12vHNoYcCGzJG1txjgQLZWVoeNHNd`
>   </li>
>
>   <li>
>     TEE (devnet-tee.magicblock.app):&#x20;
>     `MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo`
>   </li>
> </ul>
>
> devnet-us runs validator 2.0. Every other validator above runs 1.1.1. SDK
> `0.18.x` reads the commit logs of both; SDK `0.20.x` reads only 2.0 commit
> logs, so use it only with devnet-us.
>
> **Localnet**
>
> <ul>
>   <li>
>     Local ER (localhost:7799):&#x20;
>     `mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev`
>   </li>
> </ul>

**3. [Deploy your program on Solana.](#3-deploy)**

Deploy your program directly on Solana using Anchor or Solana CLI.

**4. [Ready to execute transactions for delegation and real-time speed](#4-test)**

Send transactions without modifications on-chain and off-chain that also
comply with the SVM RPC specification.

***

## Counter Example

![Counter GIF](https://mintcdn.com/magicblock-42/iteauKFqxDKE2Vln/images/gifs/counter-420w.gif?s=f6feee67754b5b0b90c94ad159d4e630)

The following software packages may be required, other versions may also be compatible:

| Software | Version | Installation Guide |
| - | - | - |
| **Solana** | 3.1.9 | [Install Solana](https://docs.anza.xyz/cli/install) |
| **Rust** | 1.89.0 | [Install Rust](https://www.rust-lang.org/tools/install) |
| **Anchor** | 1.0.2 | [Install Anchor](https://www.anchor-lang.com/docs/installation) |
| **Node** | 24.10.0 | [Install Node](https://nodejs.org/en/download/current) |

### Code Snippets

> [tab] 1. Write program
>
> The program implements two main instructions:
>
> 1. `initialize`: Sets the counter to 0
> 2. `increment`: Increments the counter by 1
>
> The program implements specific instructions for delegating and undelegating the counter:
>
> 1. `Delegate`: Delegates counter from Base Layer to ER (called on Base Layer)
> 2. `CommitAndUndelegate`: Schedules sync of counter from ER to Base Layer, and undelegates counter on ER (called on ER)
> 3. `Commit`: Schedules sync of counter from ER to Base Layer (called on ER)
> 4. `Undelegate`:
>    * Schedules sync and undelegation of counter (called on ER)
>
>    * Undelegation triggered through callback instruction injected through #\[ephemeral] (called on Base Layer through validator CPI)
>
> > [!NOTE]
> >
> > The undelegation callback discriminator `[196, 28, 41, 206, 48, 37, 51, 167]`
> > and its instruction processor must be specified in your program. This
> > instruction triggered by Delegation Program reverts account ownership on the
> > Base Layer after calling undelegation on ER.
> >
> > With [`[#ephemeral]`](/pages/ephemeral-rollups-ers/how-to-guide/quickstart#1-write-program) Anchor macro from MagicBlock's Ephemeral Rollup SDK, the undelegation callback discriminator and processor are injected into your program.
>
> ```rust theme={null}
> #[ephemeral]
> #[program]
> pub mod public_counter {
>     use super::*;
>
>     /// Initialize the counter.
>     pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
>         let counter = &mut ctx.accounts.counter;
>         counter.count = 0;
>         Ok(())
>     }
>
>     /// Increment the counter.
>     pub fn increment(ctx: Context<Increment>) -> Result<()> {
>         let counter = &mut ctx.accounts.counter;
>         counter.count += 1;
>         Ok(())
>     }
>
>     /// Delegate the account to the delegation program
>     /// Set specific validator based on ER, see https://docs.magicblock.gg/pages/get-started/how-integrate-your-program/local-setup
>     pub fn delegate(ctx: Context<DelegateInput>) -> Result<()> {
>     // ...
>     }
>
>     /// Manually commit the counter state in the Ephemeral Rollup session.
>     pub fn commit(ctx: Context<IncrementAndCommit>) -> Result<()> {
>     // ...
>     }
>
>     /// Increment the counter and commit in the same instruction.
>     pub fn increment_and_commit(ctx: Context<IncrementAndCommit>) -> Result<()> {
>     // ...
>     }
>
>     /// Undelegate the account from the delegation program.
>     pub fn undelegate(ctx: Context<IncrementAndCommit>) -> Result<()> {
>     // ...
>     }
> }
>
> pub const COUNTER_SEED: &[u8] = b"counter";
>
> /// Context for initializing counter
> #[derive(Accounts)]
> pub struct Initialize<'info> {
>     #[account(init_if_needed, payer = user, space = 8 + 8, seeds = [COUNTER_SEED], bump)]
>     pub counter: Account<'info, Counter>,
>     #[account(mut)]
>     pub user: Signer<'info>,
>     pub system_program: Program<'info, System>,
> }
>
> /// Context for incrementing counter
> #[derive(Accounts)]
> pub struct Increment<'info> {
>     #[account(mut, seeds = [COUNTER_SEED], bump)]
>     pub counter: Account<'info, Counter>,
> }
>
> /// Counter struct
> #[account]
> pub struct Counter {
>     pub count: u64,
> }
>
> /// Other context for delegation
> ```
>
> Nothing special here, just a simple Anchor program that increments a counter. The only difference is that we're adding the `ephemeral` macro for undelegation and `delegate` macro to inject some useful logic to interact with the delegation program.
>
> [⬆️ Back to Top](#code-snippets)

> [tab] 2. Delegate
>
> > [!NOTE]
> >
> > These public validators are supported for development. Make sure to add the
> > specific ER validator in your delegation instruction:
> >
> > **Mainnet**
> >
> > <ul>
> >   <li>
> >     Asia (as.magicblock.app):&#x20;
> >     `MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57`
> >   </li>
> >
> >   <li>
> >     EU (eu.magicblock.app):&#x20;
> >     `MEUGGrYPxKk17hCr7wpT6s8dtNokZj5U2L57vjYMS8e`
> >   </li>
> >
> >   <li>
> >     US (us.magicblock.app):&#x20;
> >     `MUS3hc9TCw4cGC12vHNoYcCGzJG1txjgQLZWVoeNHNd`
> >   </li>
> >
> >   <li>
> >     TEE (mainnet-tee.magicblock.app):&#x20;
> >     `MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo`
> >   </li>
> >
> >   <li>
> >     Asia Tokyo (as-tyo.magicblock.app):&#x20;
> >     `MSHuBweMkNp4yPLDkPLA8bLRHbDmURWjYzsxrcBjix8`
> >   </li>
> >
> >   <li>
> >     South America (sa.magicblock.app):&#x20;
> >     `MSARyz2yVTkqiYQWWgaEtUKug7oCF7PphE9GjEYHtDh`
> >   </li>
> > </ul>
> >
> > **Devnet**
> >
> > <ul>
> >   <li>
> >     Asia (devnet-as.magicblock.app):&#x20;
> >     `MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57`
> >   </li>
> >
> >   <li>
> >     EU (devnet-eu.magicblock.app):&#x20;
> >     `MEUGGrYPxKk17hCr7wpT6s8dtNokZj5U2L57vjYMS8e`
> >   </li>
> >
> >   <li>
> >     US (devnet-us.magicblock.app, validator 2.0):&#x20;
> >     `MUS3hc9TCw4cGC12vHNoYcCGzJG1txjgQLZWVoeNHNd`
> >   </li>
> >
> >   <li>
> >     TEE (devnet-tee.magicblock.app):&#x20;
> >     `MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo`
> >   </li>
> > </ul>
> >
> > devnet-us runs validator 2.0. Every other validator above runs 1.1.1. SDK
> > `0.18.x` reads the commit logs of both; SDK `0.20.x` reads only 2.0 commit
> > logs, so use it only with devnet-us.
> >
> > **Localnet**
> >
> > <ul>
> >   <li>
> >     Local ER (localhost:7799):&#x20;
> >     `mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev`
> >   </li>
> > </ul>
>
> 1. Add `ephemeral-rollups-sdk` with Anchor features to your program
>
> ```bash theme={null}
> cargo add ephemeral-rollups-sdk@0.18 --features anchor
> ```
>
> Use `0.18.x` with the public ERs, which run validator 1.1.1. A plain `cargo add` without a version installs `0.20.x`, the validator 2.0 line (currently only `devnet-us`), which needs `anchor-lang` 1.2.2 or later.
>
> Import `delegate`, `commit`, `ephemeral`, `DelegateConfig`, and `MagicIntentBundleBuilder` (which replaces the deprecated `commit_accounts` and `commit_and_undelegate_accounts` helpers):
>
> ```rust theme={null}
> use ephemeral_rollups_sdk::anchor::{
>   commit,
>   delegate,
>   ephemeral
> };
> use ephemeral_rollups_sdk::cpi::DelegateConfig;
> use ephemeral_rollups_sdk::ephem::MagicIntentBundleBuilder;
> ```
>
> 2. Add `delegate` macro and instruction, `ephemeral` macro, and `undelegate` instruction to your program. Specify your preferred delegation config, such as the ER validator. State reaches the base layer only when your program commits; the ER does not commit on a timer:
>
> ```rust theme={null}
> /// Add delegate function to the context
> #[delegate]
> #[derive(Accounts)]
> pub struct DelegateInput<'info> {
>     pub payer: Signer<'info>,
>     /// CHECK: The pda to delegate
>     #[account(mut, del)]
>     pub pda: AccountInfo<'info>,
> }
> ```
>
> ```rust theme={null}
> /// Delegate the account to the delegation program
> /// Set specific validator based on ER, see https://docs.magicblock.gg/pages/get-started/how-integrate-your-program/local-setup
> pub fn delegate(ctx: Context<DelegateInput>) -> Result<()> {
>     ctx.accounts.delegate_pda(
>         &ctx.accounts.payer,
>         &[COUNTER_SEED],
>         DelegateConfig {
>             // Optionally set a specific validator from the first remaining account
>             validator: ctx.remaining_accounts.first().map(|acc| acc.key()),
>             ..Default::default()
>         },
>     )?;
>     Ok(())
> }
> ```
>
> ```rust theme={null}
> use ephemeral_rollups_sdk::ephem::MagicIntentBundleBuilder;
>
> /// Manually commit the counter state in the Ephemeral Rollup session.
> pub fn commit(ctx: Context<IncrementAndCommit>) -> Result<()> {
>     MagicIntentBundleBuilder::new(
>         ctx.accounts.payer.to_account_info(),
>         ctx.accounts.magic_context.to_account_info(),
>         ctx.accounts.magic_program.to_account_info(),
>     )
>     .commit(&[ctx.accounts.counter.to_account_info()])
>     .build_and_invoke()?;
>     Ok(())
> }
>
> /// Increment the counter and commit the new state in the same instruction.
> pub fn increment_and_commit(ctx: Context<IncrementAndCommit>) -> Result<()> {
>     let counter = &mut ctx.accounts.counter;
>     counter.count += 1;
>     // Serialize the Anchor account before the CPI sees it
>     counter.exit(&crate::ID)?;
>     MagicIntentBundleBuilder::new(
>         ctx.accounts.payer.to_account_info(),
>         ctx.accounts.magic_context.to_account_info(),
>         ctx.accounts.magic_program.to_account_info(),
>     )
>     .commit(&[ctx.accounts.counter.to_account_info()])
>     .build_and_invoke()?;
>     Ok(())
> }
> ```
>
> ```rust theme={null}
> use ephemeral_rollups_sdk::ephem::MagicIntentBundleBuilder;
>
> /// Undelegate the account from the delegation program.
> /// Commits the latest state and returns ownership of the PDA back to the owner program.
> pub fn undelegate(ctx: Context<IncrementAndCommit>) -> Result<()> {
>     MagicIntentBundleBuilder::new(
>         ctx.accounts.payer.to_account_info(),
>         ctx.accounts.magic_context.to_account_info(),
>         ctx.accounts.magic_program.to_account_info(),
>     )
>     .commit_and_undelegate(&[ctx.accounts.counter.to_account_info()])
>     .build_and_invoke()?;
>     Ok(())
> }
> ```
>
> > `Delegation` is the process of transferring ownership of one or more of your program's `PDAs` to the delegation program. Ephemeral Validators will then be able to use the `PDAs` to perform transactions in the SVM runtime.
>
> > `Commit` is the process of updating the state of the `PDAs` from ER to the base layer. After the finalization process, the `PDAs` remain locked on base layer.
>
> > `Undelegation` is the process of transferring ownership of the `PDAs` back to your program. On undelegation, the state is committed and it trigger the finalization process. Once state it validated, the `PDAs` are unlocked and can be used as normal on base layer.
>
> [⬆️ Back to Top](#code-snippets)

> [tab] 3. Deploy
>
> Now you’re program is upgraded and ready! Fund your deployer wallet, then build and deploy to the
> desired cluster:
>
> ```bash theme={null}
> # Devnet: airdrop SOL to your configured keypair before deploying
> solana airdrop 2 --url https://api.devnet.solana.com
>
> anchor build && anchor deploy
> ```
>
> [⬆️ Back to Top](#code-snippets)

> [tab] 4. Test
>
> Ready to execute transactions for delegation and real-time speed.
>
> > [!NOTE]
> >
> > These public validators are supported for development. Make sure to add the
> > specific ER validator in your delegation instruction:
> >
> > **Mainnet**
> >
> > <ul>
> >   <li>
> >     Asia (as.magicblock.app):&#x20;
> >     `MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57`
> >   </li>
> >
> >   <li>
> >     EU (eu.magicblock.app):&#x20;
> >     `MEUGGrYPxKk17hCr7wpT6s8dtNokZj5U2L57vjYMS8e`
> >   </li>
> >
> >   <li>
> >     US (us.magicblock.app):&#x20;
> >     `MUS3hc9TCw4cGC12vHNoYcCGzJG1txjgQLZWVoeNHNd`
> >   </li>
> >
> >   <li>
> >     TEE (mainnet-tee.magicblock.app):&#x20;
> >     `MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo`
> >   </li>
> >
> >   <li>
> >     Asia Tokyo (as-tyo.magicblock.app):&#x20;
> >     `MSHuBweMkNp4yPLDkPLA8bLRHbDmURWjYzsxrcBjix8`
> >   </li>
> >
> >   <li>
> >     South America (sa.magicblock.app):&#x20;
> >     `MSARyz2yVTkqiYQWWgaEtUKug7oCF7PphE9GjEYHtDh`
> >   </li>
> > </ul>
> >
> > **Devnet**
> >
> > <ul>
> >   <li>
> >     Asia (devnet-as.magicblock.app):&#x20;
> >     `MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57`
> >   </li>
> >
> >   <li>
> >     EU (devnet-eu.magicblock.app):&#x20;
> >     `MEUGGrYPxKk17hCr7wpT6s8dtNokZj5U2L57vjYMS8e`
> >   </li>
> >
> >   <li>
> >     US (devnet-us.magicblock.app, validator 2.0):&#x20;
> >     `MUS3hc9TCw4cGC12vHNoYcCGzJG1txjgQLZWVoeNHNd`
> >   </li>
> >
> >   <li>
> >     TEE (devnet-tee.magicblock.app):&#x20;
> >     `MTEWGuqxUpYZGFJQcp8tLN7x5v9BSeoFHYWQQ3n3xzo`
> >   </li>
> > </ul>
> >
> > devnet-us runs validator 2.0. Every other validator above runs 1.1.1. SDK
> > `0.18.x` reads the commit logs of both; SDK `0.20.x` reads only 2.0 commit
> > logs, so use it only with devnet-us.
> >
> > **Localnet**
> >
> > <ul>
> >   <li>
> >     Local ER (localhost:7799):&#x20;
> >     `mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev`
> >   </li>
> > </ul>
>
> ```bash theme={null}
> anchor test --skip-build --skip-deploy --skip-local-validator
> ```
>
> Run the following test:
>
> ```typescript theme={null}
> const COUNTER_SEED = "counter";
>
> // Set Anchor providers
> const provider = new anchor.AnchorProvider(
>   new anchor.web3.Connection(
>     process.env.PROVIDER_ENDPOINT || "https://api.devnet.solana.com",
>     {
>       wsEndpoint: process.env.PROVIDER_WS_ENDPOINT || undefined,
>       commitment: "confirmed",
>     },
>   ),
>   anchor.Wallet.local(),
> );
> anchor.setProvider(provider);
>
> const providerEphemeralRollup = new anchor.AnchorProvider(
>   new anchor.web3.Connection(
>     process.env.EPHEMERAL_PROVIDER_ENDPOINT ||
>       "https://devnet-as.magicblock.app/",
>     {
>       wsEndpoint:
>         process.env.EPHEMERAL_WS_ENDPOINT || "wss://devnet-as.magicblock.app/",
>       commitment: "confirmed",
>     },
>   ),
>   anchor.Wallet.local(),
> );
>
> // Set program and PDA
> const program = anchor.workspace.PublicCounter as Program<PublicCounter>;
> const [counterPDA] = anchor.web3.PublicKey.findProgramAddressSync(
>   [Buffer.from(COUNTER_SEED)],
>   program.programId,
> );
>
> // Initialize counter on base layer
> let initTx = await program.methods
>   .initialize()
>   .accounts({
>     user: provider.wallet.publicKey,
>   })
>   .transaction();
> const initTxHash = await provider.sendAndConfirm(initTx, [
>   provider.wallet.payer,
> ]);
>
> // Increment counter on base layer
> let incBaseTx = await program.methods
>   .increment()
>   .accounts({
>     counter: counterPDA,
>   })
>   .transaction();
> const incBaseTxHash = await provider.sendAndConfirm(incBaseTx, [
>   provider.wallet.payer,
> ]);
>
> // Delegate counter to ER
> // Pin a specific validator by passing it in remaining_accounts
> const ER_VALIDATOR = new anchor.web3.PublicKey(
>   "MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57", // Asia ER validator
> );
> let delTx = await program.methods
>   .delegate()
>   .accounts({
>     payer: provider.wallet.publicKey,
>     pda: counterPDA,
>   })
>   .remainingAccounts([
>     { pubkey: ER_VALIDATOR, isSigner: false, isWritable: false },
>   ])
>   .transaction();
> const delTxHash = await provider.sendAndConfirm(delTx, [
>   provider.wallet.payer,
> ]);
>
> // Increment counter in real time on ER
> let incErTx = await program.methods
>   .increment()
>   .accounts({
>     counter: counterPDA,
>   })
>   .transaction();
> incErTx.feePayer = providerEphemeralRollup.wallet.publicKey;
> incErTx.recentBlockhash = (
>   await providerEphemeralRollup.connection.getLatestBlockhash()
> ).blockhash;
> incErTx = await providerEphemeralRollup.wallet.signTransaction(incErTx);
> const incErTxHash = await providerEphemeralRollup.sendAndConfirm(incErTx);
>
> // Commit and undelegate counter from ER back to base layer
> let undelTx = await program.methods
>   .undelegate()
>   .accounts({
>     payer: providerEphemeralRollup.wallet.publicKey,
>   })
>   .transaction();
> undelTx.feePayer = providerEphemeralRollup.wallet.publicKey;
> undelTx.recentBlockhash = (
>   await providerEphemeralRollup.connection.getLatestBlockhash()
> ).blockhash;
> undelTx = await providerEphemeralRollup.wallet.signTransaction(undelTx);
> const undelTxHash = await providerEphemeralRollup.sendAndConfirm(undelTx);
> ```
>
> To make it easier to integrate via the frontend, we created the [Magic Router](/pages/ephemeral-rollups-ers/introduction/magic-router). You send transactions directly to the magic router, and we can determine for you whether it should be routed to the [Ephemeral Rollup](/pages/ephemeral-rollups-ers/introduction/ephemeral-rollup) or base layer.
>
> > [!NOTE]
> >
> > These public RPC endpoints are currently free and supported for development:
> > \
> > &#x20;Magic Router Devnet: [https://devnet-router.magicblock.app](https://devnet-router.magicblock.app) \
> > &#xA;Solana Devnet: [https://api.devnet.solana.com](https://api.devnet.solana.com) \
> > &#xA;ER Devnet (Asia): [https://devnet-as.magicblock.app](https://devnet-as.magicblock.app) \
> > &#xA;ER Devnet (EU): [https://devnet-eu.magicblock.app](https://devnet-eu.magicblock.app) \
> > &#xA;ER Devnet (US, validator 2.0): [https://devnet-us.magicblock.app](https://devnet-us.magicblock.app) \
> > &#xA;TEE Devnet: [https://devnet-tee.magicblock.app/](https://devnet-tee.magicblock.app/) \
> > &#xA;Find out more details&#x20;
> > [here](/pages/ephemeral-rollups-ers/how-to-guide/local-development)
> > .
>
> [⬆️ Back to Top](#code-snippets)

***

### Advanced Code Snippets

> [tab] Resize PDA
>
> When resizing a delegated PDA:
>
> * PDA must have enough lamports to remain rent-exempt for the new account size.
> * If additional lamports are needed, the **payer account must be delegated** to provide the difference.
> * PDA must be owned by the program, and the transaction must include any signer(s) required for transferring lamports.
> * Use `system_instruction::allocate`
>
> ```rust theme={null}
> #[account]
> pub struct Counter {
>     pub count: u64,
>     pub extra_data: Vec<u8>,
> }
>
> #[derive(Accounts)]
> pub struct ResizeCounter<'info> {
>     #[account(mut)]
>     pub counter: Account<'info, Counter>,
>     #[account(mut)]
>     pub payer: Signer<'info>,
>     pub system_program: Program<'info, System>,
> }
>
>     // Resize the counter (e.g., to store more extra_data)
>     pub fn resize_counter(ctx: Context<ResizeCounter>, new_size: usize) -> Result<()> {
>         let account_to_resize = &mut ctx.accounts.counter.to_account_info();
>         let payer = &mut ctx.accounts.payer.to_account_info();
>
>         // Calculate rent-exemption for the new size
>         let rent = Rent::get()?;
>         let min_balance = rent.minimum_balance(new_size);
>
>         // Top up lamports if needed
>         let current_lamports = **account_to_resize.lamports.borrow();
>         if current_lamports < min_balance {
>             let to_transfer = min_balance - current_lamports;
>             **payer.try_borrow_mut_lamports()? -= to_transfer;
>             **account_to_resize.try_borrow_mut_lamports()? += to_transfer;
>         }
>
>         // Resize account
>         account_to_resize.resize(new_size)?;
>
>         Ok(())
>     }
> ```
>
> [⬆️ Back to Top](#advanced-code-snippets)

> [tab] Magic Router
>
> Initialize connection with Magic Router before you send transactions dynamically.
>
> > [!NOTE]
> >
> > These public RPC endpoints are currently free and supported for development:
> > \
> > &#x20;Magic Router Devnet: [https://devnet-router.magicblock.app](https://devnet-router.magicblock.app) \\
>
> Choose your preferred SDK to initialize, send and confirm transactions:
>
> * `ephemeral-rollups-kit` for `@solana/kit`
> * `ephemeral-rollups-sdk` for `@solana/web.js`
>
> ```typescript Kit theme={null}
> import { Connection } from "@magicblock-labs/ephemeral-rollups-kit";
>
> // Initialize connection
> const connection = await Connection.create(
>   "https://devnet-router.magicblock.app",
>   "wss://devnet-router.magicblock.app"
> );
>
> // ... create transaction
>
> // Send and confirm transaction
> const txHash = await connection.sendAndConfirmTransaction(
>   transactionMessage,
>   [userKeypair],
>   { commitment: "confirmed", skipPreflight: true }
> );
> ```
>
> ```typescript Web3.js theme={null}
> import { sendAndConfirmTransaction } from "@solana/web3.js";
> import { ConnectionMagicRouter } from "@magicblock-labs/ephemeral-rollups-sdk";
>
> // Initialize connection
> const connection = new ConnectionMagicRouter(
>   "https://devnet-router.magicblock.app/",
>   { wsEndpoint: "wss://devnet-router.magicblock.app/" }
> );
>
> // ... create transaction
>
> // Send and confirm transaction
> const txHash = await sendAndConfirmTransaction(connection, tx, [payer], {
>   skipPreflight: true,
>   commitment: "confirmed",
> });
> ```
>
> [Learn more about Magic Router](/pages/ephemeral-rollups-ers/introduction/magic-router)
>
> [⬆️ Back to Top](#advanced-code-snippets)

> [tab] Magic Action
>
> ### Quick Access
>
> > [card] [Magic Actions Example](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/magic-actions/anchor)
> >
> > Explore reference implementation on GitHub
>
> Attach one or more instructions that run automatically on the Solana base layer immediately after an Ephemeral Rollup
> (ER) commit.
> [Learn more about Magic Action](/pages/ephemeral-rollups-ers/magic-actions/overview)
>
> ### 1) Create action instruction
>
> The instruction `update_leaderboard` runs on the base layer immediately after the commit lands. The `#[action]` attribute on its accounts context marks it as callable from a post-commit action.
>
> > [!WARNING]
> >
> > `#[action]` makes the instruction **callable from** a post-commit action — it
> > does not make it callable **only** that way. The handler is an ordinary
> > base-layer instruction, so anyone can invoke it directly with a wallet.
> > Address, `seeds`, and `owner` constraints only pin *which* accounts are passed;
> > they do not authenticate *who* called it. Any handler that moves value or
> > changes authoritative state must verify the injected `escrow` signer — see
> > [Authenticate the caller](/pages/ephemeral-rollups-ers/magic-actions/troubleshooting#security-authenticate-the-caller).
>
> ```rust theme={null}
> // program instruction
> pub fn update_leaderboard(ctx: Context<UpdateLeaderboard>) -> Result<()> {
>     let leaderboard = &mut ctx.accounts.leaderboard;
>     let counter_info = &mut ctx.accounts.counter.to_account_info();
>     let mut data: &[u8] = &counter_info.try_borrow_data()?;
>     let counter = Counter::try_deserialize(&mut data)?;
>
>     if counter.count > leaderboard.high_score {
>         leaderboard.high_score = counter.count;
>     }
>
>     msg!(
>         "Leaderboard updated! High score: {}",
>         leaderboard.high_score
>     );
>     Ok(())
> }
>
> // instruction context
> #[action]
> #[derive(Accounts)]
> pub struct UpdateLeaderboard<'info> {
>     #[account(mut, seeds = [LEADERBOARD_SEED], bump)]
>     pub leaderboard: Account<'info, Leaderboard>,
>     /// CHECK: PDA owner depends on: 1) Delegated: Delegation Program; 2) Undelegated: Your program ID
>     pub counter: UncheckedAccount<'info>,
>     /// CHECK: the program that scheduled the action. The delegation program passes it
>     /// before the `escrow_auth` and `escrow` accounts that `#[action]` adds.
>     #[account(address = crate::ID)]
>     pub source_program: UncheckedAccount<'info>,
> }
> ```
>
> ### 2) Build the commit instruction with the action
>
> The commit instruction `commit_and_update_leaderboard` runs on the ER. It uses `MagicIntentBundleBuilder` to schedule both the commit and the post-commit action onto `magic_context` — both are applied together when the ER transaction is sealed back to the base layer.
>
> ```rust theme={null}
> // commit action instruction on ER
> pub fn commit_and_update_leaderboard(ctx: Context<CommitAndUpdateLeaderboard>) -> Result<()> {
>     // Build the post-commit action that updates the leaderboard on base layer
>     let instruction_data =
>         anchor_lang::InstructionData::data(&crate::instruction::UpdateLeaderboard {});
>     let action_args = ActionArgs::new(instruction_data);
>     let action_accounts = vec![
>         ShortAccountMeta {
>             pubkey: ctx.accounts.leaderboard.key(),
>             is_writable: true,
>         },
>         ShortAccountMeta {
>             pubkey: ctx.accounts.counter.key(),
>             is_writable: false,
>         },
>     ];
>     let action = CallHandler {
>         destination_program: crate::ID,
>         accounts: action_accounts,
>         args: action_args,
>         // Signer that pays transaction fees for the action from its escrow PDA
>         escrow_authority: ctx.accounts.payer.to_account_info(),
>         compute_units: 200_000,
>     };
>
>     // Schedule commit + post-commit action on magic_context
>     MagicIntentBundleBuilder::new(
>         ctx.accounts.payer.to_account_info(),
>         ctx.accounts.magic_context.to_account_info(),
>         ctx.accounts.magic_program.to_account_info(),
>     )
>     .commit(&[ctx.accounts.counter.to_account_info()])
>     .add_post_commit_actions([action])
>     .build_and_invoke()?;
>
>     Ok(())
> }
>
> // commit action context on ER
> #[commit]
> #[derive(Accounts)]
> pub struct CommitAndUpdateLeaderboard<'info> {
>     #[account(mut)]
>     pub payer: Signer<'info>,
>
>     #[account(mut, seeds = [COUNTER_SEED], bump)]
>     pub counter: Account<'info, Counter>,
>
>     /// CHECK: Leaderboard PDA - not mut here, writable set in handler
>     #[account(seeds = [LEADERBOARD_SEED], bump)]
>     pub leaderboard: UncheckedAccount<'info>,
>
>     /// CHECK: Your program ID
>     pub program_id: AccountInfo<'info>,
> }
> ```
>
> ### Execute multiple actions
>
> You can commit multiple accounts and chain several actions in one call. Actions execute sequentially in the order they're passed to `add_post_commit_actions`.
>
> ```rust theme={null}
> // Chain several actions — they execute sequentially on base layer after the commit lands.
> MagicIntentBundleBuilder::new(
>     ctx.accounts.payer.to_account_info(),
>     ctx.accounts.magic_context.to_account_info(),
>     ctx.accounts.magic_program.to_account_info(),
> )
> .commit(&[
>     ctx.accounts.counter.to_account_info(),
>     // ... additional committed accounts
> ])
> .add_post_commit_actions([action_1, action_2, action_3])
> .build_and_invoke()?;
> ```
>
> ### Undelegate with actions
>
> Actions can also be chained onto an undelegation: the counter commits and undelegates, then the actions run on the base layer. Actions are not atomic with the commit. If an action fails, the validator drops the actions and retries the commit and undelegation without them.
>
> ```rust theme={null}
> // Commit, undelegate, then run the actions on base layer. If an action fails, the commit and
> // undelegation are retried without the actions.
> MagicIntentBundleBuilder::new(
>     ctx.accounts.payer.to_account_info(),
>     ctx.accounts.magic_context.to_account_info(),
>     ctx.accounts.magic_program.to_account_info(),
> )
> .commit_and_undelegate(&[ctx.accounts.counter.to_account_info()])
> .add_post_commit_actions([action])
> .build_and_invoke()?;
> ```
>
> [⬆️ Back to Top](#advanced-code-snippets)

> [tab] Top-up delegated account
>
> Top up a delegated account's lamports on the ER side. The transaction is submitted on the **base layer** and uses the Ephemeral SPL Token program to shuttle lamports to the destination's delegated balance via a single-use lamports PDA.
>
> Common use case: keeping a delegated fee payer funded for a long session. Without a fee payer, an account stops after 10 commits. With a fee payer and `magic_fee_vault`, it can keep committing, and the payer starts paying live commit fees on commit 26.
>
> Notes:
>
> * Generate a fresh 32-byte salt per top-up via `crypto.getRandomValues` — re-using a salt collides with an existing PDA.
> * Submit to the base-layer RPC, not the ER.
> * The destination must already be delegated.
>
> ```typescript theme={null}
> import {
>   Connection,
>   Keypair,
>   PublicKey,
>   Transaction,
>   sendAndConfirmTransaction,
> } from "@solana/web3.js";
> import {
>   lamportsDelegatedTransferIx,
>   deriveLamportsPda,
> } from "@magicblock-labs/ephemeral-rollups-sdk";
>
> /**
>  * Top up a delegated account with lamports.
>  *
>  * The transaction is submitted on the BASE LAYER. The Ephemeral SPL Token
>  * program creates a single-use lamports PDA, funds it from the payer, and
>  * delegates it so the ER credits the destination's delegated balance.
>  */
> async function topUpDelegatedAccount(
>   connection: Connection,         // base-layer connection
>   payer: Keypair,
>   destination: PublicKey,         // delegated account to top up
>   amountLamports: bigint,
> ) {
>   // Generate a fresh 32-byte salt per top-up.
>   // Re-using a salt collides with an existing lamports PDA and the call fails.
>   const salt = crypto.getRandomValues(new Uint8Array(32));
>
>   const [lamportsPda] = deriveLamportsPda(payer.publicKey, destination, salt);
>
>   const ix = await lamportsDelegatedTransferIx(
>     payer.publicKey,
>     destination,
>     amountLamports,
>     salt,
>   );
>
>   const tx = new Transaction().add(ix);
>   tx.feePayer = payer.publicKey;
>
>   // CRITICAL: send to the base-layer RPC, not the ER.
>   const sig = await sendAndConfirmTransaction(connection, tx, [payer], {
>     commitment: "confirmed",
>     skipPreflight: true,
>   });
>
>   return { sig, lamportsPda };
> }
> ```
>
> [⬆️ Back to Top](#advanced-code-snippets)

> [tab] On-Curve Delegation
>
> ### Quick Access
>
> > [card] [GitHub](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/oncurve-delegation/client)
> >
> > On-Curve Delegation
>
> Required signers for delegating an on-curve account:
>
> 1. On-curve account to be delegated
> 2. Fee payer
>
> Required instructions for delegating on-curve accounts:
>
> 1. Assign System Account to Delegation Program
> 2. Delegate to Delegation Program
>
> ```typescript Kit theme={null}
> // Create assign instruction
> // The on-curve account must sign this instruction to change its owner
> const accountSigner = await cryptoKeyPairToTransactionSigner(userKeypair);
> const delegationProgramAddress = address(DELEGATION_PROGRAM_ID.toString());
> const assignInstruction = getAssignInstruction({
>   account: accountSigner,
>   programAddress: delegationProgramAddress,
> });
>
> // Create delegate instruction
> const delegateInstruction = await createDelegateInstruction({
>   payer: feePayerAddress,
>   delegatedAccount: userAddress,
>   ownerProgram: ownerProgramAddress,
>   validator: validatorAddress,
> });
>
> // Prepare transaction
> const transactionMessage = pipe(
>   createTransactionMessage({ version: 0 }),
>   (tx) => setTransactionMessageFeePayer(feePayerAddress, tx),
>   (tx) =>
>     appendTransactionMessageInstructions(
>       [assignInstruction, delegateInstruction],
>       tx
>     )
> );
> // Send and confirm transaction (fee payer need to sign, on-curve account cannot be signer since delegated)
> const txHash = await connection.sendAndConfirmTransaction(
>   transactionMessage,
>   [userKeypair, feePayerKeypair],
>   { commitment: "confirmed", skipPreflight: true }
> );
> ```
>
> ```typescript Web3.js theme={null}
> // Create assign instruction
> const assignInstruction = SystemProgram.assign({
>   accountPubkey: userPubkey,
>   programId: DELEGATION_PROGRAM_ID,
> });
>
> // Create delegate instruction
> const delegateInstruction = createDelegateInstruction({
>   payer: feePayerKeypair.publicKey,
>   delegatedAccount: userPubkey,
>   ownerProgram: ownerProgram,
>   validator: validator,
> });
>
> // Create and send transaction (fee payer need to sign, on-curve account cannot be signer since delegated)
> const tx = new Transaction().add(assignInstruction, delegateInstruction);
> tx.feePayer = feePayerKeypair.publicKey;
> const txSignature = await sendAndConfirmTransaction(
>   connectionBaseLayer,
>   tx,
>   [userKeypair, feePayerKeypair],
>   {
>     skipPreflight: true,
>   }
> );
> ```
>
> Direct commit and undelegate through Magic Program only.
>
> ```typescript Kit theme={null}
> // Create commit and undelegate instruction
> const commitAndUndelegateInstruction = createCommitAndUndelegateInstruction(
>   userAddress,
>   [userAddress]
> );
>
> // Prepare transaction
> const transactionMessage = pipe(
>   createTransactionMessage({ version: 0 }),
>   (tx) => setTransactionMessageFeePayer(feePayerAddress, tx),
>   (tx) =>
>     appendTransactionMessageInstructions([commitAndUndelegateInstruction], tx)
> );
>
> // Send and confirm transaction on ephemeral connection
> const txHash = await ephemeralConnection.sendAndConfirmTransaction(
>   transactionMessage,
>   [userKeypair, feePayerKeypair],
>   { commitment: "confirmed", skipPreflight: true }
> );
> ```
>
> ```typescript Web3.js theme={null}
> // Create commit and undelegate instruction
> const commitAndUndelegateInstruction = createCommitAndUndelegateInstruction(
>   userPubkey,
>   [userPubkey]
> );
>
> // Send and confirm transaction on ephemeral connection
> const tx = new Transaction().add(commitAndUndelegateInstruction);
> tx.feePayer = feePayerKeypair.publicKey;
> const txSignature = await sendAndConfirmTransaction(
>   ephemeralConnection,
>   tx,
>   [userKeypair, feePayerKeypair],
>   {
>     skipPreflight: true,
>   }
> );
> ```
>
> [⬆️ Back to Top](#advanced-code-snippets)

> [tab] Delegation Actions
>
> ### Quick Access
>
> > [card] [Delegation Actions Example](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/delegation-actions/anchor)
> >
> > Explore reference implementation on GitHub
>
> Attach instructions to a delegation that the ER validator runs automatically
> inside the rollup, right after the account is delegated — no extra transaction:
>
> * Build the action(s) as standard `Instruction`s and convert them to the
>   compact payload with `.cleartext()` (public) — encrypted actions are built
>   off-chain by a client holding the validator key.
> * The base-layer delegation program stores the payload in the delegation
>   record; the ER validator executes it once the account lands in the rollup.
> * CPI with `delegate_account_with_actions` instead of the plain `delegate_pda`
>   helper; the `#[delegate]` macro still provides the buffer/record/metadata
>   accounts.
>
> ```rust theme={null}
> /// Reuse the same accounts context as a normal delegation
> #[delegate]
> #[derive(Accounts)]
> pub struct DelegateInput<'info> {
>     pub payer: Signer<'info>,
>     /// CHECK: The pda to delegate
>     #[account(mut, del)]
>     pub pda: AccountInfo<'info>,
> }
> ```
>
> ```rust theme={null}
> use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
> use anchor_lang::InstructionData;
> use ephemeral_rollups_sdk::cpi::{
>     delegate_account_with_actions, DelegateAccounts, DelegateConfig,
> };
> use ephemeral_rollups_sdk::dlp_api::compact::ClearText;
>
> /// Delegate the account AND attach a post-delegation action. The action is stored
> /// in the delegation record on the base layer and executed automatically by the ER
> /// validator inside the rollup, right after the account is delegated — no extra
> /// transaction. Here the action is a self-CPI back into `increment`.
> pub fn delegate_with_actions(ctx: Context<DelegateInput>) -> Result<()> {
>     let counter_key = ctx.accounts.pda.key();
>
>     // The instruction the ER validator runs post-delegation, inside the rollup.
>     let increment_action = Instruction {
>         program_id: crate::ID,
>         accounts: vec![AccountMeta::new(counter_key, false)],
>         data: crate::instruction::Increment {}.data(),
>     };
>
>     // Convert to the compact, cleartext post-delegation actions payload.
>     // (Use `cleartext` for public actions; encrypted actions are built off-chain
>     // by a client that holds the validator key.)
>     let actions = vec![increment_action].cleartext();
>
>     let payer = ctx.accounts.payer.to_account_info();
>     let pda = ctx.accounts.pda.to_account_info();
>     let delegate_accounts = DelegateAccounts {
>         payer: &payer,
>         pda: &pda,
>         owner_program: &ctx.accounts.owner_program,
>         buffer: &ctx.accounts.buffer_pda,
>         delegation_record: &ctx.accounts.delegation_record_pda,
>         delegation_metadata: &ctx.accounts.delegation_metadata_pda,
>         delegation_program: &ctx.accounts.delegation_program,
>         system_program: &ctx.accounts.system_program,
>     };
>
>     delegate_account_with_actions(
>         delegate_accounts,
>         &[COUNTER_SEED],
>         DelegateConfig {
>             // Optionally set a specific validator from the first remaining account
>             validator: ctx.remaining_accounts.first().map(|acc| acc.key()),
>             ..Default::default()
>         },
>         actions,
>         // No extra signers are required by the increment action.
>         &[],
>     )?;
>     Ok(())
> }
> ```
>
> [⬆️ Back to Top](#advanced-code-snippets)

***

### Quick Access

Learn more about private ER, Rust Native implementation, and local development:

> [card] [Private Ephemeral Rollups (PER)](/pages/private-ephemeral-rollups-pers/how-to-guide/quickstart)
>
> Quickstart

> [card] [Rust Native](/pages/ephemeral-rollups-ers/how-to-guide/rust-program)
>
> Quickstart

> [card] [Guide](/pages/ephemeral-rollups-ers/how-to-guide/local-development)
>
> Local Development

***

## Solana Explorer

Get insights about your transactions and accounts on Solana:

> [card] [Solana Explorer](https://explorer.solana.com/)
>
> Official Solana Explorer

> [card] [Solscan](https://solscan.io/)
>
> Explore Solana Blockchain

## Solana RPC Providers

Send transactions and requests through existing RPC providers:

> [card] [Solana](https://solana.com/docs/references/clusters#on-a-high-level)
>
> Free Public Nodes

> [card] [Helius](https://www.helius.dev/solana-rpc-nodes)
>
> Free Shared Nodes

> [card] [Triton](https://triton.one/solana)
>
> Dedicated High-Performance Nodes

## Solana Validator Dashboard

Find real-time updates on Solana's validator infrastructure:

> [card] [Solana Beach](https://solanabeach.io/)
>
> Get Validator Insights

> [card] [Validators App](https://www.validators.app/)
>
> Discover Validator Metrics

## Server Status

Subscribe to Solana's and MagicBlock's server status:

> [card] [Solana Status](https://status.solana.com/)
>
> Subscribe to Solana Server Updates

> [card] [MagicBlock Status](/pages/overview/additional-information/system-status)
>
> Subscribe to MagicBlock Server Status

***

## MagicBlock Products

> [card] [Ephemeral Rollup (ER)](/pages/ephemeral-rollups-ers/how-to-guide/quickstart)
>
> Execute real-time, zero-fee transactions securely on Solana.

> [card] [Private Ephemeral Rollup (PER)](/pages/private-ephemeral-rollups-pers/how-to-guide/quickstart)
>
> Protect sensitive data with compliance — built on top of Ephemeral Rollups.

> [card] [Ephemeral SPL Token](/pages/ephemeral-spl-token/overview)
>
> Move SPL tokens at rollup speed — public or private transfers, swaps, and private payments for trading and DeFi apps.

> [card] [Prediction Markets & Trading](/pages/solutions/prediction-markets)
>
> Combine real-time execution, session keys, token custody, price feeds, automation, and settlement.

> [card] [Solana VRF](/pages/verifiable-randomness-functions-vrfs/introduction/solana-vrf)
>
> Add provably fair onchain randomness to games, raffles, and real-time apps.

> [card] [Pricing Oracle](/pages/tools/oracle/introduction)
>
> Access low-latency onchain price feeds for trading and DeFi.

***


# Ephemeral Rollups with Native Rust
