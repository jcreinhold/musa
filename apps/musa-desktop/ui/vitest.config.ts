import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

/**
 * The unit suite runs in node against plain modules — the sanitizer, the
 * token file, and the session store. Anything that needs a browser is a
 * Playwright test instead, so that what the screenshots prove is what the
 * application actually renders.
 *
 * The Svelte plugin is here for `.svelte.ts` modules: the session's state is
 * runes, and testing it any other way would test a copy of it.
 */
export default defineConfig({
  plugins: [svelte({ compilerOptions: { hmr: false } })],
  test: {
    include: ["tests/unit/**/*.test.ts"],
    environment: "node",
  },
});
