/**
 * The render layer's entire surface to the rest of the UI
 * (docs/interface/02-engraving.md §1).
 *
 * No component outside this module touches a Verovio toolkit, an MEI string,
 * or a raw SVG string. Rust owns MEI; this is a projection, not a model.
 */

import { DEFAULT_LAYOUT } from "./options";
import type { Layout, LayoutOptions, PageSvg, Request, Response } from "./protocol";

export type { Layout, LayoutOptions, PageSvg };

export interface Engraver {
  load(mei: string, revision: number, options?: Partial<LayoutOptions>): Promise<Layout>;
  page(n: number): Promise<PageSvg>;
  relayout(options: Partial<LayoutOptions>): Promise<Layout>;
  /** The engraved element for an event id, once a page is in the document. */
  locate(eventId: string): Element | null;
  dispose(): void;
}

interface Pending {
  resolve: (value: never) => void;
  reject: (reason: Error) => void;
}

class WorkerEngraver implements Engraver {
  #worker: Worker;
  #pending = new Map<number, Pending>();
  #next = 0;
  #generation = 0;
  #options: LayoutOptions = { ...DEFAULT_LAYOUT };

  constructor() {
    this.#worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
    this.#worker.onmessage = (event: MessageEvent<Response>) => {
      const response = event.data;
      const pending = this.#pending.get(response.id);
      if (!pending) return;
      this.#pending.delete(response.id);
      if (response.kind === "error") {
        pending.reject(new Error(response.message));
      } else if (response.kind === "layout") {
        (pending.resolve as (value: Layout) => void)(response.layout);
      } else {
        (pending.resolve as (value: PageSvg) => void)(response.page);
      }
    };
  }

  #send<T>(request: Request): Promise<T> {
    const id = (this.#next += 1);
    return new Promise<T>((resolve, reject) => {
      this.#pending.set(id, { resolve: resolve as Pending["resolve"], reject });
      this.#worker.postMessage({ ...request, id });
    });
  }

  /** A new load supersedes every in-flight page render. */
  load(mei: string, revision: number, options?: Partial<LayoutOptions>): Promise<Layout> {
    if (options) this.#options = { ...this.#options, ...options };
    this.#generation = Math.max(this.#generation, revision) + 1;
    return this.#send<Layout>({
      kind: "load",
      generation: this.#generation,
      mei,
      options: this.#options,
    });
  }

  relayout(options: Partial<LayoutOptions>): Promise<Layout> {
    this.#options = { ...this.#options, ...options };
    this.#generation += 1;
    return this.#send<Layout>({ kind: "relayout", generation: this.#generation, options: this.#options });
  }

  page(n: number): Promise<PageSvg> {
    return this.#send<PageSvg>({ kind: "page", generation: this.#generation, page: n });
  }

  locate(eventId: string): Element | null {
    return document.getElementById(eventId);
  }

  dispose(): void {
    this.#worker.terminate();
    this.#pending.clear();
  }
}

export function createEngraver(): Engraver {
  return new WorkerEngraver();
}
