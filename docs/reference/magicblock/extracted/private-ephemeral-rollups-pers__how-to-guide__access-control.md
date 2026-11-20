

# Access Control for Private Ephemeral Rollups
Source: https://docs.magicblock.gg/pages/private-ephemeral-rollups-pers/how-to-guide/access-control

Learn how to manage fine-grained access control and member permissions in Private Ephemeral Rollups.

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

***

> [card] Permission Program
>
> On-chain Permission Management (Coming soon)

> [card] [Ephemeral Rollups SDK](https://github.com/magicblock-labs/ephemeral-rollups-sdk)
>
> SDK for Private Ephemeral Rollups

***

## Overview

Private Ephemeral Rollups are [Ephemeral Rollups](/pages/ephemeral-rollups-ers/introduction/why) that enable fine-grained permission over permissioned accounts in a [Trusted Execution Environment](/pages/private-ephemeral-rollups-pers/introduction/onchain-privacy) with [compliance](/pages/private-ephemeral-rollups-pers/introduction/compliance-framework) at its heart. Each permission account maintains a list of members with specific flags that determine what actions they can perform.

### Key Concepts

* **Permission Account**: A PDA that stores access control rules for a specific account
* **Members**: Addresses granted specific permissions via flags
* **Flags**: Bitmasks that define what a member can do (authority, view logs, view balances, etc.)
* **Privacy Flag**: `is_private` decides visibility. With `is_private: false` the permissioned account is public, whatever the member list holds. With `is_private: true` only members can read it

***

## Member Flags

Member flags define fine-grained permissions for each member. Flags can be combined using bitwise OR to grant multiple permissions.

**Flag Descriptions:**

* **AUTHORITY**: Allows a member to update permission settings, add/remove other members, and update member flags.
* **TX_LOGS**: Allows a member to view transaction execution logs.
* **TX_BALANCES**: Allows a member to view account balance changes.
* **TX_MESSAGE**: Allows a member to view transaction message data.
* **ACCOUNT_SIGNATURES**: Allows a member to view account signatures

> [tab] Rust SDK
>
> ```rust theme={null}
> use ephemeral_rollups_sdk::access_control::structs::{
>     Member,
>     AUTHORITY_FLAG,
>     TX_LOGS_FLAG,
>     TX_BALANCES_FLAG,
>     TX_MESSAGE_FLAG,
>     ACCOUNT_SIGNATURES_FLAG,
> };
>
> // Set flags by combining them with bitwise OR
> let flags = AUTHORITY_FLAG | TX_LOGS_FLAG;
>
> // Create a member with combined flags
> let mut member = Member {
>     flags,
>     pubkey: user_pubkey,
> };
>
> // Check if member has a specific flag using bitwise AND
> let is_authority = (member.flags & AUTHORITY_FLAG) != 0;
> let can_see_logs = (member.flags & TX_LOGS_FLAG) != 0;
>
> // Use helper methods to set/remove flags
> member.set_flags(TX_BALANCES_FLAG); // Add a flag
> member.remove_flags(TX_LOGS_FLAG);  // Remove a flag
> ```

> [tab] Pinocchio
>
> ```rust theme={null}
> use ephemeral_rollups_pinocchio::types::{Member, MemberFlags};
> use pinocchio::Address;
>
> // Create and set flags using individual methods
> let mut flags = MemberFlags::new();
> flags.set(MemberFlags::AUTHORITY);
> flags.set(MemberFlags::TX_LOGS);
> flags.set(MemberFlags::TX_BALANCES);
>
> // Create a member with flags
> let member = Member {
>     flags,
>     pubkey: user_address,
> };
>
> // Remove a flag
> flags.remove(MemberFlags::TX_LOGS);
>
> // Create flags from individual boolean values
> let flags = MemberFlags::from_acl_flags(
>     true,  // authority
>     true,  // tx_logs
>     false, // tx_balances
>     true,  // tx_message
>     false, // account_signatures
> );
>
> // Convert flags to byte value
> let flag_byte = flags.to_acl_flag_byte();
>
> // Create flags from byte value
> let flags = MemberFlags::from_acl_flag_byte(flag_byte);
> ```

> [tab] Web3.js
>
> ```typescript theme={null}
> import { PublicKey } from "@solana/web3.js";
> import {
>   AUTHORITY_FLAG,
>   TX_LOGS_FLAG,
>   TX_BALANCES_FLAG,
>   TX_MESSAGE_FLAG,
>   ACCOUNT_SIGNATURES_FLAG,
>   type Member,
> } from "@magicblock-labs/ephemeral-rollups-sdk";
>
> // Set flags by combining them with bitwise OR
> const flags = AUTHORITY_FLAG | TX_LOGS_FLAG;
>
> // Create a member with combined flags
> const member: Member = {
>   flags,
>   pubkey: new PublicKey(userAddress),
> };
>
> // Check if a flag is present using bitwise AND
> const isAuthority = (member.flags & AUTHORITY_FLAG) !== 0;
> const canSeeLogs = (member.flags & TX_LOGS_FLAG) !== 0;
> const canSeeBalances = (member.flags & TX_BALANCES_FLAG) !== 0;
>
> // Add a flag to existing flags
> const updatedFlags = member.flags | TX_BALANCES_FLAG;
>
> // Remove a flag from existing flags
> const removedFlags = member.flags & ~TX_LOGS_FLAG;
> ```

> [tab] Kit
>
> ```typescript theme={null}
> import {
>   AUTHORITY_FLAG,
>   TX_LOGS_FLAG,
>   TX_BALANCES_FLAG,
>   TX_MESSAGE_FLAG,
>   ACCOUNT_SIGNATURES_FLAG,
>   isAuthority,
>   canSeeTxLogs,
>   canSeeTxBalances,
>   canSeeTxMessages,
>   canSeeAccountSignatures,
>   type Member,
> } from "@magicblock-labs/ephemeral-rollups-kit";
>
> // Set flags by combining them with bitwise OR
> const flags = AUTHORITY_FLAG | TX_LOGS_FLAG | TX_BALANCES_FLAG;
>
> // Create a member with combined flags
> const member: Member = {
>   flags,
>   pubkey: userAddress,
> };
>
> // Use helper functions to check specific permissions
> const canModifyPermission = isAuthority(member, userAddress);
> const canViewLogs = canSeeTxLogs(member, userAddress);
> const canViewBalances = canSeeTxBalances(member, userAddress);
> const canViewMessages = canSeeTxMessages(member, userAddress);
> const canViewSignatures = canSeeAccountSignatures(member, userAddress);
>
> // Add a flag to existing member
> const updatedFlags = member.flags | TX_MESSAGE_FLAG;
>
> // Remove a flag from existing member
> const removedFlags = member.flags & ~TX_LOGS_FLAG;
> ```

***

## Ephemeral Permission

> [!NOTE]
>
> The access-control CPIs are behind the `access-control` feature of
> `ephemeral-rollups-sdk`. Without it, `ephemeral_rollups_sdk::access_control`
> does not exist and the snippets below fail to compile. Pin the 0.18.x line
> for the public TEE endpoints:
>
> ```toml theme={null}
> ephemeral-rollups-sdk = { version = "0.18", features = ["anchor", "access-control"] }
> ```

> [!NOTE]
>
> `EphemeralPermission` accounts live entirely on the Ephemeral Rollup and are
> paid for by the delegated PDA — no base-layer permission account to create,
> delegate, or commit-and-undelegate. Three CPI ops cover the full lifecycle:
> **Create**, **Update**, **Close** — all PDA-signed by the data account on
> the ER, via MagicBlock's Permission Program `ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1`.

**Prerequisite — delegate the data PDA.** Only the data account is delegated to
the TEE validator (on the base layer, via MagicBlock's Delegation Program
`DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh`). Once delegated, the PDA signs
all three permission ops on the ER using its program seeds and pays the
ephemeral permission rent — so it must be pre-funded at `initialize` time. See
[Quickstart](/pages/private-ephemeral-rollups-pers/how-to-guide/quickstart#2-delegate-and-create-permission)
for the end-to-end flow.

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

**1. [Create Ephemeral Permission](#1-create-ephemeral-permission)**

Initialize a new `EphemeralPermission` account on the ER with initial
members and the privacy flag. Idempotent — skip if it already exists.

**2. [Update Ephemeral Permission](#2-update-ephemeral-permission)**

Toggle the privacy flag, or add / remove / re-flag members. Updates take
effect immediately on the ER.

**3. [Close Ephemeral Permission](#3-close-ephemeral-permission)**

Close the `EphemeralPermission` account on the ER and refund the rent to
the data PDA when the permission is no longer needed.

***

## Ephemeral Permission Operations

> [tab] 1. Create Ephemeral Permission
>
> Initialize a new `EphemeralPermission` account on the ER via
> `CreateEphemeralPermissionCpi`. Payer = the delegated data PDA, which
> signs with its program seeds and covers the rent from the lamports
> pre-funded at `initialize` time.
>
> > [tab] Anchor
> >
> > ```rust theme={null}
> > use ephemeral_rollups_sdk::access_control::{
> >     instructions::CreateEphemeralPermissionCpi,
> >     structs::{EphemeralMembersArgs, Member},
> > };
> >
> > // Counter PDA pays for its own permission rent (it carries lamports onto the ER
> > // after delegation and signs as PDA via seeds).
> > let signers = [
> >     COUNTER_SEED,
> >     ctx.accounts.counter.authority.as_ref(),
> >     &[ctx.bumps.counter],
> > ];
> >
> > CreateEphemeralPermissionCpi {
> >     payer: ctx.accounts.counter.to_account_info(),                  // pays ephemeral rent
> >     permissioned_account: ctx.accounts.counter.to_account_info(),   // what the permission gates
> >     permission: ctx.accounts.permission.to_account_info(),
> >     vault: ctx.accounts.ephemeral_vault.to_account_info(),
> >     magic_program: ctx.accounts.magic_program.to_account_info(),
> >     permission_program: ctx.accounts.permission_program.to_account_info(),
> >     args: EphemeralMembersArgs {
> >         is_private: false,    // start public — flip via UpdateEphemeralPermission
> >         members: vec![],
> >     },
> > }
> > .invoke_signed(&[&signers])?;
> > ```
>
> > [tab] Rust SDK
> >
> > ```rust theme={null}
> > use ephemeral_rollups_sdk::access_control::{
> >     instructions::CreateEphemeralPermissionCpi,
> >     structs::{EphemeralMembersArgs, Member},
> > };
> >
> > // `permissioned_account` (here a counter PDA) signs as PDA via seeds; pass the
> > // same seeds you used for `find_program_address` to derive it.
> > let seeds: &[&[u8]] = &[
> >     COUNTER_SEED,
> >     permissioned_account_authority.as_ref(),
> >     &[bump],
> > ];
> >
> > CreateEphemeralPermissionCpi {
> >     payer: &counter_account_info,                // pays ephemeral rent
> >     permissioned_account: &counter_account_info, // what the permission gates
> >     permission: &permission_account_info,
> >     vault: &ephemeral_vault_account_info,
> >     magic_program: &magic_program_account_info,
> >     permission_program: &permission_program_account_info,
> >     args: EphemeralMembersArgs {
> >         is_private: false,   // start public — flip via UpdateEphemeralPermission
> >         members: vec![],
> >     },
> > }
> > .invoke_signed(&[seeds])?;
> > ```
>
> > [tab] Pinocchio
> >
> > ```rust theme={null}
> > use ephemeral_rollups_pinocchio::acl::{
> >     CreateEphemeralPermission, EphemeralMembersArgs, Member,
> > };
> > use pinocchio::cpi::{Seed, Signer};
> >
> > // Buffer size: discriminator (8) + EphemeralMembersArgs body.
> > // 64 bytes covers up to 1 member with slack for future Update calls.
> > const PERMISSION_CPI_BUF: usize = 64;
> >
> > // PDA-signed CPI — the counter PDA pays rent and authorizes the permission.
> > let bump_seed = [bump];
> > let seeds_array: [Seed; 3] = [
> >     Seed::from(b"counter"),
> >     Seed::from(authority.address().as_ref()),
> >     Seed::from(&bump_seed),
> > ];
> > let signer = Signer::from(&seeds_array);
> >
> > let members: [Member; 0] = [];   // start public; toggle via Update
> > CreateEphemeralPermission {
> >     payer: counter_account,
> >     permissioned_account: counter_account,
> >     permission,
> >     vault,
> >     magic_program,
> >     permission_program,
> >     args: EphemeralMembersArgs {
> >         is_private: false,
> >         members: &members,
> >     },
> > }
> > .invoke_signed::<PERMISSION_CPI_BUF>(&[signer])?;
> > ```
>
> > [tab] Kit
> >
> > ```typescript theme={null}
> > import {
> >   EPHEMERAL_VAULT_ID,
> >   MAGIC_PROGRAM_ID,
> >   PERMISSION_PROGRAM_ID,
> > } from "@magicblock-labs/ephemeral-rollups-kit";
> > import {
> >   AccountRole,
> >   appendTransactionMessageInstructions,
> >   createTransactionMessage,
> >   pipe,
> >   setTransactionMessageFeePayer,
> >   type Instruction,
> > } from "@solana/kit";
> >
> > // EphemeralPermissions are created on the ER by the delegated PDA (via the
> > // user-program's wrapper instruction). Submit to the ER connection, not base.
> > // `ephemeralConnection` is a `Connection` from `@magicblock-labs/ephemeral-rollups-kit`.
> > const initIx: Instruction = {
> >   programAddress: COUNTER_PROGRAM_ID,
> >   accounts: [
> >     { address: tempKeypair.address, role: AccountRole.WRITABLE_SIGNER }, // authority
> >     { address: counterPda, role: AccountRole.WRITABLE },
> >     { address: permissionPda, role: AccountRole.WRITABLE },
> >     { address: PERMISSION_PROGRAM_ID, role: AccountRole.READONLY },
> >     { address: EPHEMERAL_VAULT_ID, role: AccountRole.WRITABLE },
> >     { address: MAGIC_PROGRAM_ID, role: AccountRole.READONLY },
> >   ],
> >   // Anchor discriminator of `init_permission` (from the program IDL)
> >   data: new Uint8Array([66, 14, 153, 250, 187, 36, 179, 236]),
> > };
> >
> > const transactionMessage = pipe(
> >   createTransactionMessage({ version: 0 }),
> >   (tx) => setTransactionMessageFeePayer(tempKeypair.address, tx),
> >   (tx) => appendTransactionMessageInstructions([initIx], tx),
> > );
> > const sig = await ephemeralConnection.sendAndConfirmTransaction(
> >   transactionMessage,
> >   [tempKeypair.keyPair],
> >   { commitment: "confirmed" },
> > );
> > console.log("init_permission tx:", sig);
> > ```
>
> > [tab] Web3.js
> >
> > ```typescript theme={null}
> > import {
> >   MAGIC_PROGRAM_ID,
> >   PERMISSION_PROGRAM_ID,
> >   EPHEMERAL_VAULT_ID,
> > } from "@magicblock-labs/ephemeral-rollups-sdk";
> > import { Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
> >
> > // EphemeralPermissions are created on the ER by the delegated PDA (via the
> > // user-program's wrapper instruction). Submit to the ER connection, not base.
> > const initIx = await counterProgram.methods
> >   .initPermission()
> >   .accountsPartial({
> >     authority: tempKeypair.publicKey,
> >     counter: counterPda,
> >     permission: permissionPda,
> >     permissionProgram: PERMISSION_PROGRAM_ID,
> >     ephemeralVault: EPHEMERAL_VAULT_ID,
> >     magicProgram: MAGIC_PROGRAM_ID,
> >   })
> >   .instruction();
> >
> > const tx = new Transaction().add(initIx);
> > const sig = await sendAndConfirmTransaction(ephemeralConnection, tx, [tempKeypair]);
> > console.log("init_permission tx:", sig);
> > ```
>
> **Use Cases:**
>
> * Bootstrap access control for a newly delegated PDA on the ER
> * Start public (`is_private: false`) and make it private later via Update with `is_private: true`
>
> [⬆️ Back to Top](#ephemeral-permission)

> [tab] 2. Update Ephemeral Permission
>
> Flip the privacy flag and rewrite the member list via
> `UpdateEphemeralPermissionCpi`. Rebuild the full member list every call
> (including the authority) so the data PDA can never lock itself out.
>
> > [tab] Anchor
> >
> > ```rust theme={null}
> > use ephemeral_rollups_sdk::access_control::{
> >     instructions::UpdateEphemeralPermissionCpi,
> >     structs::{
> >         EphemeralMembersArgs, Member,
> >         TX_LOGS_FLAG, TX_MESSAGE_FLAG, TX_BALANCES_FLAG,
> >     },
> > };
> >
> > let signers = [
> >     COUNTER_SEED,
> >     ctx.accounts.counter.authority.as_ref(),
> >     &[ctx.bumps.counter],
> > ];
> >
> > // When private, only listed members can read ER state via the TEE.
> > // Empty member list + is_private=false = fully public.
> > let members = if is_private {
> >     vec![Member {
> >         flags: TX_LOGS_FLAG | TX_MESSAGE_FLAG | TX_BALANCES_FLAG,
> >         pubkey: ctx.accounts.counter.authority,
> >     }]
> > } else {
> >     vec![]
> > };
> >
> > UpdateEphemeralPermissionCpi {
> >     payer: ctx.accounts.counter.to_account_info(),
> >     permissioned_account: ctx.accounts.counter.to_account_info(),
> >     permission: ctx.accounts.permission.to_account_info(),
> >     vault: ctx.accounts.ephemeral_vault.to_account_info(),
> >     magic_program: ctx.accounts.magic_program.to_account_info(),
> >     permission_program: ctx.accounts.permission_program.to_account_info(),
> >     authority: ctx.accounts.counter.to_account_info(),
> >     authority_is_signer: false,    // PDA signs via the seeds above
> >     args: EphemeralMembersArgs { is_private, members },
> > }
> > .invoke_signed(&[&signers])?;
> > ```
>
> > [tab] Rust SDK
> >
> > ```rust theme={null}
> > use ephemeral_rollups_sdk::access_control::{
> >     instructions::UpdateEphemeralPermissionCpi,
> >     structs::{
> >         EphemeralMembersArgs, Member,
> >         TX_LOGS_FLAG, TX_MESSAGE_FLAG, TX_BALANCES_FLAG,
> >     },
> > };
> >
> > let seeds: &[&[u8]] = &[
> >     COUNTER_SEED,
> >     permissioned_account_authority.as_ref(),
> >     &[bump],
> > ];
> >
> > // When private, only listed members can read ER state via the TEE.
> > let members = if is_private {
> >     vec![Member {
> >         flags: TX_LOGS_FLAG | TX_MESSAGE_FLAG | TX_BALANCES_FLAG,
> >         pubkey: permissioned_account_authority,
> >     }]
> > } else {
> >     vec![]
> > };
> >
> > UpdateEphemeralPermissionCpi {
> >     payer: &counter_account_info,
> >     permissioned_account: &counter_account_info,
> >     permission: &permission_account_info,
> >     vault: &ephemeral_vault_account_info,
> >     magic_program: &magic_program_account_info,
> >     permission_program: &permission_program_account_info,
> >     authority: &counter_account_info,
> >     authority_is_signer: false,   // PDA signs via the seeds above
> >     args: EphemeralMembersArgs { is_private, members },
> > }
> > .invoke_signed(&[seeds])?;
> > ```
>
> > [tab] Pinocchio
> >
> > ```rust theme={null}
> > use ephemeral_rollups_pinocchio::acl::{
> >     EphemeralMembersArgs, Member, MemberFlags, UpdateEphemeralPermission,
> > };
> > use pinocchio::cpi::{Seed, Signer};
> >
> > const PERMISSION_CPI_BUF: usize = 64;
> >
> > let bump_seed = [bump];
> > let seeds_array: [Seed; 3] = [
> >     Seed::from(b"counter"),
> >     Seed::from(authority.address().as_ref()),
> >     Seed::from(&bump_seed),
> > ];
> > let signer = Signer::from(&seeds_array);
> >
> > // Read the on-chain Counter to grab `authority` — the sole "private" member.
> > let counter_authority = {
> >     let data = counter_account.try_borrow()?;
> >     Counter::load(&data)?.authority
> > };
> >
> > let single_member = [Member {
> >     flags: MemberFlags::from_acl_flag_byte(
> >         MemberFlags::TX_LOGS | MemberFlags::TX_MESSAGE | MemberFlags::TX_BALANCES,
> >     ),
> >     pubkey: counter_authority,
> > }];
> > let members: &[Member] = if is_private { &single_member } else { &[] };
> >
> > UpdateEphemeralPermission {
> >     payer: counter_account,
> >     permissioned_account: counter_account,
> >     permission,
> >     vault,
> >     magic_program,
> >     permission_program,
> >     authority: counter_account,
> >     authority_is_signer: false,   // PDA signs via the seeds above
> >     args: EphemeralMembersArgs { is_private, members },
> > }
> > .invoke_signed::<PERMISSION_CPI_BUF>(&[signer])?;
> > ```
>
> > [tab] Kit
> >
> > ```typescript theme={null}
> > import {
> >   EPHEMERAL_VAULT_ID,
> >   MAGIC_PROGRAM_ID,
> >   PERMISSION_PROGRAM_ID,
> > } from "@magicblock-labs/ephemeral-rollups-kit";
> > import {
> >   AccountRole,
> >   appendTransactionMessageInstructions,
> >   createTransactionMessage,
> >   pipe,
> >   setTransactionMessageFeePayer,
> >   type Instruction,
> > } from "@solana/kit";
> >
> > // Toggle the `is_private` flag. The program rebuilds the full member list
> > // on every call.
> > const updateIx: Instruction = {
> >   programAddress: COUNTER_PROGRAM_ID,
> >   accounts: [
> >     { address: tempKeypair.address, role: AccountRole.WRITABLE_SIGNER }, // authority
> >     { address: counterPda, role: AccountRole.WRITABLE },
> >     { address: permissionPda, role: AccountRole.WRITABLE },
> >     { address: PERMISSION_PROGRAM_ID, role: AccountRole.READONLY },
> >     { address: EPHEMERAL_VAULT_ID, role: AccountRole.WRITABLE },
> >     { address: MAGIC_PROGRAM_ID, role: AccountRole.READONLY },
> >   ],
> >   // Anchor discriminator of `set_privacy` (from the program IDL), then the `is_private` bool
> >   data: new Uint8Array([120, 77, 9, 207, 149, 140, 138, 129, isPrivate ? 1 : 0]),
> > };
> >
> > const transactionMessage = pipe(
> >   createTransactionMessage({ version: 0 }),
> >   (tx) => setTransactionMessageFeePayer(tempKeypair.address, tx),
> >   (tx) => appendTransactionMessageInstructions([updateIx], tx),
> > );
> > const sig = await ephemeralConnection.sendAndConfirmTransaction(
> >   transactionMessage,
> >   [tempKeypair.keyPair],
> >   { commitment: "confirmed" },
> > );
> > console.log("set_privacy tx:", sig);
> > ```
>
> > [tab] Web3.js
> >
> > ```typescript theme={null}
> > import {
> >   MAGIC_PROGRAM_ID,
> >   PERMISSION_PROGRAM_ID,
> >   EPHEMERAL_VAULT_ID,
> > } from "@magicblock-labs/ephemeral-rollups-sdk";
> > import { Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
> >
> > // Toggle the `is_private` flag. Idempotent — the program rebuilds the member
> > // list every call so the authority never locks itself out.
> > const updateIx = await counterProgram.methods
> >   .setPrivacy(isPrivate)
> >   .accountsPartial({
> >     authority: tempKeypair.publicKey,
> >     counter: counterPda,
> >     permission: permissionPda,
> >     permissionProgram: PERMISSION_PROGRAM_ID,
> >     ephemeralVault: EPHEMERAL_VAULT_ID,
> >     magicProgram: MAGIC_PROGRAM_ID,
> >   })
> >   .instruction();
> >
> > const tx = new Transaction().add(updateIx);
> > const sig = await sendAndConfirmTransaction(ephemeralConnection, tx, [tempKeypair]);
> > console.log("set_privacy tx:", sig);
> > ```
>
> **Use Cases:**
>
> * Toggle `is_private` on demand (e.g. private play, public reveal)
> * Add new viewers with `TX_LOGS | TX_MESSAGE | TX_BALANCES` flags
> * Revoke a member by omitting them from the next call's member list
>
> [⬆️ Back to Top](#ephemeral-permission)

> [tab] 3. Close Ephemeral Permission
>
> Close the `EphemeralPermission` account on the ER via
> `CloseEphemeralPermissionCpi`. Rent is refunded to the data PDA (the
> original payer). Optional — only call when the permission is no longer
> needed.
>
> > [tab] Anchor
> >
> > ```rust theme={null}
> > use ephemeral_rollups_sdk::access_control::instructions::CloseEphemeralPermissionCpi;
> >
> > let signers = [
> >     COUNTER_SEED,
> >     ctx.accounts.counter.authority.as_ref(),
> >     &[ctx.bumps.counter],
> > ];
> >
> > // Refunds the permission's rent to `payer` (the counter PDA).
> > CloseEphemeralPermissionCpi {
> >     payer: ctx.accounts.counter.to_account_info(),
> >     permissioned_account: ctx.accounts.counter.to_account_info(),
> >     permission: ctx.accounts.permission.to_account_info(),
> >     vault: ctx.accounts.ephemeral_vault.to_account_info(),
> >     magic_program: ctx.accounts.magic_program.to_account_info(),
> >     permission_program: ctx.accounts.permission_program.to_account_info(),
> >     authority: ctx.accounts.counter.to_account_info(),
> >     authority_is_signer: false,
> > }
> > .invoke_signed(&[&signers])?;
> > ```
>
> > [tab] Rust SDK
> >
> > ```rust theme={null}
> > use ephemeral_rollups_sdk::access_control::instructions::CloseEphemeralPermissionCpi;
> >
> > let seeds: &[&[u8]] = &[
> >     COUNTER_SEED,
> >     permissioned_account_authority.as_ref(),
> >     &[bump],
> > ];
> >
> > // Refunds the permission's rent to `payer` (the counter PDA).
> > CloseEphemeralPermissionCpi {
> >     payer: &counter_account_info,
> >     permissioned_account: &counter_account_info,
> >     permission: &permission_account_info,
> >     vault: &ephemeral_vault_account_info,
> >     magic_program: &magic_program_account_info,
> >     permission_program: &permission_program_account_info,
> >     authority: &counter_account_info,
> >     authority_is_signer: false,
> > }
> > .invoke_signed(&[seeds])?;
> > ```
>
> > [tab] Pinocchio
> >
> > ```rust theme={null}
> > use ephemeral_rollups_pinocchio::acl::CloseEphemeralPermission;
> > use pinocchio::cpi::{Seed, Signer};
> >
> > let bump_seed = [bump];
> > let seeds_array: [Seed; 3] = [
> >     Seed::from(b"counter"),
> >     Seed::from(authority.address().as_ref()),
> >     Seed::from(&bump_seed),
> > ];
> > let signer = Signer::from(&seeds_array);
> >
> > // Refunds the permission's rent to `payer` (the counter PDA).
> > CloseEphemeralPermission {
> >     payer: counter_account,
> >     permissioned_account: counter_account,
> >     permission,
> >     vault,
> >     magic_program,
> >     permission_program,
> >     authority: counter_account,
> >     authority_is_signer: false,
> > }
> > .invoke_signed(&[signer])?;
> > ```
>
> > [tab] Kit
> >
> > ```typescript theme={null}
> > import {
> >   EPHEMERAL_VAULT_ID,
> >   MAGIC_PROGRAM_ID,
> >   PERMISSION_PROGRAM_ID,
> > } from "@magicblock-labs/ephemeral-rollups-kit";
> > import {
> >   AccountRole,
> >   appendTransactionMessageInstructions,
> >   createTransactionMessage,
> >   pipe,
> >   setTransactionMessageFeePayer,
> >   type Instruction,
> > } from "@solana/kit";
> >
> > // Refunds the permission's rent to the counter PDA.
> > const closeIx: Instruction = {
> >   programAddress: COUNTER_PROGRAM_ID,
> >   accounts: [
> >     { address: tempKeypair.address, role: AccountRole.WRITABLE_SIGNER }, // authority
> >     { address: counterPda, role: AccountRole.WRITABLE },
> >     { address: permissionPda, role: AccountRole.WRITABLE },
> >     { address: PERMISSION_PROGRAM_ID, role: AccountRole.READONLY },
> >     { address: EPHEMERAL_VAULT_ID, role: AccountRole.WRITABLE },
> >     { address: MAGIC_PROGRAM_ID, role: AccountRole.READONLY },
> >   ],
> >   // Anchor discriminator of `close_permission` (from the program IDL)
> >   data: new Uint8Array([17, 241, 212, 43, 238, 201, 203, 210]),
> > };
> >
> > const transactionMessage = pipe(
> >   createTransactionMessage({ version: 0 }),
> >   (tx) => setTransactionMessageFeePayer(tempKeypair.address, tx),
> >   (tx) => appendTransactionMessageInstructions([closeIx], tx),
> > );
> > const sig = await ephemeralConnection.sendAndConfirmTransaction(
> >   transactionMessage,
> >   [tempKeypair.keyPair],
> >   { commitment: "confirmed" },
> > );
> > console.log("close_permission tx:", sig);
> > ```
>
> > [tab] Web3.js
> >
> > ```typescript theme={null}
> > import {
> >   MAGIC_PROGRAM_ID,
> >   PERMISSION_PROGRAM_ID,
> >   EPHEMERAL_VAULT_ID,
> > } from "@magicblock-labs/ephemeral-rollups-sdk";
> > import { Transaction, sendAndConfirmTransaction } from "@solana/web3.js";
> >
> > // Refunds the permission's rent to the counter PDA.
> > const closeIx = await counterProgram.methods
> >   .closePermission()
> >   .accountsPartial({
> >     authority: tempKeypair.publicKey,
> >     counter: counterPda,
> >     permission: permissionPda,
> >     permissionProgram: PERMISSION_PROGRAM_ID,
> >     ephemeralVault: EPHEMERAL_VAULT_ID,
> >     magicProgram: MAGIC_PROGRAM_ID,
> >   })
> >   .instruction();
> >
> > const tx = new Transaction().add(closeIx);
> > const sig = await sendAndConfirmTransaction(ephemeralConnection, tx, [tempKeypair]);
> > console.log("close_permission tx:", sig);
> > ```
>
> **Use Cases:**
>
> * Reclaim ephemeral rent before undelegating the data PDA
> * Tear down access control once private execution is finished
>
> [⬆️ Back to Top](#ephemeral-permission)

> [!TIP]
>
> Reference implementations:
> [`private-counter/anchor`](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/private-counter/anchor)
> (Anchor)
> and
> [`private-counter/pinocchio`](https://github.com/magicblock-labs/magicblock-engine-examples/tree/main/private-counter/pinocchio).

***

## Best Practices

1. **Authority Management**: Always assign AUTHORITY_FLAG to at least one trusted member
2. **Least Privilege**: Grant only necessary flags to each member
3. **Real-time Updates**: Permissions can be updated in real-time on Private Ephemeral Rollup without undelegating, allowing dynamic access control adjustments
4. **Cleanup**: Close unused ephemeral permission accounts to refund their rent to the data PDA

***

## Security Considerations

* **Signer Validation**: Only a member with AUTHORITY_FLAG, or the permissioned account itself (a PDA signing through its program), can authorize changes
* **Privacy Flag**: `is_private: false` makes the account publicly visible, even when the member list is not empty. Listing members does not restrict anything until `is_private` is `true`
* **Owner Entry**: The program records the permissioned account's owner program as the first member, with no flags. It is not an AUTHORITY member
* **Private With No Members**: `is_private: true` with an empty member list hides the account from every wallet. The data PDA can still update the permission by signing with its seeds
* **Full List on Update**: Each update replaces the member list. A member left out of the next update loses access
* **Access Auditing**: Use member flags to audit and control access

***

> [card] [Access Control](/pages/private-ephemeral-rollups-pers/how-to-guide/access-control)
>
> Fine-grained Access Control

> [card] [On-chain Privacy](/pages/private-ephemeral-rollups-pers/introduction/onchain-privacy)
>
> Privacy Mechanisms and Concepts

> [card] [Authorization](/pages/private-ephemeral-rollups-pers/introduction/authorization)
>
> Authorization Framework

> [card] [Compliance Framework](/pages/private-ephemeral-rollups-pers/introduction/compliance-framework)
>
> Compliance Standards and Guidelines


# Run Private Ephemeral Rollups Locally
