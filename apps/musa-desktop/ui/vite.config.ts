import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [svelte()],
  // Fixtures are imported (`?raw`, `?json`), never fetched: roadmap §14.8 wants
  // an application that works with no network at all, and the smoke test holds
  // it to that.
  assetsInclude: ["**/*.mei"],
  build: {
    // Verovio's WASM arrives base64-inlined in a 7 MB module. Warning about it
    // on every build would train us to ignore the warning that matters.
    chunkSizeWarningLimit: 12_000,
    target: "es2022",
  },
  worker: { format: "es" },
  server: { port: 5173, strictPort: true },
});
