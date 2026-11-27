import { describe, expect, it } from "vitest";
import { explorerAddress, explorerTx, formatAmount, formatClock, formatMusd, formatRatio, formatWait, parseAmount } from "./format";

describe("money formatting (6 decimals, bigint, no floats)", () => {
  it("formats base units with commas and rounds half up", () => {
    expect(formatAmount(1_234_560_000n)).toBe("1,234.56");
    expect(formatAmount(1_000_000n)).toBe("1.00");
    expect(formatAmount(2_960_000n)).toBe("2.96");
    expect(formatAmount(1_005_000n)).toBe("1.01"); // half rounds up
    expect(formatAmount(999_999n)).toBe("1.00");
    expect(formatAmount(0n)).toBe("0.00");
    expect(formatAmount(-2_500_000n)).toBe("-2.50");
    expect(formatAmount(1_500_000n, 0)).toBe("2");
    expect(formatAmount(1_234_567n, 6)).toBe("1.234567");
  });
  it("is exact for amounts a float cannot hold", () => {
    expect(formatAmount(9_007_199_254_740_993_000_000n)).toBe("9,007,199,254,740,993.00");
  });
  it("adds the unit", () => { expect(formatMusd(5_000_000n)).toBe("5.00 MUSD"); });
  it("parses what a person types", () => {
    expect(parseAmount("12.5")).toBe(12_500_000n);
    expect(parseAmount(" 5 ")).toBe(5_000_000n);
    expect(parseAmount("0.000001")).toBe(1n);
    for (const bad of ["", "abc", "1.2.3", "-1", "1e3", "1,000", "0.1234567", ".5"]) expect(parseAmount(bad)).toBeNull();
  });
  it("formats a ratio", () => {
    expect(formatRatio(1_250_000n, 1_000_000n, 4)).toBe("1.2500");
    expect(formatRatio(1n, 0n)).toBe("1.0000");
  });
});

describe("time and links", () => {
  it("formats clocks and waits", () => {
    expect(formatClock(72)).toBe("01:12");
    expect(formatClock(-5)).toBe("00:00");
    expect(formatWait(86_340)).toBe("23h 59m");
    expect(formatWait(250)).toBe("4m 10s");
    expect(formatWait(30)).toBe("30s");
  });
  it("builds devnet Explorer links", () => {
    expect(explorerTx("abc")).toBe("https://explorer.solana.com/tx/abc?cluster=devnet");
    expect(explorerAddress("xyz")).toBe("https://explorer.solana.com/address/xyz?cluster=devnet");
  });
});
