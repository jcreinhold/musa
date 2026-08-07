/**
 * Verovio, in a worker, forever.
 *
 * docs/interface/02-engraving.md §2: laying out a 100-bar orchestral score
 * takes hundreds of milliseconds, and on the main thread that is a frozen
 * window, a dropped playhead, and dropped keystrokes. The toolkit is
 * instantiated once here and kept alive; `loadData` runs once per revision
 * and `renderToSVG` once per page.
 *
 * Every request carries a generation token. A page request whose generation
 * is older than the newest layout is abandoned rather than computed — without
 * this a fast typist sees pages arrive out of order.
 */

import createVerovioModule from "verovio/wasm";
import { VerovioToolkit } from "verovio/esm";

import { verovioOptions } from "./options";
import type { Request, Response } from "./protocol";
import { pageBox, sanitize } from "./sanitize";

/**
 * The one toolkit, as a promise rather than as a value: several messages can
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

self.onmessage = async (event: MessageEvent<Request & { id: number }>) => {
  const request = event.data;
  const reply = (response: Response) => self.postMessage(response);
  try {
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
        reply({
          kind: "layout",
          id: request.id,
          layout: {
            pages,
            staffSpace: staffSpaceOf(first),
            ids,
            box: pageBox(first),
            generation: request.generation,
          },
        });
        break;
      }
      case "page": {
        // A superseded page render is abandoned rather than computed — the
        // client has already asked for its replacement — but it is still
        // answered, because a request that never resolves would strand the
        // caller waiting for a page that is never coming.
        if (request.generation < newest) {
          reply({
            kind: "page",
            id: request.id,
            page: {
              page: request.page,
              svg: "",
              box: { width: 0, height: 0 },
              generation: request.generation,
            },
          });
          return;
        }
        const svg = sanitize(verovio.renderToSVG(request.page));
        reply({
          kind: "page",
          id: request.id,
          page: { page: request.page, svg, box: pageBox(svg), generation: request.generation },
        });
        break;
      }
      case "locate": {
        const page = verovio.getPageWithElement(request.eventId);
        reply({ kind: "located", id: request.id, page: page > 0 ? page : null });
        break;
      }
    }
  } catch (error) {
    reply({ kind: "error", id: request.id, message: String(error) });
  }
};
