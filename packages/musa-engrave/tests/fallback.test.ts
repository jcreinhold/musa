/**
 * The in-process fallback: Node has no `Worker`, so
 * `createEngraver` must return an engraver that runs the same rules against
 * the same toolkit lifecycle. The supersede rule itself is tested at the
 * core, where generations are explicit and no timing is involved.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { engrave } from "../src/core";
import { createEngraver } from "../src/engraver";
import { DEFAULT_LAYOUT } from "../src/options";

const MEI = readFileSync(
  fileURLToPath(new URL("../fixtures/twinkle.mei", import.meta.url)),
  "utf8",
);

describe("the in-process engraver", () => {
  it("is the one Node gets", () => {
    expect(typeof Worker).toBe("undefined");
  });

  it("lays out a real score and renders its pages, in order", async () => {
    const engraver = createEngraver();
    const layout = await engraver.load(MEI, 1, DEFAULT_LAYOUT);
    expect(layout.pages).toBeGreaterThan(0);
    expect(layout.ids.some((id) => id.startsWith("event-"))).toBe(true);
    const page = await engraver.page(1);
    expect(page.svg).toContain("event-");
    expect(page.box.width).toBeGreaterThan(0);
  }, 60_000);

  it("locates an engraved event", async () => {
    const engraver = createEngraver();
    const layout = await engraver.load(MEI, 1, DEFAULT_LAYOUT);
    const first = layout.ids[0];
    expect(first).toBeDefined();
    expect(await engraver.locate(first ?? "")).toBe(1);
    expect(await engraver.locate("event-does-not-exist")).toBeNull();
  }, 60_000);
});

describe("the supersede rule", () => {
  it("answers an abandoned page with an empty one rather than stranding it", async () => {
    await engrave({
      kind: "load",
      generation: 5,
      mei: MEI,
      options: DEFAULT_LAYOUT,
    });
    const stale = await engrave({ kind: "page", generation: 4, page: 1 });
    if (stale.kind !== "page") throw new Error("expected a page outcome");
    expect(stale.page.svg).toBe("");
    const current = await engrave({ kind: "page", generation: 5, page: 1 });
    if (current.kind !== "page") throw new Error("expected a page outcome");
    expect(current.page.svg).toContain("event-");
  }, 60_000);
});
