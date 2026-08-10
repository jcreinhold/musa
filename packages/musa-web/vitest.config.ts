import { defineConfig } from "vitest/config";

/**
 * The unit suite runs in Node: the wasm module loads from bytes, and the
 * engraver takes its in-process path. DOM behavior is Playwright's job
 * (prompt 141).
 */
export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
    environment: "node",
  },
});
