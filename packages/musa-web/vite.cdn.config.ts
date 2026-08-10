import { fileURLToPath } from "node:url";

import { defineConfig, type Plugin } from "vite";

const STUB = fileURLToPath(new URL("./src/cdn-worker-default-stub.ts", import.meta.url));

/**
 * The default worker URL would emit a worker asset nobody fetches — the
 * Blob-inlined worker replaces it (see cdn-entry.ts). Alias at resolution,
 * where the importer is known.
 */
function noDefaultWorker(): Plugin {
  return {
    name: "musa-cdn-no-default-worker",
    enforce: "pre",
    resolveId: {
      filter: { id: /^\.\/worker-default$/ },
      handler(source, importer) {
        if (importer?.endsWith("musa-engrave/src/engraver.ts") === true) return STUB;
        return null;
      },
    },
  };
}

/**
 * The CDN build (prompt 143): one classic script, one file — the worker
 * (with Verovio inside) arrives as an inlined Blob, so there is no worker
 * URL to configure. The only external asset is the musa compiler wasm,
 * copied beside `musa-web.js` by `scripts/prepare-cdn.mjs`:
 *
 *   dist-cdn/
 *     musa-web.js         ← the one <script src>
 *     musa_wasm_bg.wasm   ← the compiler
 */
export default defineConfig({
  plugins: [noDefaultWorker()],
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
      // The in-process fallback's Verovio is dead code here (a browser has
      // workers); externalizing keeps the main thread from carrying a second
      // copy. It is only ever reached where this file is never used.
      external: ["verovio/wasm"],
      output: {
        inlineDynamicImports: true,
        globals: { "verovio/wasm": "MusaWebVerovioFallback" },
      },
    },
    assetsInlineLimit: 0,
    target: "es2022",
    chunkSizeWarningLimit: 14_000,
  },
});
