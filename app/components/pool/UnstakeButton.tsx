/** Unstake is refused while the admin transfer switch is off (D46, D47), so the button says why. */
export function UnstakeButton({ transfersEnabled, staked }: { transfersEnabled: boolean; staked: bigint }) {
  const disabled = !transfersEnabled || staked === 0n;
  return (
    <div className="bet-panel">
      <button type="button" className="btn ghost" disabled={disabled} aria-describedby="unstake-why">Unstake</button>
      <p id="unstake-why" className="dim small">
        {transfersEnabled ? "Unstaking returns MUNDA to your wallet." : "Locked for now: you can't unstake MUNDA until transfers are turned back on."}
      </p>
    </div>
  );
}
