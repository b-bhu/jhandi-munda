import type { PoolView } from "@/lib/types";
import { formatAmount, formatRatio } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { ExampleTag } from "@/components/shell/ExampleTag";

/** NAV per share = (vault - owed to the queue) / shares. Queued wins are IOUs, not pool money (D44). */
export function NavCard({ pool }: { pool: PoolView }) {
  const assets = pool.vault - pool.owed;
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>Pool NAV · per share</span><ExampleTag /></div>
      <div className="bigval">${formatRatio(pool.nav, 1_000_000n, 2)} <small>per share</small></div>
      <div className="formula">
        NAV = (vault − owed to queue) ÷ shares<br />
        = ({formatAmount(pool.vault)} − {formatAmount(pool.owed)} owed) ÷ {formatAmount(pool.shares, 0)}<br />
        = {formatRatio(pool.nav, 1_000_000n, 4)}
      </div>
      <div className="kv" style={{ marginTop: 16 }}><span>Vault (MUSD)</span><b>{formatAmount(pool.vault)}</b></div>
      <div className="kv"><span>Owed to the payout queue</span><b>{formatAmount(pool.owed)}</b></div>
      <div className="kv"><span>Pool shares outstanding</span><b>{formatAmount(pool.shares, 0)}</b></div>
      <div className="kv"><span>Pool assets (vault − owed)</span><b>{formatAmount(assets)}</b></div>
      <p className="note">Queued wins are IOUs, not pool money (D44). All numbers here are examples; the real ones are read from the devnet pool. Pool value can go down as well as up.</p>
    </CornerBox>
  );
}
