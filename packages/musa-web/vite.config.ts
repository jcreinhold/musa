import { defineConfig } from "vite";
import dts from "vite-plugin-dts";

/**
 * The npm library build: one ESM module. Verovio stays external (a peer —
 * one version across the workspace, chosen by the app); the CDN build
 * inlines everything instead.
 */
export default defineConfig(({ command }) => ({
  plugins: [dts({ include: ["src"] })],
  // Relative asset URLs: a library is served from anywhere, never from the
  // server root. The dev server (Playwright) keeps absolute paths.
  base: command === "build" ? "./" : "/",
  build: {
    lib: {
      entry: "src/index.ts",
      formats: ["es"],
      fileName: () => "index.js",
    },
    rollupOptions: {
      external: [/^node:/, "verovio/wasm", "verovio/esm"],
    },
    // The musa wasm stays a separate, cacheable file — not base64 in the JS.
    assetsInlineLimit: 0,
    target: "es2022",
  },
  worker: { format: "es" },
}));
