/**
 * Verovio, in a worker, forever.
 *
 * docs/interface/02-engraving.md §2: laying out a 100-bar orchestral score
 * takes hundreds of milliseconds, and on the main thread that is a frozen
 * window, a dropped playhead, and dropped keystrokes. The toolkit is
 * instantiated once here and kept alive; `loadData` runs once per revision
 * and `renderToSVG` once per page.
 *
 * Every request carries a generation token. A request whose generation is
 * older than the newest load is answered but marked, and the client drops it
 * — without this a fast typist sees pages arrive out of order.
 */

import createVerovioModule from "verovio/wasm";
import { VerovioToolkit } from "verovio/esm";

import { verovioOptions } from "./options";
import type { Request, Response } from "./protocol";
import { identifiers, pageBox, sanitize } from "./sanitize";

let toolkit: VerovioToolkit | undefined;
let newest = 0;

async function ready(): Promise<VerovioToolkit> {
  if (!toolkit) {
    const module = await createVerovioModule();
    toolkit = new VerovioToolkit(module);
  }
  return toolkit;
}

/** One staff space in page units, read back from the laid-out page. */
function staffSpaceOf(svg: string): number {
  // Verovio draws the five staff lines as `<path d="M0 450 L… 450" …>`; the
  // gap between the first two is one staff space, in the page's own units.
  const lines = [...svg.matchAll(/<path d="M[\d.]+ ([\d.]+) L[\d.]+ \1"/g)].map((match) => Number(match[1]));
  for (let index = 1; index < lines.length; index += 1) {
    const gap = lines[index] - lines[index - 1];
    if (gap > 0) return gap;
  }
  return 180;
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
            ids: identifiers(first).filter((id) => id.startsWith("event-")),
            generation: request.generation,
          },
        });
        break;
      }
      case "page": {
        // A superseded page render is abandoned rather than computed: the
        // client has already asked for its replacement.
        if (request.generation < newest) return;
        const svg = sanitize(verovio.renderToSVG(request.page));
        reply({
          kind: "page",
          id: request.id,
          page: { page: request.page, svg, box: pageBox(svg), generation: request.generation },
        });
        break;
      }
    }
  } catch (error) {
    reply({ kind: "error", id: request.id, message: String(error) });
  }
};
