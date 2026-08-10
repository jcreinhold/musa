/**
 * Copy the musa compiler wasm beside the CDN bundle (prompt 143): it is the
 * one external asset `musa-web.js` needs, and the script resolves it from
 * its own directory.
 */
import { copyFileSync, mkdirSync } from "node:fs";

mkdirSync("dist-cdn", { recursive: true });
copyFileSync("wasm/musa_wasm_bg.wasm", "dist-cdn/musa_wasm_bg.wasm");
console.log("dist-cdn/musa_wasm_bg.wasm copied");
