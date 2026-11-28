import { afterEach, describe, expect, it, vi } from "vitest";
import { dataMode, getClient, isMock } from "./game-client";
import { createMockClient } from "./mock-client";

afterEach(() => { vi.unstubAllEnvs(); });

describe("the data flag", () => {
  it("defaults to mock", () => {
    vi.stubEnv("NEXT_PUBLIC_DATA", "");
    expect(dataMode()).toBe("mock");
    expect(isMock()).toBe(true);
  });
  it("program mode refuses to hand out a client until program-client.ts exists", () => {
    vi.stubEnv("NEXT_PUBLIC_DATA", "program");
    expect(dataMode()).toBe("program");
    expect(() => getClient()).toThrow(/program-client/);
  });
});

describe("the mock client keeps the program's rules", () => {
  it("a loss claim has no Take now (D50), a win claim has 95%", async () => {
    const loss = createMockClient("loss");
    const [lossClaim] = await loss.getMyClaims();
    expect(lossClaim.kind).toBe("loss");
    expect(lossClaim.takeNowPayout).toBeNull();
    await expect(loss.redeemClaim(lossClaim.id, "takeNow")).rejects.toThrow(/Wait only/);

    const win = createMockClient("win");
    const [winClaim] = await win.getMyClaims();
    expect(winClaim.kind).toBe("win");
    expect(winClaim.takeNowPayout).toBe((winClaim.amount * 95n) / 100n);
  });
  it("NAV is (vault - owed) / shares", async () => {
    const pool = await createMockClient("open").getPool();
    expect(pool.nav).toBe(((pool.vault - pool.owed) * 1_000_000n) / pool.shares);
  });
});
