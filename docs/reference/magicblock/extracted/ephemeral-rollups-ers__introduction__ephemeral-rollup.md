

# Delegating Accounts to an Ephemeral Rollup
Source: https://docs.magicblock.gg/pages/ephemeral-rollups-ers/introduction/ephemeral-rollup

How to delegate a Solana account to an Ephemeral Rollup, run transactions on it, commit the state back to Solana, and undelegate.

![](https://mintcdn.com/magicblock-42/5iyVpKJBt1PkwHw4/images/architecture_overview.png?fit=max\&auto=format\&n=5iyVpKJBt1PkwHw4\&q=85\&s=49ab1e3257a554ea6ce7d2b8cba9b1e8)

[Magicblock's Ephemeral Rollup](/pages/overview/additional-information/whitepaper) **leverages the Solana Virtual Machine (SVM)’s account-based structure and parallel execution** to optimize state management. By structuring the state into **clusters**, users can **lock one or multiple accounts** and temporarily shift state execution to a **dedicated auxiliary layer**— "Ephemeral Rollup (ER)". A dynamic fraud-proof mechanism enables fast state finalization through a decentralized Security Committee, see [whitepaper](/public/Ephemeral_Rollups_Fraud_Proof.pdf).

***

## Account Lifecycle for executing transactions in real-time with ER:

![](https://mintcdn.com/magicblock-42/Thv8LjyS00pp5fQ2/images/magicblock-delegation-lifecycle.png?fit=max\&auto=format\&n=Thv8LjyS00pp5fQ2\&q=85\&s=7e3dc70c29cedb2485d4610e446137b4)

**1. Delegate Account**

State accounts must be delegated to an specific ER validator first by
changing the account owner to the [Delegation
Program](https://github.com/magicblock-labs/delegation-program)
`DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh` and specifying the ER
validator.

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

**2. Execute Transaction in real-time**

Delegated state accounts are updated in real-time with transactions on the
ER directly or via [Magic
Router](/pages/ephemeral-rollups-ers/introduction/magic-router). The
**initial transaction on ER clones the delegated account** from base layer
to the ephemeral rollup.

**3. Commit State**

The operator commits the ephemeral state to the base layer **on demand**,
when your program schedules a commit, including new state and relevant
pointers. Validators do not commit delegated accounts automatically. The account state is
finalized using a fraud-proof mechanism as detailed in the paper.

**4. Execute Transaction in real-time continuously**

Delegated account states can continuously be updated in real-time on the ER
directly or via [Magic
Router](/pages/ephemeral-rollups-ers/introduction/magic-router).

**5. Undelegate Account**

Delegated account states are committed through ER validator to the base
layer and the account owner are reversed from the [Delegation
Program](https://github.com/magicblock-labs/delegation-program)
`DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh` to the original owner.


# Ephemeral Rollups FAQ
