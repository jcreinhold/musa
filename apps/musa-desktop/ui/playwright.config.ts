import { defineConfig } from "@playwright/test";

/**
 * The design's regression net for the rest of the sequence
 * (`docs/prompts/20-interface-prototype.md`). Screens are compared as images
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
  use: {
    baseURL: "http://localhost:5173",
    deviceScaleFactor: 2,
  },
  webServer: {
    command: "npm run dev",
    url: "http://localhost:5173",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
