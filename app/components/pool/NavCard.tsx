import type { PoolView } from "@/lib/types";
import { formatAmount, formatMusd, formatRatio } from "@/lib/format";

/** NAV per share = (vault - owed) / shares. "Owed" is what queued wins still claim from the vault. */
export function NavCard({ pool }: { pool: PoolView }) {
  return (
    <>
      <div className="stats">
        <div className="stat"><b>${formatRatio(pool.nav, 1_000_000n, 4)}</b><span>Price per share</span></div>
        <div className="stat"><b>{formatMusd(pool.vault)}</b><span>In the vault</span></div>
        <div className="stat"><b>{formatMusd(pool.owed)}</b><span>Owed to queued wins</span></div>
        <div className="stat"><b>{formatAmount(pool.shares)}</b><span>Pool shares</span></div>
        <div className="stat"><b>{formatMusd(pool.insurance)}</b><span>Insurance fund</span></div>
      </div>
      <p className="dim small pool-note">Share price is what is left in the pool after unpaid wins, divided by the number of shares. Players bet against this pool, so the price goes up when players lose and down when they win big. Depositing is risking test MUSD: you can end with less than you put in.</p>
    </>
  );
}
