/**
 * The laziness guarantee (prompt 148): `parse` must never initialize the
 * engraver — a page that only validates never downloads Verovio's 25 MB.
 * The mock makes an engraver construction a loud failure; if `parse` ever
 * reaches for one, this suite fails.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it, vi } from "vitest";

vi.mock("musa-engrave", () => ({
  CONTINUOUS_WIDTH: 1_000_000,
  createEngraver: () => {
    throw new Error("the engraver must not be created by parse");
  },
  modeOptions: () => ({}),
}));

import { parse } from "../src/index";

const BROKEN = readFileSync(
  fileURLToPath(new URL("../../../examples/broken/bar-too-long.musa", import.meta.url)),
  "utf8",
);

describe("parse", () => {
  it("validates without touching the engraver", async () => {
    const diagnostics = await parse(BROKEN);
    expect(diagnostics.some((d) => d.severity === "error")).toBe(true);
  });
});
