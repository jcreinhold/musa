/**
 * The accessibility floor of `03-interaction.md` §5, checked rather than
 * asserted in prose.
 *
 * Zero violations is the bar. Contrast is in it, which is why the two themes
 * are both scanned: `01-visual-language.md`'s palette is a claim about
 * legibility, and a claim about legibility is a testable one.
 */

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { drawer } from "./source";
import { stubShell } from "./shell";

/**
 * Verovio's engraving is scanned for everything except the rules about text
 * alternatives on its thousands of glyph groups: the score is named as a
 * whole, its events carry musical names, and its paths are decoration of
 * those. `svg-img-alt` on a stem is noise, not a finding.
 */
function scan(page: Page): AxeBuilder {
  return new AxeBuilder({ page }).disableRules(["svg-img-alt"]);
}

for (const theme of ["light", "dark"] as const) {
  test(`the workspace has no accessibility violations in ${theme}`, async ({ page }) => {
    await stubShell(page);
    await page.goto(`/?theme=${theme}`);
    await engraved(page);

    const { violations } = await scan(page).analyze();
    expect(violations.map((violation) => `${violation.id}: ${violation.help}`)).toEqual([]);
  });
}

test("the drawer, the palette, and the sheet are clean too", async ({ page }) => {
  await stubShell(page);
  await page.goto("/");
  await engraved(page);

  await drawer(page).click();
  await expect(page.getByRole("textbox", { name: "Source" })).toBeVisible();
  expect((await scan(page).analyze()).violations.map((violation) => violation.id)).toEqual([]);

  await page.keyboard.press("Meta+k");
  await expect(page.getByRole("dialog", { name: "Commands" })).toBeVisible();
  expect((await scan(page).analyze()).violations.map((violation) => violation.id)).toEqual([]);

  await page.keyboard.press("Escape");
  await page.getByRole("application", { name: "Engraved score" }).focus();
  await page.keyboard.press("Shift+?");
  await expect(page.getByRole("dialog", { name: "Keyboard" })).toBeVisible();
  expect((await scan(page).analyze()).violations.map((violation) => violation.id)).toEqual([]);
});

/**
 * 200 % browser zoom, which WCAG asks for and which a fixed-pixel layout
 * fails. The leaf may become smaller; nothing may be cut off the side.
 */
test("nothing is clipped at 200 % zoom", async ({ page }) => {
  await stubShell(page);
  await page.setViewportSize({ width: 720, height: 450 });
  await page.goto("/");
  await engraved(page);

  const overflow = await page.evaluate(() => {
    const root = document.documentElement;
    return root.scrollWidth - root.clientWidth;
  });
  expect(overflow, "horizontal overflow in CSS pixels").toBeLessThanOrEqual(1);
});
