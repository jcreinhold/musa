/**
 * The sanitizer's contract (`docs/rules/desktop/02-engraving.md` §3).
 *
 * Dark mode must **re-ink** the score, not filter it, and Origin view must be
 * able to re-ink part of the page without touching the rest. Both depend on
 * exactly one thing: that nothing in the emitted SVG pins a colour the
 * cascade cannot override. These tests run against real Verovio output so the
 * contract is checked against what Verovio actually emits, not against a
 * fixture of what it emitted once.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { beforeAll, describe, expect, it } from "vitest";
import createVerovioModule from "verovio/wasm";
import { VerovioToolkit } from "verovio/esm";

import { DEFAULT_LAYOUT, verovioOptions } from "../src/options";
import { identifiers, pageBox, sanitize } from "../src/sanitize";

const FIXTURES = ["glass-mountain", "counterpoint", "twinkle"] as const;

function fixturePath(name: string): string {
  return fileURLToPath(new URL(`../fixtures/${name}.mei`, import.meta.url));
}

let raw: Record<string, string> = {};

beforeAll(async () => {
  const toolkit = new VerovioToolkit(await createVerovioModule());
  toolkit.setOptions(verovioOptions(DEFAULT_LAYOUT));
  for (const name of FIXTURES) {
    toolkit.loadData(readFileSync(fixturePath(name), "utf8"));
    raw[name] = toolkit.renderToSVG(1);
  }
}, 60_000);

describe.each(FIXTURES)("%s", (name) => {
  it("emits the one attribute that defeats theming, before sanitizing", () => {
    // If this ever fails, Verovio changed and the sanitizer may be obsolete —
    // which is worth knowing loudly rather than discovering as a grey score.
    expect(raw[name]).toContain('color="black"');
  });

  it("leaves no pinned colour for the cascade to lose to", () => {
    const clean = sanitize(raw[name]);
    expect(clean).not.toContain('color="black"');
    expect(clean).not.toMatch(/(fill|stroke)="(black|#000|#000000)"/i);
  });

  it("drops the Times fallback so score text takes Academico", () => {
    expect(sanitize(raw[name])).not.toContain("Times");
  });

  it("preserves every id verbatim — the id map is the engraver's contract", () => {
    expect(identifiers(sanitize(raw[name]))).toStrictEqual(
      identifiers(raw[name]),
    );
  });

  it("engraves at least one event id the editor can select by", () => {
    const events = identifiers(sanitize(raw[name])).filter((id) =>
      id.startsWith("event-"),
    );
    expect(events.length).toBeGreaterThan(0);
  });

  it("reports the page's own coordinate system for the overlay layer", () => {
    const box = pageBox(sanitize(raw[name]));
    expect(box.width).toBeGreaterThan(0);
    expect(box.height).toBeGreaterThan(0);
  });
});

describe("sanitize", () => {
  it("leaves a colour the MEI itself asked for alone", () => {
    const svg = '<g fill="#A83A2B" stroke="currentColor"><path d="M0 0"/></g>';
    expect(sanitize(svg)).toBe(svg);
  });

  it("removes color=black only from the definition-scale element", () => {
    const svg =
      '<svg class="definition-scale" color="black" viewBox="0 0 100 50">' +
      '<g class="note" color="black"/></svg>';
    const clean = sanitize(svg);
    expect(clean).toContain(
      '<svg class="definition-scale" viewBox="0 0 100 50">',
    );
    expect(clean).toContain('<g class="note" color="black"/>');
  });
});
