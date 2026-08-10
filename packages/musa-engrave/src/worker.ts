/// <reference path="./types/verovio.d.ts" />
/**
 * Verovio, in a worker, forever.
 *
 * docs/interface/02-engraving.md §2: laying out a 100-bar orchestral score
 * takes hundreds of milliseconds, and on the main thread that is a frozen
 * window, a dropped playhead, and dropped keystrokes. The toolkit is
 * instantiated once and kept alive; `loadData` runs once per revision and
 * `renderToSVG` once per page. This module is only the transport; the
 * lifecycle and the rules live in `core.ts`, shared with the in-process
 * fallback.
 */

import { engrave } from "./core";
import type { Request, Response } from "./protocol";

self.onmessage = async (event: MessageEvent<Request & { id: number }>) => {
  const request = event.data;
  const reply = (response: Response) => self.postMessage(response);
  try {
    const outcome = await engrave(request);
    reply({ ...outcome, id: request.id });
  } catch (error) {
    reply({ kind: "error", id: request.id, message: String(error) });
  }
};
