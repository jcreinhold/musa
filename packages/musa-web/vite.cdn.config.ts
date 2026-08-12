import { fileURLToPath } from "node:url";

import { defineConfig, type Plugin } from "vite";

const WORKER_STUB = fileURLToPath(
  new URL("./src/cdn-worker-default-stub.ts", import.meta.url),
);
const CORE_STUB = fileURLToPath(
  new URL("./src/cdn-core-stub.ts", import.meta.url),
);

/**
 * Two aliasing rules keep the main file lean:
 * - the default worker URL would emit a worker asset nobody fetches — the
 *   Blob-inlined worker replaces it (see cdn-entry.ts);
 * - the in-process engraver path would pull Verovio into the main thread —
 *   unreachable in a browser, where the worker always exists.
 * Both alias only the main-thread importer; the worker sub-build keeps the
 * real modules.
 */
function cdnAliases(): Plugin {
  return {
    name: "musa-cdn-aliases",
    enforce: "pre",
    resolveId: {
      filter: { id: /^\.(\/worker-default|\/core)$/ },
      handler(source, importer) {
        if (importer?.endsWith("musa-engrave/src/engraver.ts") !== true)
          return null;
        return source === "./core" ? CORE_STUB : WORKER_STUB;
      },
    },
  };
}

/**
 * The CDN build: one classic script, one file — the worker
 * (with Verovio inside) arrives as an inlined Blob, so there is no worker
 * URL to configure. The only external asset is the musa compiler wasm,
 * copied beside `musa-web.js` by `scripts/prepare-cdn.mjs`:
 *
 *   dist-cdn/
 *     musa-web.js         ← the one <script src>
 *     musa_wasm_bg.wasm   ← the compiler
 */
export default defineConfig({
  plugins: [cdnAliases()],
  build: {
    lib: {
      entry: "src/cdn-entry.ts",
      formats: ["iife"],
      name: "MusaWeb",
      fileName: () => "musa-web.js",
    },
    outDir: "dist-cdn",
    emptyOutDir: true,
    rollupOptions: {
      output: { inlineDynamicImports: true },
    },
    assetsInlineLimit: 0,
    target: "es2022",
    chunkSizeWarningLimit: 14_000,
  },
});
