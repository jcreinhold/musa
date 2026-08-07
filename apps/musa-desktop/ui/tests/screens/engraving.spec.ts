/**
 * Raster goldens for each fixture's first page, per `02-engraving.md` §9.
 *
 * Both themes are photographed, and the dark image is the one that matters:
 * it proves the `currentColor` plumbing rather than a filter. A score that
 * had been inverted would show grey staff lines and the wrong glyph weights,
 * and the diff would catch it.
 *
 * Fixtures: `glass-mountain` (multi-part, motif expansion), `counterpoint`
 * (dense two-voice), `twinkle` (single line — the small-score case where bad
 * spacing is most visible).
 */

import { expect, test } from "@playwright/test";

import { engraved } from "./engraved";

const FIXTURES = ["glass-mountain", "counterpoint", "twinkle"] as const;
const THEMES = ["light", "dark"] as const;

for (const fixture of FIXTURES) {
  for (const theme of THEMES) {
    test(`${fixture} page 1 ${theme}`, async ({ page }) => {
      await page.setViewportSize({ width: 900, height: 1000 });
      await page.emulateMedia({ colorScheme: theme });
      await page.goto(`/?score=${fixture}&view=sheet`);
      await engraved(page);
      await expect(page.locator(".sheet")).toHaveScreenshot(`${fixture}-${theme}.png`);
    });
  }
}
