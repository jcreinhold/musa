import { getConfig } from "./configure";

/**
 * Where the musa wasm lives. An explicit `configure({ wasmUrl })` wins;
 * otherwise the module sits beside this one — bundlers rewrite the `new URL`
 * and static serving of the package needs no configuration at all.
 */
export function musaWasmUrl(): string {
  return (
    getConfig().wasmUrl ??
    // @vite-ignore inside the constructor: library mode would inline the
    // wasm as base64 into the JS; it stays a separate, cacheable file
    // beside the package instead.
    new URL(/* @vite-ignore */ "../wasm/musa_wasm_bg.wasm", import.meta.url).href
  );
}
