import { getConfig } from "./configure";

/**
 * The classic script tag's own URL, captured at module init. Only set for
 * the CDN build (a classic script); module scripts leave it undefined.
 */
let scriptBase: string | undefined;

/** Capture the script URL at import time, when `document.currentScript` is still set. */
export function noteScriptBase(): void {
  if (typeof document === "undefined") return;
  const current = document.currentScript;
  if (current instanceof HTMLScriptElement && current.src !== "") scriptBase = current.src;
}

/**
 * Where the musa wasm lives, in order of precedence: an explicit
 * `configure({ wasmUrl })`; an `assetsPath` (the CDN convention, served
 * beside the script otherwise); the script's own directory in the classic
 * (CDN) build; the `wasm/` directory beside this module. Bundlers rewrite
 * the `new URL` in the last case; static serving needs no configuration.
 */
export function musaWasmUrl(): string {
  const config = getConfig();
  if (config.wasmUrl !== undefined) return config.wasmUrl;
  if (config.assetsPath !== undefined) {
    return `${config.assetsPath.replace(/\/+$/, "")}/musa_wasm_bg.wasm`;
  }
  if (scriptBase !== undefined) return new URL("musa_wasm_bg.wasm", scriptBase).href;
  return (
    // @vite-ignore inside the constructor: library mode would inline the
    // wasm as base64 into the JS; it stays a separate, cacheable file
    // beside the package instead.
    new URL(/* @vite-ignore */ "../wasm/musa_wasm_bg.wasm", import.meta.url).href
  );
}
