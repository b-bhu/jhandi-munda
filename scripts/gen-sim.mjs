#!/usr/bin/env node
/**
 * Writes the replay fixture app/public/sim.json from the REAL program simulation
 * (scripts/sim-rounds.sh: the deployed program's code run in LiteSVM, no network, no keys).
 *
 *   node scripts/gen-sim.mjs [--rounds 50] [--seed 1]            builds and runs the sim, then writes the file
 *   node scripts/gen-sim.mjs --from-file sim-output.txt          parses saved sim output instead
 *
 * The /replay page labels it SIMULATION: these are test bets by three made-up players, not devnet rounds.
 */
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = resolve(root, "app/public/sim.json");

/** Parses the text printed by `sim_rounds_from_env` into the fixture object. Pure, so it is testable. */
export function parseSim(text, seed = 1) {
  const rounds = [];
  let jackpot = null;
  let pending = null; // a JACKPOT block whose summary line comes after it
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const jp = /^JACKPOT round (\d+) .*?owed ([\d.]+)/.exec(line);
    if (jp) {
      const pick = (s) => /pool (\d+) \| queue (\d+) \| insurance (\d+)(?: \| owed (\d+))?/.exec(s);
      const [b, a, f] = [lines[i + 1], lines[i + 2], lines[i + 3]].map(pick);
      if (!b || !a || !f) throw new Error(`malformed JACKPOT block at line ${i + 1}`);
      const st = (m) => ({ vault: m[1], queue: Number(m[2]), insurance: m[3], owed: m[4] ?? "0" });
      pending = { round: Number(jp[1]), owedMusd: jp[2], beforeSettle: st(b), afterSettle: st(a), afterRound: st(f) };
      i += 3;
      continue;
    }
    const m = /^round\s+(\d+) \| bets (.*?) \| faces \[([\d, ]+)\] \| pool\s+(\d+) \| NAV\s+([\d.]+) \| queue (\d+) \| insurance (\d+)(?: \| owed (\d+))?(?: \| (.+))?$/.exec(line);
    if (!m) continue;
    const bets = m[2].trim() === "-" ? [] : m[2].trim().split(/\s+/).map((b) => {
      const bm = /^P(\d+):f([1-6])$/.exec(b);
      if (!bm) throw new Error(`bad bet "${b}" in: ${line}`);
      return { player: Number(bm[1]), face: Number(bm[2]) };
    });
    const faces = m[3].split(",").map((x) => Number(x.trim()));
    if (faces.length !== 6) throw new Error(`expected six faces in: ${line}`);
    rounds.push({
      id: Number(m[1]), bets, faces, vault: m[4], navMicro: Math.round(Number(m[5]) * 1e6),
      queue: Number(m[6]), insurance: m[7], owed: m[8] ?? "0", note: m[9] ?? null,
    });
    if (pending && pending.round === Number(m[1])) { jackpot = pending; pending = null; }
  }
  if (rounds.length === 0) throw new Error("no round lines found: is this the sim output?");
  return { simulation: true, source: "scripts/sim-rounds.sh: the program's code run in LiteSVM", seed, stakeBaseUnits: "1000000", rounds, jackpot };
}

function main() {
  const args = process.argv.slice(2);
  const arg = (k, d) => (args.includes(k) ? args[args.indexOf(k) + 1] : d);
  const seed = Number(arg("--seed", "1"));
  let text;
  if (args.includes("--from-file")) {
    text = readFileSync(resolve(arg("--from-file")), "utf8");
  } else {
    const r = spawnSync(resolve(root, "scripts/sim-rounds.sh"), ["--rounds", arg("--rounds", "50"), "--seed", String(seed)], { encoding: "utf8", maxBuffer: 1 << 26 });
    if (r.status !== 0) { console.error(r.stderr || r.stdout); process.exit(r.status ?? 1); }
    text = r.stdout;
  }
  const sim = parseSim(text, seed);
  mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, JSON.stringify(sim) + "\n");
  console.log(`wrote ${OUT}: ${sim.rounds.length} rounds, seed ${seed}${sim.jackpot ? `, jackpot at round ${sim.jackpot.round}` : ""}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
