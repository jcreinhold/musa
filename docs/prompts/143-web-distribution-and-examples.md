---
id: 143
slug: web-distribution-and-examples
status: pending
depends_on: [141, 142]
phase: 5
---

# CDN Distribution, Examples, and the Static-Site Recipe

## Task

Make `@musa/web` consumable the two ways the ecosystem actually consumes such libraries — an npm
dependency for bundled apps, and one `<script src>` tag for everyone else — and prove both with real,
committed example pages. Also ship the third mode MathJax users rely on: typesetting at *build time* for
static sites, using `musa-cli` and Node, with zero wasm on the client.

## Read

- Prompts 140–142 — the package being distributed; its asset-resolution rules (`configure`,
  `import.meta.url` derivation) are what the CDN build must make work from a plain script tag.
- MathJax's CDN model (combined component + config-before-load) and mermaid's (`mermaid@11/dist/…` on a
  CDN, `initialize` in the page).
- `examples/*.musa` — the corpus the example pages draw from; snippet sources are copied into the
  example HTML or fetched as static files, never compiled from the repo at serve time.
- The worker-inlining problem: `new Worker(new URL(...))` needs a bundler; a single-file CDN build
  must inline the worker source and spawn it from a Blob URL.

## Design

**CDN build.** A second vite library entry producing `dist/musa-web.js` (iife, everything JS inlined:
API, typesetter, engraver client). Two assets cannot be inlined into JS: the musa wasm and the Verovio
wasm. The iife build therefore:

1. Inlines the *worker* as a Blob: the worker module is emitted as raw text by the build and spawned
   with `new Worker(URL.createObjectURL(new Blob([source])))`, so the CDN user never configures a
   worker URL.
2. Resolves the two wasm URLs the MathJax way: an explicit `window.MusaWeb = { assetsPath: "…" }` set
   before the script tag wins; otherwise the URL is derived from the script's own `src`
   (`document.currentScript`), so serving the package's `dist/` directory from any static host or CDN
   just works. Both wasm files are copied into `dist/` by the build and must sit beside `musa-web.js`.

**npm publish layout.** `exports`: `.` (ESM, types), `./cdn` (the iife file, for hosts that proxy npm),
`./wasm` (the musa wasm asset path). `sideEffects: false` except the stylesheet injection and custom
element registration, which are marked. A `prepublishOnly` script runs the wasm build, tests, and the
bundles; publishing itself is manual for now (no CI registry credentials in this prompt).

**Examples** (`packages/musa-web/examples/`, plain HTML, served by `vite preview` or any static
server; each page is also a Playwright smoke test so the examples can never silently rot):

- `blog.html` — a prose article with two `<musa-score>` excerpts (from `examples/canon.musa`,
  `examples/changing-meter.musa`) and one inline fragment; the MathJax-in-a-blog-post shape.
- `docs.html` — a language-docs page: several short snippets, one deliberately broken to show the
  error box, auto-start via `data-musa-autostart`.
- `dynamic.html` — snippets injected after load (a "add example" button), typeset via `watch`.
- `interactive.html` — `onEventClick` + `highlight`: clicking a note shows its event id and lights the
  whole event (ties included); a "reveal source" toggle showing the hidden light-DOM source.
- `cdn.html` — the only page using the iife build via a relative `dist/` path, proving the
  script-tag story without a bundler.

**Static-site recipe** (package README + `examples/build-time/`): a small Node script using `@musa/web`
in Node (prompt 140's worker-less path) that typesets every `text/musa` block in a glob of HTML files
and writes the SVG in, ahead of time. For sites that want no client wasm at all. One page of docs, one
script, one test proving the emitted page needs no JS.

## Target

- `vite.cdn.config.ts` (iife + Blob-inlined worker + wasm copying), asset-path resolution as designed.
- `package.json` publish layout: `exports`, `files`, `sideEffects`, `prepublishOnly`.
- The five example pages + `examples/build-time/` script and its test.
- Playwright smoke test per example page (typesets without console errors; `cdn.html` typesets with no
  module scripts on the page; build-time page has SVG present with JS disabled).
- Package README: three consumption modes (npm, CDN, build-time), asset layout diagram, size table
  (brotli'd musa wasm, verovio wasm, JS) updated by the build script.

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
- No framework-specific example apps (Next/Astro/SvelteKit starters): the recipe + custom element
  cover them until a real user asks.
- No source-maps/debug builds beyond vite defaults; no minification forks.
- No service-worker caching, no prefetch machinery: static assets on a CDN are already the answer.
- No playback button on the examples (prompt 144, deferred).
