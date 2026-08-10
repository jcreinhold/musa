/**
 * DOM typesetting (prompt 141): the MathJax.typesetPromise / mermaid.run
 * analog. Scan a root for musa snippets, typeset each one, and swap the
 * engraved SVG in — with the source kept in the document, because text is
 * canonical on the web too.
 */

import { render } from "./api";
import { getConfig } from "./configure";
import { errorBox } from "./errors";
import { ensureDocumentStyles, shadowStyles } from "./styles";

/** The built-in snippet forms; a configured selector adds to these. */
const BUILT_IN_SELECTOR = 'script[type="text/musa"], musa-score, [data-musa]';
/** Set once a snippet is typeset; the idempotence marker (mermaid's data-processed). */
const PROCESSED = "data-musa-processed";
/** The sibling container a script-tag snippet renders into. */
const RENDERED_CLASS = "musa-rendered";

export interface TypesetOptions {
  /** Re-typeset elements already processed (mermaid's data-processed reset). */
  reprocess?: boolean;
  /** Watch for snippets added later and typeset them as they arrive. */
  watch?: boolean;
}

/**
 * Typeset every snippet under `root`. Idempotent: processed elements are
 * skipped, so calling it after every DOM change is safe; pass
 * `reprocess: true` to force. Concurrent calls queue behind one another.
 */
export function typeset(
  root: Document | Element = document,
  options: TypesetOptions = {},
): Promise<void> {
  const run = queue.then(() => typesetNow(root, options));
  queue = run.catch(() => undefined);
  return run;
}

let queue: Promise<void> = Promise.resolve();

async function typesetNow(root: Document | Element, options: TypesetOptions): Promise<void> {
  const config = getConfig();
  const doc = root instanceof Document ? root : (root.ownerDocument ?? document);
  ensureDocumentStyles(doc);
  const elements = scan(root, config.selector, options.reprocess ?? false);
  const done: Element[] = [];
  for (const element of elements) {
    try {
      await typesetOne(element);
      done.push(element);
    } catch (error) {
      // One failing snippet never blocks its siblings: report it, box it,
      // move on (mermaid's per-diagram isolation).
      config.onError?.(error as Error, element);
      const target = targetFor(element);
      target.replaceChildren(errorBox(doc, element.textContent ?? "", []));
      element.setAttribute(PROCESSED, "");
    }
  }
  config.onTypeset?.(done);
  if (options.watch) observe(root, options);
}

/** Every unprocessed snippet under the root, in document order. */
function scan(root: Document | Element, selector: string | undefined, reprocess: boolean): Element[] {
  const wanted = selector === undefined ? BUILT_IN_SELECTOR : `${BUILT_IN_SELECTOR}, ${selector}`;
  const found = new Set<Element>();
  if (root instanceof Element && root.matches(wanted)) found.add(root);
  for (const element of root.querySelectorAll(wanted)) found.add(element);
  if (reprocess) {
    for (const element of found) element.removeAttribute(PROCESSED);
  }
  return [...found].filter((element) => !element.hasAttribute(PROCESSED));
}

/** Typeset one snippet: render, then put the outcome where it belongs. */
async function typesetOne(element: Element): Promise<void> {
  const source = element.textContent ?? "";
  const result = await render(source);
  const doc = element.ownerDocument ?? document;
  const target = targetFor(element);
  if (result.diagnostics.some((diagnostic) => diagnostic.severity === "error")) {
    target.replaceChildren(errorBox(doc, source, result.diagnostics));
  } else if (result.svg === "") {
    // Material declares and does not sound; that is an empty state, not an
    // error box.
    const empty = doc.createElement("span");
    empty.className = "musa-empty";
    empty.textContent = "no score";
    target.replaceChildren(empty);
  } else {
    const parsed = new DOMParser().parseFromString(result.svg, "image/svg+xml");
    target.replaceChildren(doc.importNode(parsed.documentElement, true));
  }
  element.setAttribute(PROCESSED, "");
}

/**
 * Where a snippet's rendered content goes:
 * - `<musa-score>` keeps its source in the light DOM (hidden by the
 *   stylesheet) and renders into its shadow root;
 * - a `text/musa` script tag is untouched and gains a sibling container;
 * - anything else renders into itself, replacing its text.
 */
function targetFor(element: Element): Element | ShadowRoot {
  if (element.tagName === "MUSA-SCORE") {
    const host = element as HTMLElement;
    const doc = element.ownerDocument ?? document;
    const shadow = host.shadowRoot ?? host.attachShadow({ mode: "open" });
    // The styles live beside the content; the content div is what gets
    // replaced, so the styles survive every re-typeset.
    if (shadow.firstChild === null) {
      const content = doc.createElement("div");
      content.className = "musa-content";
      shadow.append(shadowStyles(doc), content);
    }
    return shadow.querySelector(".musa-content") ?? shadow;
  }
  if (element.tagName === "SCRIPT") {
    const sibling = element.nextElementSibling;
    if (sibling?.classList.contains(RENDERED_CLASS)) return sibling;
    const container = (element.ownerDocument ?? document).createElement("div");
    container.className = RENDERED_CLASS;
    element.after(container);
    return container;
  }
  element.classList.add(RENDERED_CLASS);
  return element;
}

/**
 * The opt-in observer (prompt 141 rule 6): one per typeset root, re-running
 * the scan as nodes arrive. Text changes inside processed elements are not
 * tracked — that is what `reprocess` is for.
 */
const watchers = new WeakMap<Document | Element, MutationObserver>();

function observe(root: Document | Element, options: TypesetOptions): void {
  if (watchers.has(root)) return;
  const observer = new MutationObserver(() => {
    void typeset(root, { ...options, watch: false });
  });
  observer.observe(root instanceof Document ? root.documentElement : root, {
    childList: true,
    subtree: true,
  });
  watchers.set(root, observer);
}
