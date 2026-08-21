---
id: 195
slug: web-package-scaffold
status: done
depends_on: [193, 194]
phase: 5
---

# `@musa/web`: Scaffold and the Low-Level API

## Task

Create `packages/musa-web`, published as `@musa/web`: the ESM-only npm package that compiles musa source to engraved SVG
in a browser or Node. This prompt delivers the package scaffold and the low-level API only — `parse` and `render`, the
mermaid.parse/mermaid.render analogs — with no DOM scanning. The MathJax-style layer is prompt 196; the low-level API
must be complete enough that 149 is a thin layer over it, not a second implementation.

## Read

- Prompts 193 (the wasm artifact this package loads) and 147 (`musa-engrave`, the engraver it drives).
- mermaid's core API shape (`parse`, `render`, `run`): three layers, each a client of the one below.
- `packages/musa-web/wasm/` (built by `scripts/build-wasm.sh`) — the `--target web` artifact with its explicit `init()`.
- The 2025 packaging baseline: ESM-only, `"type": "module"`, an `exports` map with `types`/`import` conditions, vite 6
  library mode, vitest. Verovio usage per `verovio/esm` + `verovio/wasm` (`createVerovioModule`), already wrapped inside
  `musa-engrave`.

## Design

```ts
// @musa/web — the whole public surface of this prompt.

export interface MusaDiagnostic {
  severity: "error" | "warning";
  code: string;
  message: string;
  labels: { start: number; end: number; primary: boolean; text: string }[];
  help?: string;
  note?: string;
}

export interface RenderResult {
  svg: string;              // empty when there is nothing to engrave
  mei: string;              // empty on compile failure — callers may inspect or persist it
  diagnostics: MusaDiagnostic[];
}

/** Validate only. The mermaid.parse analog; never initializes the engraver. */
export function parse(source: string): Promise<MusaDiagnostic[]>;

/** Compile and engrave one self-contained snippet. The mermaid.render analog. */
export function render(source: string, options?: RenderOptions): Promise<RenderResult>;

export interface RenderOptions {
  layout?: Partial<LayoutOptions>;   // forwarded to musa-engrave; defaults stay inside
}
```

Loading is lazy and shared: the first call initializes the musa wasm module (explicit `init(wasmUrl)`, URL resolved per
*Asset loading* below) and the first `render` starts the `musa-engrave` worker; both are cached module-level promises so
concurrent calls coalesce, the pattern the desktop worker already uses. `parse` never pays for the engraver; a page that
only validates never downloads Verovio (~25 MB unpacked — laziness is a requirement, not a nicety).

**Asset loading.** Two resolution rules, in order: an explicit `configure({ wasmUrl, verovioUrl? })` call wins;
otherwise the package derives URLs from its own module URL (`new URL("musa.wasm", import.meta.url)`) — bundlers rewrite
this correctly and CDN serving needs no configuration. `configure` is the whole configuration surface of this prompt;
DOM-level options arrive with prompt 196.

**Errors.** A source with error diagnostics resolves with `svg: ""`, `mei: ""`, and the diagnostics — an invalid score
is a result, not an exception. Exceptions are reserved for the environment failing (wasm fetch fails, worker cannot
start) and carry the URL that failed.

**Node.** `render` works in Node (Verovio runs there; the worker module degrades to in-process when `Worker` is
undefined — `musa-engrave` gains this fallback here, with tests, because the desktop app never needed it). This is what
makes vitest meaningful without a browser and what prompt 198's build-time recipe uses.

The MEI and SVG are passed through verbatim. This package never edits either string: Rust owns MEI; the strings are
projections, not models.

## Target

- `packages/musa-web/`: `package.json` (`@musa/web`, ESM-only, exports map, `files: ["dist", "wasm"]`), `vite.config.ts`
  (library mode), `tsconfig.json`, `src/{index.ts,api.ts,assets.ts,configure.ts}`.
- Wasm loading: `scripts/build-wasm.sh` output consumed from `packages/musa-web/wasm/`; `.gitignore` excludes the built
  artifacts, CI builds them.
- `musa-engrave`: in-process fallback when `Worker` is undefined, behind the same `Engraver` interface, with its own
  unit test.
- Vitest suite: `render("examples/canon.musa" text)` returns SVG containing an `event-` id and the MEI; a broken snippet
  resolves with diagnostics and empty strings; `parse` on the same input returns identical diagnostics without touching
  the engraver (asserted via a worker-spy); concurrent `render` calls share one module init; material-only source
  returns empty strings and no diagnostics.
- `README.md` for the package: the two functions, asset configuration, the laziness guarantee.

## Check

```sh
bash scripts/build-wasm.sh
pnpm install && pnpm --filter @musa/web test
pnpm --filter @musa/web build
cd apps/musa-desktop/ui && pnpm test        # 147's extraction is undisturbed
```

Commit as `Scaffold @musa/web with the low-level parse/render API`.

## Stop

- No DOM scanning, custom elements, `window.MusaWeb`, MutationObserver, or error boxes (prompt 196).
- No provenance callbacks or highlighting (prompt 197).
- No CDN/iife build, no examples, no publish workflow (prompt 198).
- No playback, no audio of any kind.
- No pagination in `RenderOptions`: snippets engrave as one continuous system (`modeOptions` already encodes this);
  page-mode web engraving is a measured need, not a speculation.
