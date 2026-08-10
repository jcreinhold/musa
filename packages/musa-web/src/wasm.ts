/**
 * The musa wasm module's lifecycle: one shared, coalesced initialization
 * (several calls during startup await the same start, never start twice).
 * The engraver's lifecycle is `musa-engrave`'s own; a caller that only
 * validates never pays for it.
 */

import __wbg_init, { typeset as wasmTypeset, validate as wasmValidate } from "../wasm/musa_wasm.js";

import { musaWasmUrl } from "./assets";
import type { MusaDiagnostic } from "./types";

/** The `TypesetResult` the wasm shell returns (crates/musa-wasm). */
export interface TypesetResult {
  mei: string | null;
  diagnostics: MusaDiagnostic[];
}

let starting: Promise<unknown> | undefined;

function module_(): Promise<unknown> {
  starting ??= start();
  return starting;
}

async function start(): Promise<unknown> {
  const url = musaWasmUrl();
  try {
    if (typeof window === "undefined" && url.startsWith("file:")) {
      // Node cannot fetch a file: URL; read the bytes instead.
      const { readFile } = await import("node:fs/promises");
      const { fileURLToPath } = await import("node:url");
      return await __wbg_init(await readFile(fileURLToPath(url)));
    }
    return await __wbg_init(url);
  } catch (error) {
    // A failed start must not be cached forever: the next call retries.
    starting = undefined;
    throw new Error(`@musa/web: could not load the musa wasm from ${url}: ${error}`);
  }
}

/** Compile one snippet to MEI (or to diagnostics). */
export async function compileSnippet(source: string): Promise<TypesetResult> {
  await module_();
  return wasmTypeset(source) as TypesetResult;
}

/** Compile one snippet for its diagnostics only. */
export async function validateSnippet(source: string): Promise<MusaDiagnostic[]> {
  await module_();
  return wasmValidate(source) as MusaDiagnostic[];
}
