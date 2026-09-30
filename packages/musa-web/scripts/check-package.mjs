/** Verify the built package from a consumer outside the workspace. */
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = new URL("../", import.meta.url);
const manifest = JSON.parse(readFileSync(new URL("package.json", root), "utf8"));
for (const entry of Object.values(manifest.exports)) {
  for (const path of typeof entry === "string" ? [entry] : Object.values(entry)) {
    assert.ok(existsSync(new URL(path, root)), `missing package export: ${path}`);
  }
}
assert.ok(!manifest.dependencies?.["musa-engrave"], "the bundled engraver must not require a private npm package");

const consumer = mkdtempSync(join(tmpdir(), "musa-web-consumer-"));
try {
  const installed = join(consumer, "node_modules", "@musa", "web");
  mkdirSync(installed, { recursive: true });
  for (const path of ["package.json", ...manifest.files]) {
    cpSync(new URL(path, root), join(installed, path), { recursive: true });
  }
  // Supply only the public peer, never the private workspace engraver.
  symlinkSync(realpathSync(new URL("node_modules/verovio", root)), join(consumer, "node_modules", "verovio"), "dir");
  writeFileSync(
    join(consumer, "consumer.ts"),
    'import { parse, render, type LayoutOptions, type RenderResult } from "@musa/web";\n' +
      "const layout: Partial<LayoutOptions> = {};\n" +
      "const source = 'piece \"plain\" { meter 1/4; score { part p { voice v { | c4/4 } } } }';\n" +
      "const result: Promise<RenderResult> = render(source, { layout });\n" +
      "void result; void parse(source);\n",
  );
  const tsc = fileURLToPath(new URL("node_modules/typescript/bin/tsc", root));
  execFileSync(
    process.execPath,
    [
      tsc,
      "--noEmit",
      "--strict",
      "--target",
      "ES2022",
      "--moduleResolution",
      "bundler",
      "--module",
      "ESNext",
      "consumer.ts",
    ],
    {
      cwd: consumer,
      stdio: "inherit",
    },
  );
  const { parse, render } = await import(pathToFileURL(join(installed, manifest.exports["."].import)).href);
  const source = readFileSync(new URL("../../examples/twinkle.musa", root), "utf8");
  assert.deepEqual(
    (await parse(source)).filter((d) => d.severity === "error"),
    [],
  );
  const result = await render(source);
  assert.deepEqual(
    result.diagnostics.filter((d) => d.severity === "error"),
    [],
  );
  assert.match(result.mei, /<mei/);
  assert.match(result.svg, /<svg/);
  console.log("package exports, isolated consumer types, validation, and engraving: ok");
} finally {
  rmSync(consumer, { recursive: true, force: true });
}
