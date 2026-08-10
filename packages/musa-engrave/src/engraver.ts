/**
 * The render layer's entire surface to the rest of the UI
 * (docs/interface/02-engraving.md §1).
 *
 * No component outside this module touches a Verovio toolkit, an MEI string,
 * or a raw SVG string. Rust owns MEI; this is a projection, not a model.
 */

import { DEFAULT_LAYOUT } from "./options";
import type { Box, Layout, LayoutOptions, PageSvg, Request, Response } from "./protocol";
import type { EngraveOutcome } from "./core";

export type { Box, Layout, LayoutOptions, PageSvg };

export interface Engraver {
  load(mei: string, revision: number, options?: Partial<LayoutOptions>): Promise<Layout>;
  page(n: number): Promise<PageSvg>;
  relayout(options: Partial<LayoutOptions>): Promise<Layout>;
  /** The page an event was engraved on, for scroll-to and anchoring. */
  locate(eventId: string): Promise<number | null>;
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
      switch (response.kind) {
        case "error":
          pending.reject(new Error(response.message));
          break;
        case "layout":
          (pending.resolve as (value: Layout) => void)(response.layout);
          break;
        case "page":
          (pending.resolve as (value: PageSvg) => void)(response.page);
          break;
        case "located":
          (pending.resolve as (value: number | null) => void)(response.page);
          break;
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

  locate(eventId: string): Promise<number | null> {
    return this.#send<number | null>({
      kind: "locate",
      generation: this.#generation,
      eventId,
    });
  }

  dispose(): void {
    this.#worker.terminate();
    this.#pending.clear();
  }
}

/**
 * The same engraver without a worker (prompt 140): Node has no `Worker`, and
 * the build-time recipe and the unit tests run there. The lifecycle and the
 * supersede rules are `core.ts`'s, unchanged; only the transport differs.
 * Errors reject rather than arriving as error messages, which is the same
 * shape the worker's client already handles.
 */
class LocalEngraver implements Engraver {
  #generation = 0;
  #options: LayoutOptions = { ...DEFAULT_LAYOUT };

  // A dynamic import, not a static one: `core.ts` pulls in Verovio, and the
  // main thread of a browser app must never carry it (the desktop's initial
  // bundle grew by the whole toolkit when this was a static import).
  async #engrave(request: Request): Promise<EngraveOutcome> {
    const { engrave } = await import("./core");
    return engrave(request);
  }

  async load(mei: string, revision: number, options?: Partial<LayoutOptions>): Promise<Layout> {
    if (options) this.#options = { ...this.#options, ...options };
    this.#generation = Math.max(this.#generation, revision) + 1;
    const outcome = await this.#engrave({
      kind: "load",
      generation: this.#generation,
      mei,
      options: this.#options,
    });
    if (outcome.kind !== "layout") throw new Error(`engrave: expected a layout, got ${outcome.kind}`);
    return outcome.layout;
  }

  async relayout(options: Partial<LayoutOptions>): Promise<Layout> {
    this.#options = { ...this.#options, ...options };
    this.#generation += 1;
    const outcome = await this.#engrave({
      kind: "relayout",
      generation: this.#generation,
      options: this.#options,
    });
    if (outcome.kind !== "layout") throw new Error(`engrave: expected a layout, got ${outcome.kind}`);
    return outcome.layout;
  }

  async page(n: number): Promise<PageSvg> {
    const outcome = await this.#engrave({ kind: "page", generation: this.#generation, page: n });
    if (outcome.kind !== "page") throw new Error(`engrave: expected a page, got ${outcome.kind}`);
    return outcome.page;
  }

  async locate(eventId: string): Promise<number | null> {
    const outcome = await this.#engrave({ kind: "locate", generation: this.#generation, eventId });
    if (outcome.kind !== "located") throw new Error(`engrave: expected a location, got ${outcome.kind}`);
    return outcome.page;
  }

  dispose(): void {
    // Nothing to terminate: the toolkit is process state and stays warm.
  }
}

/**
 * An engraver for the platform we are on: a worker where workers exist, the
 * same rules in-process where they do not (Node).
 */
export function createEngraver(): Engraver {
  return typeof Worker === "undefined" ? new LocalEngraver() : new WorkerEngraver();
}
