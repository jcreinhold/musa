/**
 * The contrast floor of `docs/interface/01-visual-language.md` §2, checked
 * over the token file rather than by eye.
 *
 * Text holds 4.5:1 on both the leaf and the surround; `--plate` and `--chalk`
 * hold 3:1 because they carry meaning on their own; `--rule` holds only 1.3:1
 * because it is a decorative hairline matched to the engraving's line
 * weights. A palette that drifts below these is a palette that stops being
 * readable for someone, and the drift is always one "slightly lighter" commit
 * at a time.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const TOKENS = readFileSync(
  fileURLToPath(new URL("../../src/lib/design/tokens.css", import.meta.url)),
  "utf8",
);

/** The declarations inside one `{ … }` block, as a token → value map. */
function block(selector: string): Map<string, string> {
  const start = TOKENS.indexOf(selector);
  if (start < 0) throw new Error(`no ${selector} block in tokens.css`);
  const open = TOKENS.indexOf("{", start);
  const close = TOKENS.indexOf("}", open);
  const declarations = new Map<string, string>();
  for (const [, name, value] of TOKENS.slice(open, close).matchAll(/(--[\w-]+):\s*([^;]+);/g)) {
    declarations.set(name, value.trim());
  }
  return declarations;
}

function channel(value: number): number {
  const srgb = value / 255;
  return srgb <= 0.04045 ? srgb / 12.92 : ((srgb + 0.055) / 1.055) ** 2.4;
}

/** WCAG relative luminance of a `#rrggbb` token. */
function luminance(hex: string): number {
  const value = Number.parseInt(hex.slice(1, 7), 16);
  const red = channel((value >> 16) & 0xff);
  const green = channel((value >> 8) & 0xff);
  const blue = channel(value & 0xff);
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function contrast(foreground: string, background: string): number {
  const [light, dark] = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return (light + 0.05) / (dark + 0.05);
}

/** Text 4.5:1; meaningful marks 3:1; the hairline merely visible. */
const REQUIRED: ReadonlyArray<readonly [string, number]> = [
  ["--ink", 4.5],
  ["--ink-muted", 4.5],
  ["--plate", 3],
  ["--chalk", 3],
  ["--rule", 1.3],
];

describe.each([
  ["light", ":root {"],
  ["dark", ':root[data-theme="dark"] {'],
])("%s tokens", (_theme, selector) => {
  const tokens = block(selector);

  it.each(REQUIRED)("%s holds %s:1 on the leaf and on the surround", (token, floor) => {
    const foreground = tokens.get(token);
    const leaf = tokens.get("--leaf");
    const surround = tokens.get("--surround");
    expect(foreground && leaf && surround).toBeTruthy();
    expect(contrast(foreground ?? "", leaf ?? "")).toBeGreaterThanOrEqual(floor);
    expect(contrast(foreground ?? "", surround ?? "")).toBeGreaterThanOrEqual(floor);
  });

  it("keeps the leaf a distinct object, not the surround", () => {
    expect(tokens.get("--leaf")).not.toBe(tokens.get("--surround"));
  });
});

describe("the token file", () => {
  it("defines the dark set for the system preference and for the toggle", () => {
    expect(TOKENS).toContain("@media (prefers-color-scheme: dark)");
    expect(TOKENS).toContain(':root[data-theme="dark"]');
    // The explicit light block is what lets the toggle win on a machine whose
    // system preference is dark.
    expect(TOKENS).toContain(':root[data-theme="light"]');
  });

  it("gives the same values to the media query and the dark attribute", () => {
    const media = block("@media (prefers-color-scheme: dark)");
    const attribute = block(':root[data-theme="dark"]');
    expect([...attribute]).toStrictEqual([...media]);
  });
});
