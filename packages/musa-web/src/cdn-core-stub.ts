/**
 * The CDN build aliases `core.ts` to this stub for the main thread: the
 * engraver always runs in the Blob-inlined worker there, so the in-process
 * path is unreachable — and Verovio must not ride along in the main file.
 */

import type { EngraveOutcome } from "../../musa-engrave/src/core";
import type { Request } from "../../musa-engrave/src/protocol";

export type { EngraveOutcome };

export function engrave(_request: Request): Promise<EngraveOutcome> {
  throw new Error("@musa/web CDN build: engraving runs in the inlined worker");
}
