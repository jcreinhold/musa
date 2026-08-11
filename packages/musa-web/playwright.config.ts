import { defineConfig } from "@playwright/test";

/**
 * The DOM suite: real pages served by vite's dev server, so the
 * package's source is exercised exactly as a browser would load it.
 */
export default defineConfig({
  testDir: "tests/dom",
  reporter: "line",
  use: {
    baseURL: "http://localhost:5199",
  },
  webServer: {
    // The examples import the built package (and cdn.html the CDN bundle);
    // both builds are cheap and the pages must test what ships.
    command: "pnpm run build && pnpm run build:cdn && npx vite serve --port 5199 --strictPort",
    url: "http://localhost:5199/tests/dom/pages/basic.html",
    reuseExistingServer: process.env["CI"] === undefined,
    timeout: 60_000,
  },
});
