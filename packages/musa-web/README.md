# @musa/web

Typeset [musa](https://github.com/jcreinhold/musa) scores in the browser — the MathJax model for a music-notation
language. Source in, engraved SVG out, with every note traceable back to the event that wrote it.

```ts
import { parse, render } from "@musa/web";

// Validate only — never downloads the engraver.
const diagnostics = await parse(source);

// Compile and engrave one snippet.
const { svg, mei, diagnostics } = await render(source);
```

## The two functions

- **`parse(source)`** — validate a snippet, return its diagnostics. The mermaid.parse analog: it initializes only the
  (small) musa compiler wasm, never the Verovio engraver (~25 MB).
- **`render(source, options?)`** — compile and engrave, returning `{ svg, mei, diagnostics }`. The mermaid.render
  analog. Snippets engrave as one continuous system trimmed to the music.

An invalid score is a **result, not an exception**: `svg` and `mei` are empty and `diagnostics` is full. Rejection is
reserved for the environment failing (the wasm cannot be fetched), and the error carries the URL that failed.

## Assets and laziness

Two wasm modules do the work, and each is loaded on first use, coalesced so concurrent calls share one start:

- the musa compiler wasm (~300 KB brotli), loaded by the first `parse` or `render`;
- the Verovio engraver, loaded by the first `render`, in a Web Worker where workers exist and in-process where they do
  not (Node, the build-time recipe).

The musa wasm resolves to the `wasm/` directory beside the package module; to serve it from somewhere else (a bundler
that relocates assets, a CDN):

```ts
import { configure } from "@musa/web";
configure({ wasmUrl: "https://example.invalid/assets/musa_wasm_bg.wasm" });
```

## The id contract

Every note, chord, and rest in the SVG carries the `xml:id` the MEI carried — `event-<hex>` where the hex is the
compiler's `EventId`, with tie pieces suffixed `-t2`, `-t3`, … Strip the suffix and you have the event. This is what
makes the rendered page addressable.

## Interaction

```ts
window.MusaWeb = {
  onEventClick: (eventId, target, context) => {
    // eventId: the compiler's EventId hex, tie-piece suffixes stripped
    // target:  the SVGElement that was hit (the exact piece)
    // context: { element, source } — which score, and its source text
  },
  onEventHover: (eventId, target) => {
    /* id or null on leave */
  },
};
```

Event-mapped elements get the class `musa-event`; `highlight(eventId)` marks every rendered piece of an event — all tie
pieces at once — with `musa-event-active`, and `highlight(null)` clears it. The active colour is one custom property:

```css
musa-score {
  --musa-event-active: #c05621;
}
```

Unmapped elements (barlines, staff lines, text) never fire: there is no "unidentified object" state.

## Node

Both functions work in Node (the engraver takes its in-process path there), which is what the unit tests and the
static-site build-time recipe use.

## Three ways to consume it

**1. npm, with a bundler** — `npm i @musa/web`, then `import { typeset } from "@musa/web"` as above. The musa wasm
resolves beside the package module; bundlers rewrite it, or set `configure({ wasmUrl })`.

**2. CDN, one script tag** — serve `dist-cdn/` (also published as `@musa/web/cdn`):

```html
<script src="musa-web.js" data-musa-autostart></script>
<musa-score> piece "plain" { … } </musa-score>
```

The worker (with the engraver inside) arrives as an inlined Blob — there is no worker URL to configure. The only
external asset is the compiler wasm, which the script resolves from its own directory; serve it from elsewhere with
`window.MusaWeb = { assetsPath: "…" }` before the tag.

```
dist-cdn/
  musa-web.js         ← the one <script src>        (8.7 MB, 2.4 MB gzip)
  musa_wasm_bg.wasm   ← the compiler                (0.93 MB, 0.29 MB brotli)
```

**3. Build time, no client wasm** — for static sites: `examples/build-time/typeset.mjs` renders every `text/musa` block
in a glob of HTML to baked-in SVG using Node (the in-process engraver path), so the deployed page needs no JavaScript at
all. `npm i @musa/web`, point the script at your pages.

## The examples

`packages/musa-web/examples/` — `blog.html` (prose with excerpts and an inline fragment), `docs.html` (auto-start, the
error box), `dynamic.html` (snippets added after load), `interactive.html` (event callbacks + highlight +
reveal-source), `cdn.html` (the iife build, any static server), `build-time/` (the recipe above). The module examples
run under the package's vite server; `cdn.html` runs from any static file server.
