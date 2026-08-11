/**
 * The CDN entry (prompt 151): the same API as `index.ts`, but the engraver's
 * worker arrives inlined as a Blob — one `<script src>` with no worker URL
 * to configure. `?worker&inline` is vite's first-class mechanism for exactly
 * this: the worker (with Verovio inside) is a base64 string in this file,
 * parsed only when the first render creates the worker.
 */

import EngraveWorker from "musa-engrave/worker?worker&inline";
import { createEngraver } from "musa-engrave";

import { setEngraverFactory } from "./api";

setEngraverFactory(() => createEngraver({ worker: () => new EngraveWorker() }));

export * from "./index";
