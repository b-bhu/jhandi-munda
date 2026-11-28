import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { checkCluster, DEVNET_GENESIS } from "./wallet";

const MAINNET_GENESIS = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
const reply = (body: unknown) => vi.fn(async () => ({ json: async () => body })) as unknown as typeof fetch;

describe("checkCluster (the devnet guard)", () => {
  it("passes only for the devnet genesis hash", async () => {
    expect(await checkCluster("rpc", reply({ result: DEVNET_GENESIS }))).toBe("devnet");
  });
  it("blocks another cluster (mainnet)", async () => {
    expect(await checkCluster("rpc", reply({ result: MAINNET_GENESIS }))).toBe("blocked");
  });
  it("fails closed on a made-up hash", async () => {
    expect(await checkCluster("rpc", reply({ result: "x".repeat(44) }))).toBe("blocked");
  });
  it.each([
    ["a non-string result", { result: 42 }],
    ["an RPC error", { error: { code: -32601, message: "no" } }],
    ["an empty reply", {}],
    ["null", null],
  ])("is unknown (not devnet) for %s", async (_name, body) => {
    expect(await checkCluster("rpc", reply(body))).toBe("unknown");
  });
  it("is unknown when the network fails", async () => {
    const down = vi.fn(async () => { throw new Error("offline"); }) as unknown as typeof fetch;
    expect(await checkCluster("rpc", down)).toBe("unknown");
  });
  it("is unknown when the reply is not JSON", async () => {
    const bad = vi.fn(async () => ({ json: async () => { throw new SyntaxError("bad json"); } })) as unknown as typeof fetch;
    expect(await checkCluster("rpc", bad)).toBe("unknown");
  });
  it("asks getGenesisHash with a POST to the given url", async () => {
    const f = reply({ result: DEVNET_GENESIS });
    await checkCluster("https://example.test/rpc", f);
    const [url, init] = (f as unknown as ReturnType<typeof vi.fn>).mock.calls[0] as [string, RequestInit];
    expect(url).toBe("https://example.test/rpc");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toMatchObject({ jsonrpc: "2.0", method: "getGenesisHash" });
  });
});

// connectWallet reads module state, so each test loads a fresh copy of the module.
describe("connectWallet never reaches the wallet unless the guard says devnet", () => {
  const connect = vi.fn(async () => ({ publicKey: { toString: () => "5qPR…acm7" } }));
  beforeEach(() => {
    vi.resetModules();
    connect.mockClear();
    vi.stubGlobal("window", { phantom: { solana: { connect, disconnect: async () => {} } } });
  });
  afterEach(() => { vi.unstubAllGlobals(); });

  async function withRpc(result: unknown | "throw") {
    vi.stubGlobal("fetch", result === "throw" ? vi.fn(async () => { throw new Error("offline"); }) : reply({ result }));
    return import("./wallet");
  }

  it("mainnet: blocked, wallet.connect is never called", async () => {
    const w = await withRpc(MAINNET_GENESIS);
    await w.verifyCluster();
    await w.connectWallet();
    expect(connect).not.toHaveBeenCalled();
    expect(w.useWallet).toBeTypeOf("function");
  });
  it("network down: blocked, wallet.connect is never called", async () => {
    const w = await withRpc("throw");
    await w.verifyCluster();
    await w.connectWallet();
    expect(connect).not.toHaveBeenCalled();
  });
  it("before the check has run: blocked", async () => {
    const w = await withRpc(DEVNET_GENESIS);
    await w.connectWallet();
    expect(connect).not.toHaveBeenCalled();
  });
  it("devnet: the wallet is asked to connect", async () => {
    const w = await withRpc(DEVNET_GENESIS);
    await w.verifyCluster();
    await w.connectWallet();
    expect(connect).toHaveBeenCalledTimes(1);
  });
});
