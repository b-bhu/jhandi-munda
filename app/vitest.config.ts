import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

// Plain Node tests for the logic in lib/ (no browser, no Next). "@/" matches tsconfig paths.
export default defineConfig({
  resolve: { alias: { "@": fileURLToPath(new URL("./", import.meta.url)) } },
  test: { include: ["lib/**/*.test.ts"], environment: "node" },
});
