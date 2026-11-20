

# VRF Pricing
Source: https://docs.magicblock.gg/pages/verifiable-randomness-functions-vrfs/introduction/pricing

Pricing for provably fair randomness requests with MagicBlock Solana VRF.

MagicBlock Solana VRF is charged per randomness request. The fee is taken from the payer when the request is made. It is not refunded if the request expires unfulfilled and is purged. Fees cover proof generation and posting the verified result on-chain. Requests to the ephemeral rollup queue (`DEFAULT_EPHEMERAL_QUEUE`) are free.

<table>
  <thead>
    <tr>
      <th>VRF type</th>
      <th>Amount (SOL)</th>
      <th>Description</th>
    </tr>
  </thead>

  <tbody>
    <tr>
      <td>ER (\<50 ms)</td>
      <td>Free</td>
      <td>Per randomness request</td>
    </tr>

    <tr>
      <td>Solana (\<500 ms)</td>
      <td>0.0008</td>
      <td>Per randomness request</td>
    </tr>

    <tr>
      <td>Solana (1/2 seconds)</td>
      <td>0.0005</td>
      <td>Per randomness request</td>
    </tr>
  </tbody>
</table>

## VRF Cost Simulator: 30 Days

Solana

With ER

VRF request(s) per minute:

**M randomness provisions** per year.

You save

per year

.

Copy chart as SVG
Annual cost comparison between other VRFs and MagicBlock VRF

Price: 200 USD/SOL

> [card] [Start Building](/pages/verifiable-randomness-functions-vrfs/how-to-guide/quickstart)
>
> Build a Solana VRF request and callback flow with the MagicBlock VRF SDK.

> [card] [Solana VRF](/pages/verifiable-randomness-functions-vrfs/introduction/solana-vrf)
>
> Return to the Solana VRF overview.


# VRF Security
