import type { LayoutOptions } from "musa-engrave";

/** One labelled place in the snippet. Spans are byte offsets in the source. */
export interface MusaLabel {
  start: number;
  end: number;
  text: string;
  primary: boolean;
}

/** A compiler diagnostic, as it crossed the wasm boundary. */
export interface MusaDiagnostic {
  severity: "error" | "warning";
  code: string;
  message: string;
  labels: MusaLabel[];
  help?: string;
  note?: string;
}

/** What one snippet rendered to. `svg` and `mei` are empty on failure. */
export interface RenderResult {
  svg: string;
  mei: string;
  diagnostics: MusaDiagnostic[];
}

export interface RenderOptions {
  /** Forwarded to the engraver; the continuous-snippet defaults stay inside. */
  layout?: Partial<LayoutOptions>;
}
