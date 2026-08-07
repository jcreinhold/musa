import { defineConfig } from "vitest/config";

/**
 * The unit suite runs in node against plain modules — the sanitizer and the
 * token file. Anything that needs a browser is a Playwright test instead, so
 * that what the screenshots prove is what the application actually renders.
 */
export default defineConfig({
  test: {
    include: ["tests/unit/**/*.test.ts"],
    environment: "node",
  },
});
