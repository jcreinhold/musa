/**
 * The low-level contract (prompt 140): `render` compiles and engraves,
 * `parse` only validates, an invalid score is a result rather than an
 * exception, and the MEI→SVG id map survives verbatim.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { parse, render } from "../src/index";

const CANON = readFileSync(
  fileURLToPath(new URL("../../../examples/canon.musa", import.meta.url)),
  "utf8",
);
const BROKEN = readFileSync(
  fileURLToPath(new URL("../../../examples/broken/bar-too-long.musa", import.meta.url)),
  "utf8",
);
const MATERIAL = "library {\n    motif rise(root: pitch = c5) {\n        root/4\n        d5/4\n    }\n}\n";

describe("render", () => {
  it("keeps the id contract on a tie-heavy fixture, pieces and all", async () => {
    const tied = readFileSync(
      fileURLToPath(new URL("../../../examples/tuplet-fixture.musa", import.meta.url)),
      "utf8",
    );
    const result = await render(tied);
    const meiIds = new Set([...result.mei.matchAll(/xml:id="([^"]+)"/g)].map((m) => m[1]));
    const svgIds = [...result.svg.matchAll(/\bid="(event-[^"]+)"/g)].map((m) => m[1] ?? "");
    expect(svgIds.some((id) => /-t\d+$/.test(id))).toBe(true);
    for (const id of svgIds) expect(meiIds.has(id)).toBe(true);
  }, 60_000);

  it("compiles and engraves a snippet, preserving the id contract", async () => {
    const result = await render(CANON);
    expect(result.mei).toContain("<mei");
    expect(result.svg).toContain("<svg");
    expect(result.diagnostics.filter((d) => d.severity === "error")).toEqual([]);
    // The contract guard: every engraved event id is an MEI xml:id. If a
    // Verovio upgrade ever rewrites ids, it breaks here, not on a page.
    const meiIds = new Set([...result.mei.matchAll(/xml:id="([^"]+)"/g)].map((m) => m[1]));
    const svgIds = [...result.svg.matchAll(/\bid="(event-[^"]+)"/g)].map((m) => m[1] ?? "");
    expect(svgIds.length).toBeGreaterThan(0);
    for (const id of svgIds) expect(meiIds.has(id)).toBe(true);
  }, 60_000);

  it("resolves, not rejects, for an invalid score", async () => {
    const result = await render(BROKEN);
    expect(result.svg).toBe("");
    expect(result.mei).toBe("");
    expect(result.diagnostics.some((d) => d.severity === "error")).toBe(true);
  });

  it("returns empty output with no errors for material", async () => {
    const result = await render(MATERIAL);
    expect(result.svg).toBe("");
    expect(result.mei).toBe("");
    expect(result.diagnostics.filter((d) => d.severity === "error")).toEqual([]);
  });

  it("serves concurrent renders of different snippets correctly", async () => {
    const [good, bad] = await Promise.all([render(CANON), render(BROKEN)]);
    expect(good.svg).toContain("event-");
    expect(bad.svg).toBe("");
    expect(bad.diagnostics.some((d) => d.severity === "error")).toBe(true);
  }, 60_000);
});

describe("parse", () => {
  it("returns the same diagnostics as render", async () => {
    const fromParse = await parse(BROKEN);
    const fromRender = await render(BROKEN);
    expect(fromParse).toEqual(fromRender.diagnostics);
  });
});
