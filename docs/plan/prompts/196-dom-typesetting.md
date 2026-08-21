---
id: 196
slug: dom-typesetting
status: done
depends_on: [195]
phase: 5
---

# The MathJax Layer: DOM Typesetting and `<musa-score>`

## Task

Give `@musa/web` its MathJax.typesetPromise / mermaid.run analog: scan a document for musa snippets, typeset them, and
swap engraved SVG into the page — plus the `<musa-score>` custom element, the config-before-load convention, visible
error boxes, and an opt-in MutationObserver for dynamic pages. After this prompt, embedding musa in a web page is: one
script, markup, done.

## Read

- Prompt 195's low-level `parse`/`render`; this prompt is a client of it, nothing reimplemented.
- MathJax's web model: `window.MathJax = {...}` before the script loads, `typesetPromise()`, error boxes inline where
  the math was. Mermaid's model: `mermaid.run({querySelector})`, `data-processed` marking, `initialize()`.
- `docs/rules/desktop/02-engraving.md` for the engraving defaults the output inherits; error-box styling is new web
  territory — keep it visually quiet (source excerpt, underlined span, message; no chrome).

## Design

```ts
// Added to @musa/web's surface in this prompt.

export interface TypesetOptions {
  /** Re-typeset elements already processed (mermaid's data-processed reset). */
  reprocess?: boolean;
  /** Watch for snippets added later and typeset them as they arrive. */
  watch?: boolean;
}

export interface MusaWebConfig {
  /** CSS selector for additional containers; built-in forms are always found. */
  selector?: string;
  onTypeset?: (elements: Element[]) => void;   // after each batch
  onError?: (error: Error, element: Element | null) => void;
}

export function typeset(root?: Document | Element, options?: TypesetOptions): Promise<void>;
export function configure(config: MusaWebConfig & Parameters<typeof baseConfigure>[0]): void;
```

**Snippet forms**, all equivalent: `<script type="text/musa">…</script>`, `<musa-score>…</musa-score>`, and any element
matching the configured `selector` with a `data-musa` attribute. Inline fragments in prose are
`<musa-score inline>…</musa-score>` — the display-math/inline-math distinction is one attribute, not two element names.

**Processing rules** (the MathJax/mermaid contract, adapted):

1. Each processed element is marked `data-musa-processed` and skipped thereafter, so `typeset()` is idempotent and safe
   to call after every DOM mutation; `reprocess: true` resets.
2. Rendered SVG replaces the snippet's *visual* content; the source stays in the document. For `<musa-score>` the source
   text remains in the light DOM (visually hidden) and the SVG goes into a shadow root — the page's DOM remains a
   projection of the text, the source-is-canonical rule holding on the web exactly as it does in the desktop app.
   `script[type=text/musa]` snippets gain a sibling container; the script tag itself is untouched.
3. A snippet with error diagnostics gets an error box **where the score would be**: the offending source excerpt with
   the primary label's span underlined, the message, and the stable code. Byte spans are converted to line/column here,
   at the display layer, from the source the page already holds. Never silent, never a console-only failure.
4. Material-only source (no score) renders a subtle empty state, not an error.
5. Concurrent `typeset()` calls queue behind one another; each batch fires `onTypeset` once. One failing snippet never
   blocks its siblings (mermaid's per-diagram isolation).
6. `watch: true` installs one MutationObserver per `typeset` root; it re-runs the scan, it does not track text changes
   inside already-processed elements (that is `reprocess`).

**Auto-start**, the MathJax convention: if `document.currentScript` has `data-musa-autostart` (or the config object set
before load says so), `typeset(document)` runs on `DOMContentLoaded`. Programmatic users import the module and call
`typeset` themselves; nothing auto-runs without the opt-in.

## Target

- `src/typeset.ts`, `src/elements.ts` (the custom element + registration), `src/errors.ts` (error-box rendering +
  byte→line/column), `src/observer.ts`; all exported through `src/index.ts`.
- Default stylesheet injected once per document (`<style data-musa>`), minimal and overridable.
- Playwright suite (`packages/musa-web/tests/`, fixture server via vite): script-tag and custom-element snippets
  typeset; idempotent re-scan; error box contains message, code, and the right underlined span; `watch` typesets a
  snippet appended after load; one broken snippet does not block a good one; auto-start fires with the attribute and not
  without; source text remains in the DOM after typeset.
- Vitest: byte→line/column conversion against multi-byte UTF-8 (spans are bytes; JS strings are UTF-16 — the conversion
  must be explicit and tested).

## Check

```sh
pnpm --filter @musa/web test          # vitest + playwright
pnpm --filter @musa/web build
```

Commit as `Add DOM typesetting and the musa-score element to @musa/web`.

## Stop

- No provenance interaction (clicks, highlight, locate) — prompt 197.
- No CDN/iife build or examples — prompt 198; Playwright fixtures here are tests, not demos.
- No editing of the source in the page, no CodeMirror, no live re-typeset on typing: the web package renders; it does
  not author.
- No sanitization beyond what `musa-engrave` already does; no new SVG transforms.
- No framework adapters (React/Vue/Svelte wrappers): the custom element is the adapter.
