/**
 * The low-level API of `@musa/web`: `parse` and `render`, the
 * mermaid.parse/mermaid.render analogs. The DOM layer is a
 * client of this — nothing here knows what a document is.
 */

import {
  CONTINUOUS_WIDTH,
  createEngraver,
  modeOptions,
  type Engraver,
  type LayoutOptions,
} from "musa-engrave";

import type { MusaDiagnostic, RenderOptions, RenderResult } from "./types";
import { compileSnippet, validateSnippet } from "./wasm";

/**
 * A snippet engraves as one system: a continuous layout trimmed to its
 * content, not an A4 leaf.
 */
const SNIPPET_LAYOUT: Partial<LayoutOptions> = {
  pageWidth: CONTINUOUS_WIDTH,
  ...modeOptions("continuous"),
};

/** The shared engraver, created on the first render and never by parse. */
let engraver: Engraver | undefined;
let engraverFactory: () => Engraver = createEngraver;

/**
 * Install a custom engraver factory. Internal: the CDN build uses it to
 * inject its Blob-inlined worker; not part of the documented API.
 */
export function setEngraverFactory(factory: () => Engraver): void {
  engraverFactory = factory;
}

function theEngraver(): Engraver {
  engraver ??= engraverFactory();
  return engraver;
}

/**
 * Validate one snippet: the mermaid.parse analog. Never initializes the
 * engraver — a page that only checks source never downloads Verovio.
 */
export async function parse(source: string): Promise<MusaDiagnostic[]> {
  return validateSnippet(source);
}

/**
 * One engraver means one score at a time: renders queue rather than
 * supersede each other. The queue swallows rejections; the caller's own
 * promise carries its error.
 */
let queue: Promise<unknown> = Promise.resolve();
let revision = 0;

/**
 * Compile and engrave one self-contained snippet: the mermaid.render
 * analog. An invalid score resolves — `svg` and `mei` empty, diagnostics
 * full; rejection is reserved for the environment failing.
 */
export function render(source: string, options?: RenderOptions): Promise<RenderResult> {
  const run = queue.then(() => renderNow(source, options));
  queue = run.catch(() => undefined);
  return run;
}

async function renderNow(source: string, options?: RenderOptions): Promise<RenderResult> {
  const result = await compileSnippet(source);
  if (!result.mei) {
    return { svg: "", mei: "", diagnostics: result.diagnostics };
  }
  const layout = { ...SNIPPET_LAYOUT, ...options?.layout };
  revision += 1;
  const engraved = theEngraver();
  await engraved.load(result.mei, revision, layout);
  const page = await engraved.page(1);
  return { svg: page.svg, mei: result.mei, diagnostics: result.diagnostics };
}
