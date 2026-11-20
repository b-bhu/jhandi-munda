

# VRF Quickstart
Source: https://docs.magicblock.gg/pages/verifiable-randomness-functions-vrfs/how-to-guide/quickstart

Learn how to request and consume Solana VRF randomness onchain using the MagicBlock VRF SDK.

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

Check out basic VRF examples:

> [card] [GitHub](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/roll-dice/anchor)
>
> Repo for roll dice example

> [card] [VRF dApp](https://roll-dice.magicblock.app/)
>
> Roll a dice onchain

> [card] [Delegated VRF dApp](https://roll-dice.magicblock.app/delegated)
>
> Roll a dice within 100 ms onchain

***

> [!NOTE]
>
> Need the product overview first? Start with the [Solana VRF](/pages/verifiable-randomness-functions-vrfs/introduction/solana-vrf) landing page, then follow this quickstart.

## Step-By-Step Guide

Any Solana program can request and consume verifiable randomness onchain within seconds using the MagicBlock VRF SDK. By the end of this guide, you'll have a working example that rolls a dice using verifiable randomness.

**1. [Write your program](#1-write-program)**

Write your Solana program as you normally.

**2. [Add request and consume randomness instructions.](#code-snippets)**

Add CPI hooks that request and consume randomness via callback from a
verified oracle.

**3. [Deploy your program on Solana](#3-deploy)**

Deploy your Solana program using Anchor CLI.

**4. [Execute transactions for onchain randomness.](#4-test)**

Send transactions to generate and consume randomness onchain.

***

## Roll Dice Example

![Roll Dice GIF](https://mintcdn.com/magicblock-42/iteauKFqxDKE2Vln/images/gifs/vrf-roll-dice-420w.gif?s=9250f6e10d1a713f3c3d02f990f3a1b4)

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
> A simple roll dice program where player initialize state account to store, request and consume randomness:
>
> ```rust theme={null}
> pub const PLAYER: &[u8] = b"playerd";
>
> #[program]
> pub mod random_dice {
>     use super::*;
>
>     pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
>         msg!(
>             "Initializing player account: {:?}",
>             ctx.accounts.player.key()
>         );
>         Ok(())
>     }
>
>     // ... Additional instructions will be added here
> }
>
> /// Context for initializing player
> #[derive(Accounts)]
> pub struct Initialize<'info> {
>     #[account(mut)]
>     pub payer: Signer<'info>,
>     #[account(init_if_needed, payer = payer, space = 8 + 1, seeds = [PLAYER, payer.key().to_bytes().as_slice()], bump)]
>     pub player: Account<'info, Player>,
>     pub system_program: Program<'info, System>,
> }
>
> /// Player struct
> #[account]
> pub struct Player {
>     pub last_result: u8,
> }
> ```
>
> [⬆️ Back to Top](#code-snippets)

> [tab] 2. Request & Consume Randomness
>
> 1. Add `ephemeral-rollups-sdk` 0.18 with the `anchor` and `vrf` features to your program, and enable Anchor's `init-if-needed` feature (the `initialize` instruction uses `init_if_needed`):
>
> ```bash theme={null}
> cargo add ephemeral-rollups-sdk@0.18 --features anchor,vrf
> cargo add anchor-lang --features init-if-needed
> ```
>
> > [!WARNING]
> >
> > Pin the version. Without `@0.18`, `cargo add` installs 0.20, the Validator v2 line, which needs anchor-lang 1.2.2 or later and only works against v2 endpoints (devnet-us today). Its `#[vrf]` macro also fails to compile on 0.20.0 (`cannot find slot_hashes in sysvar`). The public Ephemeral Rollups run validator 1.1.1, so use 0.18.x. `create_request_scoped_randomness_ix` from SDK 0.16 was removed in 0.17.
>
> Import the `vrf` and `vrf_callback` macros, `program_identity_pubkey`, `create_request_randomness_ix_const`, `RequestRandomnessParams`, and `SerializableAccountMeta`:
>
> ```rust theme={null}
> use ephemeral_rollups_sdk::anchor::{vrf, vrf_callback};
> use ephemeral_rollups_sdk::vrf::consts::program_identity_pubkey;
> use ephemeral_rollups_sdk::vrf::instructions::{create_request_randomness_ix_const, RequestRandomnessParams};
> use ephemeral_rollups_sdk::vrf::types::SerializableAccountMeta;
> ```
>
> 2. Add instructions `roll_dice` to request randomness and `callback_roll_dice` to consume randomness, along with its context:
>
> ```rust theme={null}
> use ephemeral_rollups_sdk::{
>     anchor::{vrf, vrf_callback},
>     vrf::{
>         self,
>         consts::program_identity_pubkey,
>         instructions::{create_request_randomness_ix_const, RequestRandomnessParams},
>         types::SerializableAccountMeta,
>     },
> };
>
> /// The identity PDA this program signs randomness requests with, derived at compile time.
> const PROGRAM_IDENTITY: Pubkey = program_identity_pubkey(&crate::ID);
>
> #[program]
> pub mod random_dice {
>     use super::*;
>
>     // ... `initialize` instruction
>
>     // Request Randomness
>     pub fn roll_dice(ctx: Context<DoRollDiceCtx>, client_seed: u8) -> Result<()> {
>         msg!("Requesting randomness...");
>         let params = RequestRandomnessParams {
>             payer: ctx.accounts.payer.key(),
>             oracle_queue: ctx.accounts.oracle_queue.key(),
>             callback_program_id: ID,
>             callback_discriminator: instruction::CallbackRollDice::DISCRIMINATOR.to_vec(),
>             caller_seed: [client_seed; 32],
>             // Specify any account that is required by the callback
>             accounts_metas: Some(vec![SerializableAccountMeta {
>                 pubkey: ctx.accounts.player.key(),
>                 is_signer: false,
>                 is_writable: true,
>             }]),
>             callback_args: Some(vec![client_seed]),
>             ..Default::default()
>         };
>         let ix = create_request_randomness_ix_const(params, PROGRAM_IDENTITY);
>         ctx.accounts
>             .invoke_signed_vrf(&ctx.accounts.payer.to_account_info(), &ix)?;
>         Ok(())
>     }
>
>     // Consume Randomness
>     pub fn callback_roll_dice(
>         ctx: Context<CallbackRollDiceCtx>,
>         randomness: [u8; 32],
>         client_seed: u8,
>     ) -> Result<()> {
>         msg!("client_seed={}", client_seed);
>         let rnd_u8 = vrf::rnd::random_u8_with_range(&randomness, 1, 6);
>         msg!("Consuming random number: {:?}", rnd_u8);
>         let player = &mut ctx.accounts.player;
>         player.last_result = rnd_u8; // Update the player's last result
>         Ok(())
>     }
> }
>
> #[vrf]
> #[derive(Accounts)]
> pub struct DoRollDiceCtx<'info> {
>     #[account(mut)]
>     pub payer: Signer<'info>,
>     #[account(seeds = [PLAYER, payer.key().to_bytes().as_slice()], bump)]
>     pub player: Account<'info, Player>,
>     /// CHECK: The oracle queue
>     #[account(
>         mut,
>         constraint =
>             oracle_queue.key() == vrf::consts::DEFAULT_QUEUE ||                // Devnet
>             oracle_queue.key() == vrf::consts::DEFAULT_TEST_QUEUE ||           // Local
>             oracle_queue.key() == vrf::consts::DEFAULT_EPHEMERAL_QUEUE ||      // ER Devnet
>             oracle_queue.key() == vrf::consts::DEFAULT_EPHEMERAL_TEST_QUEUE    // ER Local
>     )]
>     pub oracle_queue: UncheckedAccount<'info>,
> }
>
> // `#[vrf_callback]` adds a `vrf_program_identity` signer that must be this program's
> // scoped VRF identity, so only the VRF program can invoke the callback. Without it,
> // any caller can spoof the callback. Don't declare your own `vrf_program_identity`
> // field: the macro then skips its check.
> #[vrf_callback]
> #[derive(Accounts)]
> pub struct CallbackRollDiceCtx<'info> {
>     #[account(mut)]
>     pub player: Account<'info, Player>,
> }
>
> // ... Other context and account struct.
> ```
>
> > [!NOTE]
> >
> > **VRF SDK constants** (`ephemeral_rollups_sdk::vrf::consts`) — reference these instead of hardcoding addresses, both in your program and in client/test code:
> >
> > | Constant | Purpose | Address |
> > | - | - | - |
> > | `VRF_PROGRAM_ID` | VRF program | `Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz` |
> > | `VRF_PROGRAM_IDENTITY` | Legacy global callback signer (deprecated; don't check it in new callbacks) | `9irBy75QS2BN81FUgXuHcjqceJJRuc9oDkAe8TKVvvAw` |
> > | `DEFAULT_QUEUE` | Base-layer queue (mainnet/devnet) | `Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh` |
> > | `DEFAULT_EPHEMERAL_QUEUE` | ER queue (mainnet/devnet) | `5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc` |
> > | `DEFAULT_TEST_QUEUE` | Base-layer queue (localnet) | `GKE6d7iv8kCBrsxr78W3xVdjGLLLJnxsGiuzrsZCGEvb` |
> > | `DEFAULT_EPHEMERAL_TEST_QUEUE` | ER queue (localnet) | `Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT` |
> >
> > Pass the queue that matches where your transaction runs as `oracle_queue`. Mainnet and Devnet share the same queue addresses; localnet uses the test queues.
> >
> > Requests made through `#[vrf]` are scoped: the VRF program signs your callback with `PDA(["identity", your_program_id], VRF_PROGRAM_ID)`, not with `VRF_PROGRAM_IDENTITY`. `#[vrf_callback]` checks that scoped signer for you.
>
> > `Request Randomness` is the process of generating a random `hashId` with the relevant callback instruction for the verified oracles to be triggered.
>
> > `Consume Randomness` is the process of using the verifiable randomness by your program which is provided and triggered through verified oracle.
>
> [⬆️ Back to Top](#code-snippets)

> [tab] 3. Deploy
>
> Now you’re program is upgraded and ready! Build and deploy to the desired
> cluster:
>
> ```bash theme={null}
> anchor build && anchor deploy
> ```
>
> [⬆️ Back to Top](#code-snippets)

> [tab] 4. Test
>
> Ready to execute transactions for onchain randomness!
>
> ```bash theme={null}
> anchor test --skip-build --skip-deploy --skip-local-validator
> ```
>
> > [!NOTE]
> >
> > **VRF SDK constants** (`ephemeral_rollups_sdk::vrf::consts`) — reference these instead of hardcoding addresses, in both your program and client/test code:
> >
> > | Constant | Purpose | Address |
> > | - | - | - |
> > | `VRF_PROGRAM_ID` | VRF program | `Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz` |
> > | `VRF_PROGRAM_IDENTITY` | Legacy global callback signer (deprecated; don't check it in new callbacks) | `9irBy75QS2BN81FUgXuHcjqceJJRuc9oDkAe8TKVvvAw` |
> > | `DEFAULT_QUEUE` | Base-layer queue (mainnet/devnet) | `Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh` |
> > | `DEFAULT_EPHEMERAL_QUEUE` | ER queue (mainnet/devnet) | `5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc` |
> > | `DEFAULT_TEST_QUEUE` | Base-layer queue (localnet) | `GKE6d7iv8kCBrsxr78W3xVdjGLLLJnxsGiuzrsZCGEvb` |
> > | `DEFAULT_EPHEMERAL_TEST_QUEUE` | ER queue (localnet) | `Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT` |
> >
> > Pass the queue that matches where your transaction runs as `oracle_queue`. Mainnet and Devnet share the same queue addresses; localnet uses the test queues.
>
> Run the following test:
>
> ```typescript theme={null}
> import * as anchor from "@coral-xyz/anchor";
> import { Program, web3 } from "@coral-xyz/anchor";
> import { RandomDice } from "../target/types/random_dice";
> import { PublicKey } from "@solana/web3.js";
>
> // Devnet base-layer VRF queue (override with VRF_BASE_QUEUE for local runs)
> const DEFAULT_BASE_QUEUE = new PublicKey(
>   process.env.VRF_BASE_QUEUE || "Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh",
> );
>
> describe("roll-dice", () => {
>   anchor.setProvider(anchor.AnchorProvider.env());
>   const provider = anchor.getProvider() as anchor.AnchorProvider;
>   const program = anchor.workspace.RandomDice as Program<RandomDice>;
>
>   const playerPda = web3.PublicKey.findProgramAddressSync(
>     [Buffer.from("playerd"), provider.publicKey!.toBytes()],
>     program.programId,
>   )[0];
>
>   it("Initialized player!", async () => {
>     const tx = await program.methods
>       .initialize()
>       .rpc({ skipPreflight: true, commitment: "confirmed" });
>     console.log("Your transaction signature", tx);
>   });
>
>   it("Do Roll Dice!", async function () {
>     // The base-chain callback can take up to 10s, so raise Mocha's timeout.
>     this.timeout(20_000);
>
>     // Generate the seed BEFORE subscribing so the handler closes over it.
>     // The program logs "client_seed=N" inside callback_roll_dice — we match
>     // on that exact substring to pin the callback to our specific request.
>     const clientSeed = Math.floor(Math.random() * 256);
>     const seedTag = `client_seed=${clientSeed}`;
>
>     // Pre-arm a one-shot promise that the onLogs handler resolves with the
>     // matching signature. No polling — we just await it, racing a timeout.
>     let resolveSig!: (sig: string) => void;
>     const sigPromise = new Promise<string>((r) => {
>       resolveSig = r;
>     });
>     const callbackSubId = provider.connection.onLogs(
>       program.programId,
>       (info) => {
>         if (
>           !info.err &&
>           info.logs.some((l) => l.includes("CallbackRollDice")) &&
>           info.logs.some((l) => l.includes(seedTag))
>         ) {
>           resolveSig(info.signature);
>         }
>       },
>       "confirmed",
>     );
>
>     try {
>       const tx = await program.methods
>         .rollDice(clientSeed)
>         .accounts({ oracleQueue: DEFAULT_BASE_QUEUE })
>         .rpc({ skipPreflight: true, commitment: "confirmed" });
>       console.log("rollDice tx:", tx);
>
>       // Base-chain VRF response is slower than ER (~1-5s typical) so 10s timeout.
>       const sig = await Promise.race([
>         sigPromise,
>         new Promise<null>((r) => setTimeout(() => r(null), 10_000)),
>       ]);
>       if (!sig) throw new Error("callbackRollDice not observed within 10s.");
>       console.log("callbackRollDice tx:", sig);
>
>       const player = await program.account.player.fetch(playerPda, "processed");
>       console.log("player:", player);
>     } finally {
>       await provider.connection.removeOnLogsListener(callbackSubId);
>     }
>   });
> });
> ```
>
> [⬆️ Back to Top](#code-snippets)

***

> [!NOTE]
>
> Want to run VRF end to end on your machine? Use the [Local Development](/pages/ephemeral-rollups-ers/how-to-guide/local-development) guide for the fully local stack, the Surfpool alternative, and the local `vrf-oracle` flow.

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

## Server Status Subscriptions

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


# VRF FAQ
