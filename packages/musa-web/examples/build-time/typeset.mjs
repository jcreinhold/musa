/**
 * The static-site recipe (prompt 150): typeset every `text/musa` block in a
 * glob of HTML at build time, so the deployed page needs no wasm and no JS
 * at all. Runs in Node, where the engraver takes its in-process path.
 *
 *   node examples/build-time/typeset.mjs
 *
 * In your site: `npm i @musa/web`, import from "@musa/web", and point the
 * paths at your pages.
 */

import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { render } from "../../dist/index.js"; // in your site: "@musa/web"

const PAGES = fileURLToPath(new URL("./pages/", import.meta.url));
const OUT = fileURLToPath(new URL("./out/", import.meta.url));
const SNIPPET = /<script type="text\/musa">([\s\S]*?)<\/script>/g;

mkdirSync(OUT, { recursive: true });

for (const name of readdirSync(PAGES).filter((file) => file.endsWith(".html"))) {
  const html = readFileSync(`${PAGES}${name}`, "utf8");
  const jobs = [];
  const baked = html.replace(SNIPPET, (_match, source) => {
    jobs.push(render(source.trim()));
    return `${jobs.length - 1}`; // placeholder swapped after all renders settle
  });
  const rendered = await Promise.all(jobs);
  const out = baked.replace(/(\d+)/g, (_match, index) => {
    const result = rendered[Number(index)];
    if (result === undefined) throw new Error(`no render for placeholder ${index}`);
    if (result.svg === "") {
      const message = result.diagnostics.map((d) => d.message).join("; ");
      throw new Error(`snippet ${index} in ${name} did not compile: ${message}`);
    }
    return `<div class="musa-rendered">${result.svg}</div>`;
  });
  writeFileSync(`${OUT}${name}`, out);
  console.log(`${name}: ${jobs.length} snippet(s) typeset`);
}
