

# How do Session Keys work?
Source: https://docs.magicblock.gg/pages/tools/session-keys/how-do-session-keys-work

How session keys work: an ephemeral keypair, a session token with an expiry, and revoking it.

Session Keys are meant to be used as secondary signers in your program, especially for frequent interactions like liking a post or moving a piece in a game of chess where constant popups can get in the way of smooth user experience. **They are not burner wallets**.&#x20;

Session Keys work in tandem with our on chain program to validate the token and it's scope.

## **Session Keys have two components**

1. An **Ephemeral Keypair**, intended to be used as a **secondary signer** in the target program.
2. A **Session Token**, a PDA containing information about **expiry and scope** of the keypair.

## **How do they work?**

![Session keys lifecycle](https://mintcdn.com/magicblock-42/5iyVpKJBt1PkwHw4/images/how_session_keys_work.png?fit=max\&auto=format\&n=5iyVpKJBt1PkwHw4\&q=85\&s=e7cc7647b782052a1b28c7d6bbaecc7a)

* Ephemeral Keys are stored on the client side, to invoke transactions.
* The transactions invoked by these ephemeral keys are validated in the target program for their validity, expiry and scope.
* Every transaction needs to present both the ephemeral signer and the session token
* This is the general idea behind *account abstraction*, where instead of just an externally owned key there is also smart contract that enhances security.


# Install Session Keys
