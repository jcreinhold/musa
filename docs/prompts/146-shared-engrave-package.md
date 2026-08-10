---
id: 146
slug: shared-engrave-package
status: done
depends_on: [20, 22]
phase: 5
---

# Extract the Worker Engraver as a Shared Package

## Task

Move `apps/musa-desktop/ui/src/lib/engrave/` into `packages/musa-engrave`, a workspace package both the
desktop UI and the web package (prompt 147) depend on. The engraver is provenance-critical — it owns the
`Engraver` interface, the worker protocol, the rastral-size derivation, and SVG sanitization — and two
copies of it would drift. This prompt changes *where the code lives and how it is built*, not what it
does: the desktop app's behavior and goldens must not move.

## Read

- `apps/musa-desktop/ui/src/lib/engrave/` — `worker.ts`, `engraver.ts`, `options.ts`, `protocol.ts`,
  `sanitize.ts`, and `apps/musa-desktop/ui/src/types/verovio.d.ts`. The module doc on `engraver.ts` is
  the design rule: "No component outside this module touches a Verovio toolkit, an MEI string, or a raw
  SVG string. Rust owns MEI; this is a projection, not a model."
- `docs/interface/02-engraving.md` §1, §4 — the surface and the layout defaults the package must keep.
- `apps/musa-desktop/ui/package.json`, `pnpm-workspace.yaml`, `vite.config.ts` — the tooling this
  extraction rearranges.

## Design

`packages/musa-engrave` is an **internal** package: `"name": "musa-engrave"`, `"private": true`,
`"type": "module"`, no build step — its `exports` map points at the TypeScript source and dependents
bundle it through their own vite builds (the standard pnpm-workspace pattern; publishing is prompt 150's
concern, and only for `@musa/web`, not this package). Its public surface is exactly the current module
surface, no wider:

```ts
// packages/musa-engrave/src/index.ts — re-export curated, not globbed
export type { Engraver } from "./engraver";
export { createEngraver } from "./engraver";      // the worker constructor, renamed from
                                                  // `new WorkerEngraver()` at the call sites
export type { Box, Layout, LayoutOptions, PageSvg } from "./protocol";
export { DEFAULT_LAYOUT, modeOptions, type ViewMode } from "./options";
```

`verovio` becomes a `peerDependency` with a devDependency pin: one version across the workspace,
chosen by the app, never duplicated. The worker keeps `new Worker(new URL("./worker.ts", import.meta.url),
{ type: "module" })` — vite resolves this in dependents' builds without configuration.

Workspace layout: a **root** `pnpm-workspace.yaml` covers `packages/*` and `apps/musa-desktop/ui`; the
nested `apps/musa-desktop/ui/pnpm-workspace.yaml` is deleted and the lockfile regenerates at the root.
The desktop UI's `package.json` gains `"musa-engrave": "workspace:*"` and loses nothing else; its source
changes are import-path edits only (`$lib/engrave/...` → `musa-engrave`). Tauri, vite dev, vitest, and
Playwright all keep working from `apps/musa-desktop/ui` via pnpm's recursive/filtered invocation.

If the extraction forces any change beyond import paths (a config assumption, a relative fixture path,
a vite alias), that change is a defect in the module boundary: fix the boundary in the same commit, and
say so in the commit message. Screenshot goldens are the arbiter — they must not be regenerated.

## Target

- `packages/musa-engrave/`: `package.json`, `tsconfig.json`, `src/` with the five modules and the
  verovio type declarations, unit tests moved alongside (vitest at the package).
- Root `pnpm-workspace.yaml`; deleted nested workspace file; regenerated root lockfile.
- `apps/musa-desktop/ui`: import-path edits only; `src/lib/engrave/` and `src/types/verovio.d.ts`
  deleted; `package.json` updated.
- `docs/interface/02-engraving.md` §1 gains one sentence naming `packages/musa-engrave` as the module's
  home, so the spec and the code agree.

## Check

```sh
pnpm install                       # root lockfile
pnpm --filter musa-engrave test
cd apps/musa-desktop/ui && pnpm run check && pnpm test   # vitest + playwright goldens unchanged
pnpm --filter musa-ui build
```

Commit as `Extract the worker engraver as packages/musa-engrave`.

## Stop

- No new engraver features, options, or API widening "while we're in there" — this is a move.
- No published package, no build artifacts in the repo, no `dist/` for an internal package.
- No changes to `Score.svelte`, the protocol's message shapes, or the layout defaults.
- Do not flatten the desktop app into the root package or rename `musa-ui`; the workspace exists to
  share `packages/*`, not to reorganize the app.
