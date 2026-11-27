import type { Replay, SimRound } from "@/lib/types";
import { SYMBOLS } from "@/lib/symbols";
import { PAYOUT_MULTIPLIER, matches, owedFor } from "@/lib/payouts";
import { formatMusd } from "@/lib/format";
import { CornerBox } from "@/components/ui/CornerBox";
import { DiceRow } from "@/components/table/DiceRow";

const STAKE = 1_000_000n; // every simulated bet is 1 MUSD
const SIM = <span className="mock">Simulation</span>;

/** The round the scrubber is on: dice, each bet, and what the pool did about the wins. */
export function RoundPanel({ round, replay }: { round: SimRound; replay: Replay }) {
  const jackpot = replay.jackpot && replay.jackpot.round === round.id ? replay.jackpot : null;
  const owedNow = round.bets.reduce((a, b) => a + owedFor(STAKE, matches(b.face, round.faces)), 0n);
  const waits = jackpot ? jackpot.afterSettle.queueOwed - jackpot.beforeSettle.queueOwed : 0n;
  return (
    <CornerBox className="panel">
      <div className="ptitle"><span>Round {round.id}</span>{SIM}</div>
      <DiceRow faces={round.faces} mark />
      {round.bets.length === 0 && <p className="small" style={{ marginTop: 16 }}>No bets this round.</p>}
      {round.bets.map((b, i) => {
        const k = matches(b.face, round.faces);
        const owed = owedFor(STAKE, k);
        return (
          <div className="kv" key={i} style={i === 0 ? { marginTop: 16 } : undefined}>
            <span>Player {b.player + 1} · $1 on {SYMBOLS.find((s) => s.id === b.face)?.label.split(" ")[0]} · {k} of 6{PAYOUT_MULTIPLIER[k] ? ` (${PAYOUT_MULTIPLIER[k]}x back)` : ""}</span>
            {owed > 0n ? <b className="g">owed {formatMusd(owed)}</b> : <b className="r">lost 1.00 MUSD</b>}
          </div>
        );
      })}
      {jackpot && (
        <>
          <div className="kv"><span>Owed this round</span><b className="g">{formatMusd(owedNow)}</b></div>
          <div className="kv"><span>Paid now from the pool</span><b>{formatMusd(owedNow - waits)}</b></div>
          <div className="kv"><span>Waits in the queue</span><b>{formatMusd(waits)}</b></div>
        </>
      )}
      <div className="kv" style={round.bets.length === 0 ? { marginTop: 16 } : undefined}><span>Pool cash after the round</span><b>{formatMusd(round.vault)}</b></div>
      <div className="kv"><span>Queue after the round</span><b>{round.queue} {round.queue === 1 ? "win" : "wins"} · {formatMusd(round.queueOwed)}</b></div>
      {round.note && <p className="note">{round.note}</p>}
    </CornerBox>
  );
}
