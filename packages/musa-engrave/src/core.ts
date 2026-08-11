/// <reference path="./types/verovio.d.ts" />
/**
 * The Verovio lifecycle and request handling, shared by the worker
 * (`worker.ts`) and the in-process fallback (`engraver.ts`'s
 * `LocalEngraver`). One toolkit, one `newest` generation, one id index —
 * the rules are the same on both sides of the boundary, so they live here
 * once (prompt 147).
 */

import createVerovioModule from "verovio/wasm";
import { VerovioToolkit } from "verovio/esm";

import { verovioOptions } from "./options";
import type { Layout, PageSvg, Request } from "./protocol";
import { pageBox, sanitize } from "./sanitize";

/** What handling one request produced; the transport adds the message id. */
export type EngraveOutcome =
  | { kind: "layout"; layout: Layout }
  | { kind: "page"; page: PageSvg }
  | { kind: "located"; page: number | null };

/**
 * The one toolkit, as a promise rather than as a value: several requests can
 * arrive while the WASM module is still starting, and each of them awaits the
 * same start. Awaiting a bare `undefined` check instead would start a second
 * module per message and leave the last one — which has no data loaded —
 * installed, so every page after the first would render blank.
 */
let starting: Promise<VerovioToolkit> | undefined;
let newest = 0;
/** Every event id in the loaded MEI, in document order. */
let ids: string[] = [];

function ready(): Promise<VerovioToolkit> {
  starting ??= createVerovioModule().then((module) => new VerovioToolkit(module));
  return starting;
}

/** One staff space in page units, read back from the laid-out page. */
function staffSpaceOf(svg: string): number {
  // Verovio draws the five staff lines as `<path d="M0 450 L… 450" …>`; the
  // gap between the first two is one staff space, in the page's own units.
  const lines = [...svg.matchAll(/<path d="M[\d.]+ ([\d.]+) L[\d.]+ \1"/g)].map((match) =>
    Number(match[1]),
  );
  for (let index = 1; index < lines.length; index += 1) {
    const gap = (lines[index] ?? 0) - (lines[index - 1] ?? 0);
    if (gap > 0) return gap;
  }
  return 180;
}

/**
 * The event ids, read from the MEI rather than from a rendered page: the id
 * index covers the whole score, and rendering every page to collect it would
 * defeat the point of rendering pages on demand (§7).
 */
function identifiersOf(mei: string): string[] {
  return [...mei.matchAll(/xml:id="(event-[^"]+)"/g)].map((match) => match[1] ?? "");
}

/** Handle one request against the shared toolkit. */
export async function engrave(request: Request): Promise<EngraveOutcome> {
  const verovio = await ready();
  switch (request.kind) {
    case "load":
    case "relayout": {
      newest = Math.max(newest, request.generation);
      verovio.setOptions(verovioOptions(request.options));
      if (request.kind === "load") {
        verovio.loadData(request.mei);
        ids = identifiersOf(request.mei);
      } else {
        verovio.redoLayout();
      }
      const pages = verovio.getPageCount();
      const first = pages > 0 ? verovio.renderToSVG(1) : "";
      return {
        kind: "layout",
        layout: {
          pages,
          staffSpace: staffSpaceOf(first),
          ids,
          box: pageBox(first),
          generation: request.generation,
        },
      };
    }
    case "page": {
      // A superseded page render is abandoned rather than computed — the
      // client has already asked for its replacement — but it is still
      // answered, because a request that never resolves would strand the
      // caller waiting for a page that is never coming.
      if (request.generation < newest) {
        return {
          kind: "page",
          page: {
            page: request.page,
            svg: "",
            box: { width: 0, height: 0 },
            generation: request.generation,
          },
        };
      }
      const svg = sanitize(verovio.renderToSVG(request.page));
      return {
        kind: "page",
        page: { page: request.page, svg, box: pageBox(svg), generation: request.generation },
      };
    }
    case "locate": {
      const page = verovio.getPageWithElement(request.eventId);
      return { kind: "located", page: page > 0 ? page : null };
    }
  }
}
