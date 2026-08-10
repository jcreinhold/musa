/**
 * Verovio 6 ships no type declarations. This is the slice of its surface the
 * engrave module uses — narrow on purpose, so an unreviewed toolkit call is a
 * type error rather than a discovery.
 */

declare module "verovio/wasm" {
  const createVerovioModule: () => Promise<unknown>;
  export default createVerovioModule;
}

declare module "verovio/esm" {
  export class VerovioToolkit {
    constructor(module: unknown);
    setOptions(options: Record<string, unknown>): void;
    loadData(data: string): boolean;
    redoLayout(): void;
    getPageCount(): number;
    renderToSVG(page: number): string;
    /** 1-based page holding an element, or 0 when it holds none. */
    getPageWithElement(xmlId: string): number;
  }
}
