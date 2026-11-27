import test from "node:test";
import assert from "node:assert/strict";
import { parseSim } from "./gen-sim.mjs";

const SAMPLE = `simulation: 3 rounds, seed 1
round   0 | bets P0:f6 P1:f2 P2:f1 | faces [6, 4, 3, 4, 4, 1] | pool   23000000 | NAV  1.150000 | queue 0 | insurance 0
JACKPOT round 1 (index 1): P0 bet 1.000000 on face 6, faces [6, 6, 6, 6, 6, 6], six matches, 101x gross, owed 99.000000
  before settle: pool 22760000 | queue 0 | insurance 0
  after settle:  pool 25760000 | queue 2 | insurance 0  (queued when the pool is short)
  after the queue pass and the round: pool 25760000 | queue 2 | insurance 0
round   1 | bets P0:f6 P1:f5 P2:f6 | faces [6, 6, 6, 6, 6, 6] | pool   25760000 | NAV  1.088742 | queue 2 | insurance 0
round   2 | bets - | faces [1, 2, 3, 4, 5, 6] | pool   25760000 | NAV  1.088742 | queue 2 | insurance 0 | LP deposit refused: pool insolvent (queue owes more than the vault)
`;

test("parses rounds, bets, faces and numbers", () => {
  const s = parseSim(SAMPLE);
  assert.equal(s.rounds.length, 3);
  assert.deepEqual(s.rounds[0].bets, [{ player: 0, face: 6 }, { player: 1, face: 2 }, { player: 2, face: 1 }]);
  assert.deepEqual(s.rounds[0].faces, [6, 4, 3, 4, 4, 1]);
  assert.equal(s.rounds[0].vault, "23000000");
  assert.equal(s.rounds[0].navMicro, 1150000);
  assert.equal(s.rounds[1].queue, 2);
});

test("an empty round and a note are kept", () => {
  const r = parseSim(SAMPLE).rounds[2];
  assert.deepEqual(r.bets, []);
  assert.match(r.note, /LP deposit refused/);
});

test("the jackpot block attaches to its round", () => {
  const j = parseSim(SAMPLE).jackpot;
  assert.equal(j.round, 1);
  assert.equal(j.owedMusd, "99.000000");
  assert.deepEqual(j.beforeSettle, { vault: "22760000", queue: 0, insurance: "0" });
  assert.equal(j.afterSettle.queue, 2);
});

test("rejects text that is not sim output", () => {
  assert.throws(() => parseSim("hello"), /no round lines/);
  assert.throws(() => parseSim("round 0 | bets P0:f9 | faces [1,2,3,4,5,6] | pool 1 | NAV 1.0 | queue 0 | insurance 0"), /bad bet/);
});
