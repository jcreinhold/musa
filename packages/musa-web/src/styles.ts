/**
 * The package's own styling: minimal, injected once per
 * document and once per shadow root, and deliberately ordinary CSS so an
 * embedder can override every rule. Nothing here sizes the music — that is
 * the engraver's layout, not the stylesheet's.
 */

const SHARED_CSS = `
.musa-event {
  cursor: pointer;
}
/* Classes, not inline styles — embedders re-ink by overriding one custom
   property, exactly as theming re-inks the whole score (02-engraving §3). */
.musa-event-active {
  color: var(--musa-event-active, #2b6cb0);
}
.musa-error {
  border-left: 2px solid #b04040;
  padding: 0.25em 0.75em;
  margin: 0.5em 0;
  font-size: 0.9em;
}
.musa-error-source {
  margin: 0.25em 0;
  overflow-x: auto;
  font-family: ui-monospace, monospace;
}
.musa-error-span {
  text-decoration: underline #b04040 2px;
  text-underline-offset: 2px;
}
.musa-error-message {
  margin: 0.25em 0;
}
.musa-error-code {
  font-family: ui-monospace, monospace;
  font-size: 0.85em;
  opacity: 0.7;
}
`;

/** Rules for containers living in the page (script-tag siblings, [data-musa]). */
const DOCUMENT_CSS = `
.musa-rendered svg {
  max-width: 100%;
  height: auto;
  color: inherit;
}
.musa-empty {
  font-style: italic;
  opacity: 0.6;
}
${SHARED_CSS}
`;

/** Rules for a `<musa-score>` element and inside its shadow root. */
const SHADOW_CSS = `
:host {
  display: block;
}
:host([inline]) {
  display: inline-block;
  vertical-align: middle;
}
/* The host hides its light-DOM source with font-size: 0; rem restores real
   text inside the shadow (em would inherit the zero). */
.musa-content {
  font-size: 1rem;
}
svg {
  max-width: 100%;
  height: auto;
  color: inherit;
}
.musa-empty {
  font-style: italic;
  opacity: 0.6;
}
${SHARED_CSS}
`;


/**
 * The processed `<musa-score>` keeps its source in the light DOM — the page
 * stays a projection of the text — but visually hidden. Zeroing the font
 * size hides the text nodes without moving them; the shadow root's SVG is
 * unaffected.
 */
const ELEMENT_CSS = `
musa-score[data-musa-processed] {
  font-size: 0;
}
`;

let injected: WeakSet<Document> = new WeakSet();

/** Inject the document-level rules once per document. */
export function ensureDocumentStyles(doc: Document): void {
  if (injected.has(doc)) return;
  injected.add(doc);
  const style = doc.createElement("style");
  // Not `data-musa`: that attribute marks snippet containers, and the
  // scanner must never typeset its own stylesheet.
  style.setAttribute("data-musa-styles", "");
  style.textContent = DOCUMENT_CSS + ELEMENT_CSS;
  doc.head.append(style);
}

/** Inject the shadow-root rules into a freshly attached shadow root. */
export function shadowStyles(doc: Document): HTMLStyleElement {
  const style = doc.createElement("style");
  style.textContent = SHADOW_CSS;
  return style;
}
