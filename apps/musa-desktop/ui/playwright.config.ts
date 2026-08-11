import { defineConfig } from "@playwright/test";

/**
 * The design's regression net. Screens are compared as images
 * because that is the only thing that catches a design regression: a token
 * that drifted, a sanitizer that stopped re-inking, an engraving that lost
 * its spacing.
 */
export default defineConfig({
  testDir: "./tests/screens",
  snapshotPathTemplate: "tests/__screenshots__/{arg}{ext}",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? "github" : "list",
  timeout: 60_000,
  expect: {
    // Font rasterization differs by a hair between machines; the engraving
    // itself does not. A small tolerance keeps the net honest without making
    // it a machine-identity test.
    toHaveScreenshot: { maxDiffPixelRatio: 0.01 },
  },
  /*
   * Budgets are measured on their own. `06-performance.md`'s numbers are what
   * one composer's machine does for one composer; measured while five other
   * browsers fight for the same cores, they measure the harness instead. So
   * the screens run in parallel, and the budgets run after them, alone.
   */
  projects: [
    { name: "screens", testIgnore: /perf\.spec\.ts/ },
    {
      name: "budgets",
      testMatch: /perf\.spec\.ts/,
      dependencies: ["screens"],
      fullyParallel: false,
    },
  ],
  use: {
    baseURL: "http://localhost:5173",
    deviceScaleFactor: 2,
  },
  webServer: {
    command: "pnpm run dev",
    url: "http://localhost:5173",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
