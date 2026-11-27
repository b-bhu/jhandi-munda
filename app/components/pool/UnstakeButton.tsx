/** Unstake is refused while the admin transfer switch is off (D46, D47), so the button says why. */
export function UnstakeButton({ transfersEnabled, staked }: { transfersEnabled: boolean; staked: bigint }) {
  const disabled = !transfersEnabled || staked === 0n;
  return (
    <button type="button" className={`go${disabled ? " off" : ""}`} style={{ marginTop: 14 }} disabled={disabled} aria-describedby="unstake-why">
      {transfersEnabled ? "Unstake" : "Unstake · disabled until transfers are on"}
    </button>
  );
}
