---
id: 199
slug: web-distribution-and-examples
status: done
depends_on: [197, 198]
phase: 5
---

# CDN Distribution, Examples, and the Static-Site Recipe

## Task

Make `@musa/web` consumable the two ways the ecosystem actually consumes such libraries — an npm dependency for bundled
apps, and one `<script src>` tag for everyone else — and prove both with real, committed example pages. Also ship the
third mode MathJax users rely on: typesetting at *build time* for static sites, using `musa` and Node, with zero wasm on
the client.

## Read

- Prompts 198–198 — the package being distributed; its asset-resolution rules (`configure`, `import.meta.url`
  derivation) are what the CDN build must make work from a plain script tag.
- MathJax's CDN model (combined component + config-before-load) and mermaid's (`mermaid@11/dist/…` on a CDN,
  `initialize` in the page).
- `examples/*.musa` — the corpus the example pages draw from; snippet sources are copied into the example HTML or
  fetched as static files, never compiled from the repo at serve time.
- The worker-inlining problem: `new Worker(new URL(...))` needs a bundler; a single-file CDN build must inline the
  worker source and spawn it from a Blob URL.

## Design

**CDN build.** A second vite library entry (`src/cdn-entry.ts`) producing `dist-cdn/musa-web.js` (iife, everything JS
inlined: API, typesetter, engraver client). Implementation evidence corrected two assumptions of the original design:
Verovio ships its wasm base64-inlined inside its own module — there is no separate Verovio wasm file to serve — so the
**only** external asset is the musa compiler wasm; and vite's first-class `?worker&inline` (with `musa-engrave` gaining
`createEngraver({ worker })` and a `./worker` subpath) does the Blob inlining, so no worker source surgery is needed.
The iife build therefore:

1. Inlines the *worker* as a Blob (`?worker&inline`): the CDN user never configures a worker URL. Two resolution aliases
   keep the main file lean: the default worker-URL construction (would emit a worker asset nobody fetches) and the
   in-process engraver path (would carry Verovio's 7 MB into the main thread) are stubbed — both unreachable in a
   browser.
2. Resolves the musa wasm the MathJax way: `configure({ wasmUrl })` wins, then `window.MusaWeb = { assetsPath: "…" }`
   set before the script tag, then the script's own directory (`document.currentScript`, captured at import), then
   beside the module. `scripts/prepare-cdn.mjs` copies `musa_wasm_bg.wasm` beside `musa-web.js` so serving `dist-cdn/`
   from any static host or CDN just works.

**npm publish layout.** `exports`: `.` (ESM, types), `./cdn` (the iife file, for hosts that proxy npm), `./wasm` (the
musa wasm asset path). `sideEffects: true` (element registration and auto-start happen at import). A `prepublishOnly`
script runs the wasm build, tests, and both bundles; publishing itself is manual for now (no CI registry credentials in
this prompt).

**Examples** (`packages/musa-web/examples/`, plain HTML, served by `vite preview` or any static server; each page is
also a Playwright smoke test so the examples can never silently rot):

- `blog.html` — a prose article with two `<musa-score>` excerpts (from `examples/canon.musa`,
  `examples/changing-meter.musa`) and one inline fragment; the MathJax-in-a-blog-post shape.
- `docs.html` — a language-docs page: several short snippets, one deliberately broken to show the error box, auto-start
  via `data-musa-autostart`.
- `dynamic.html` — snippets injected after load (a "add example" button), typeset via `watch`.
- `interactive.html` — `onEventClick` + `highlight`: clicking a note shows its event id and lights the whole event (ties
  included); a "reveal source" toggle showing the hidden light-DOM source.
- `cdn.html` — the only page using the iife build via a relative `dist/` path, proving the script-tag story without a
  bundler.

**Static-site recipe** (package README + `examples/build-time/`): a small Node script using `@musa/web` in Node (prompt
196's worker-less path) that typesets every `text/musa` block in a glob of HTML files and writes the SVG in, ahead of
time. For sites that want no client wasm at all. One page of docs, one script, one test proving the emitted page needs
no JS.

## Target

- `vite.cdn.config.ts` (iife + `?worker&inline` Blob worker + the two aliases), `src/cdn-entry.ts`,
  `scripts/prepare-cdn.mjs` (wasm copied beside the bundle).
- `musa-engrave`: `createEngraver({ worker })`, a `./worker` subpath export, `worker-default.ts`.
- `package.json` publish layout: `exports`, `files`, `sideEffects`, `prepublishOnly`.
- The five example pages + `examples/build-time/` script and its test.
- Playwright smoke test per example page (typesets without console errors; `cdn.html` typesets with no module scripts on
  the page; build-time page has SVG present with JS disabled).
- Package README: three consumption modes (npm, CDN, build-time), asset layout diagram, size table (brotli'd musa wasm,
  verovio wasm, JS) updated by the build script.

## Check

```sh
pnpm --filter @musa/web test          # unit + playwright, including example smokes
pnpm --filter @musa/web build         # ESM dist
pnpm --filter @musa/web build:cdn     # iife dist + assets
cd packages/musa-web/examples && python3 -m http.server  # manual spot-check of every page
```

Commit as `Add CDN distribution and examples to @musa/web`.

## Stop

- No CI publish workflow, no npm org automation, no versioned CDN hosting infrastructure.
- No framework-specific example apps (Next/Astro/SvelteKit starters): the recipe + custom element cover them until a
  real user asks.
- No source-maps/debug builds beyond vite defaults; no minification forks.
- No service-worker caching, no prefetch machinery: static assets on a CDN are already the answer.
- No playback button on the examples (prompt 200, deferred).
