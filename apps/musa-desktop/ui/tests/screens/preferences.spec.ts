/**
 * How the composer reads and types (prompt 55).
 *
 * Two preferences, and the two boundaries that make them safe: the text size
 * is the *frame's* and never the score's, and vim mode is the source column's
 * and never the application's. Each of these tests is one of those boundaries
 * being held.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";
import { pane, source, text, toggleSource } from "./source";

/** Run a command the way the native menu and the palette both do. */
async function run(page: Page, id: string): Promise<void> {
  await page.evaluate((command) => window.__musaEmit("musa://command", command), id);
}

/** The computed size of a piece of the frame's own type, in pixels. */
async function frameType(page: Page): Promise<number> {
  return page
    .locator("nav.workspaces button")
    .first()
    .evaluate((node) => Number.parseFloat(globalThis.getComputedStyle(node).fontSize));
}

/**
 * How much of the page one staff takes.
 *
 * A ratio rather than a height, because the leaf is fitted to whatever room
 * the chrome leaves and larger chrome legitimately leaves less: the page gets
 * smaller, uniformly, exactly as it does when the window is resized. What
 * must not change is the engraving *inside* it — a staff that took a larger
 * share of its page would be a second zoom.
 */
async function staffShare(page: Page): Promise<number> {
  const staff = await page.locator(".engraving .page svg g.staff").first().boundingBox();
  const leaf = await page.locator(".engraving .page").first().boundingBox();
  return (staff?.height ?? 0) / (leaf?.height ?? 1);
}

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await stubShell(page);
  await page.goto("/");
  await engraved(page);
});

test("a step larger is the frame's type, and only the frame's", async ({ page }) => {
  const before = await frameType(page);
  const share = await staffShare(page);

  await run(page, "settings.text.larger");
  await expect.poll(() => frameType(page)).toBeGreaterThan(before);

  // The score's size is zoom, it is a re-layout, and it already has a control.
  // A text-size preference that also grew the staves would be a second zoom
  // that disagrees with the first (`02-engraving.md` §5).
  //
  // Polled, because the claim is about where the engraving lands and not about
  // the moment in between: taller chrome resizes the leaf before the layout it
  // asked for comes back, and read in that gap the ratio is last layout's staff
  // over this layout's page — a number that belongs to neither.
  //
  // And read as a ratio against itself, within a few percent rather than to
  // the pixel. Larger type makes the band 9px taller, the leaf is fitted to
  // the 784px that leaves instead of 793, and Verovio's staff-to-page ratio
  // is not exactly scale-invariant across that: measured, it moves 1.3%,
  // which is the same drift a window resize of the same size produces. The
  // smallest zoom step is 10% (`ZOOM_STEPS`). What this has to tell apart is
  // those two, and 5% sits between them.
  await expect.poll(async () => (await staffShare(page)) / share).toBeCloseTo(1, 1);

  await run(page, "settings.text.reset");
  await expect.poll(() => frameType(page)).toBeCloseTo(before, 1);
});

test("the ladder has four rungs and stops at both ends", async ({ page }) => {
  const normal = await frameType(page);
  for (let step = 0; step < 5; step += 1) await run(page, "settings.text.larger");
  const largest = await frameType(page);
  for (let step = 0; step < 5; step += 1) await run(page, "settings.text.smaller");
  const smallest = await frameType(page);

  expect(largest / normal).toBeCloseTo(1.3, 2);
  expect(smallest / normal).toBeCloseTo(0.85, 2);
});

test("the size outlives the window it was chosen in", async ({ page }) => {
  const before = await frameType(page);
  await run(page, "settings.text.larger");
  await expect.poll(() => frameType(page)).toBeGreaterThan(before);

  await page.reload();
  await engraved(page);
  expect(await frameType(page)).toBeCloseTo(before * 1.15, 1);
});

test("with vim off, Esc still puts the source column away", async ({ page }) => {
  await toggleSource(page);
  await source(page).click();
  await page.keyboard.press("Escape");
  // Nothing is selected after the click landed in the text, so the second
  // Escape is the one that closes it — the first clears the selection.
  await page.keyboard.press("Escape");
  await expect(pane(page)).toHaveCount(0);
});

test("with vim on, Esc leaves insert mode and the column stays", async ({ page }) => {
  await run(page, "settings.vim");
  await toggleSource(page);
  await source(page).click();

  // The mode says which mode it is in, which is the one thing worse to omit
  // than the mode itself.
  await expect(page.locator(".cm-vim-panel")).toContainText("NORMAL");

  await page.keyboard.press("i");
  await expect(page.locator(".cm-vim-panel")).toContainText("INSERT");
  await page.keyboard.press("Escape");

  // Back to normal mode, and the column is still there: `Esc` belongs to vim
  // while vim mode is on and the caret is in the source, and nowhere else.
  await expect(page.locator(".cm-vim-panel")).toContainText("NORMAL");
  await expect(pane(page)).toHaveCount(1);
});

test("vim's undo is the project's undo, not a second history", async ({ page }) => {
  await run(page, "settings.vim");
  await toggleSource(page);
  await source(page).click();

  const before = await page.evaluate(() => window.__musaRevision);
  await page.keyboard.press("i");
  await page.keyboard.insertText("// a note to self\n");
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBeGreaterThan(before);
  await page.keyboard.press("Escape");

  await page.keyboard.press("u");
  // The revision went back, which is the whole point: `u` and `⌘Z` are one
  // undo over one document, and a vim history beside the project's would
  // disagree with it about what the document is.
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBe(before);
});

test("`:w` is the project's save", async ({ page }) => {
  await run(page, "settings.vim");
  await toggleSource(page);
  await source(page).click();
  await page.keyboard.press("i");
  await page.keyboard.insertText("// x\n");
  await page.keyboard.press("Escape");

  await page.keyboard.press(":");
  await page.keyboard.insertText("w");
  await page.keyboard.press("Enter");

  await expect.poll(() => page.evaluate(() => window.__musaSaves)).toBeGreaterThan(0);
});

test("the mode outlives the window it was turned on in", async ({ page }) => {
  await run(page, "settings.vim");
  await page.reload();
  await engraved(page);
  await toggleSource(page);
  await source(page).click();
  await expect(page.locator(".cm-vim-panel")).toContainText("NORMAL");
});

test("vim mode changes the keys and nothing about the document", async ({ page }) => {
  await toggleSource(page);
  const before = await text(page);
  await run(page, "settings.vim");
  expect(await text(page)).toBe(before);
});

/**
 * The settings sheet (prompt 59).
 *
 * The preferences above are reached by key and by palette; this is the third
 * way in, and the only one that shows all of them at once and says which
 * state each is in. Every assertion here is about that: the sheet marks what
 * is in force, and choosing marks something else.
 */
test.describe("settings", () => {
  const sheet = (page: Page) => page.getByRole("dialog", { name: "Settings" });

  /** The choice currently in force in the row named `label`. */
  function inForce(page: Page, label: string) {
    return sheet(page)
      .getByRole("group", { name: label })
      .locator('button[aria-pressed="true"]');
  }

  test("⌘, opens it, and Esc closes it", async ({ page }) => {
    await expect(sheet(page)).toHaveCount(0);
    await page.keyboard.press("ControlOrMeta+,");
    await expect(sheet(page)).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(sheet(page)).toHaveCount(0);
  });

  test("each row says which choice is in force", async ({ page }) => {
    await run(page, "settings.open");

    await expect(inForce(page, "Theme")).toHaveText("System");
    await expect(inForce(page, "Text size")).toHaveText("Normal");
    await expect(inForce(page, "Vim mode")).toHaveText("Off");
  });

  test("choosing a text size takes it, and the sheet says so", async ({ page }) => {
    const before = await frameType(page);
    await run(page, "settings.open");

    await sheet(page).getByRole("button", { name: "Large", exact: true }).click();
    await expect(inForce(page, "Text size")).toHaveText("Large");
    await expect.poll(() => frameType(page)).toBeGreaterThan(before);
  });

  /**
   * The state the menu could not reach. `ThemeChoice` always had three —
   * `chosen: null` is *follow the system* — and "Switch theme" only ever
   * moved between the other two, so an override was permanent.
   */
  test("System is reachable again after choosing a theme", async ({ page }) => {
    await run(page, "settings.open");

    await sheet(page).getByRole("button", { name: "Dark" }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");

    await sheet(page).getByRole("button", { name: "System" }).click();
    await expect(page.locator("html")).not.toHaveAttribute("data-theme", /.*/);
    await expect(inForce(page, "Theme")).toHaveText("System");
  });

  test("vim mode is turned on here and holds in the source", async ({ page }) => {
    await run(page, "settings.open");
    await sheet(page).getByRole("button", { name: "On" }).click();
    await expect(inForce(page, "Vim mode")).toHaveText("On");
    await page.keyboard.press("Escape");

    await toggleSource(page);
    await source(page).click();
    await expect(page.locator(".cm-vim-panel")).toContainText("NORMAL");
  });

  /**
   * A preference the composer sets once and sets again next launch is not a
   * preference. Each of the three is stored on its own (`musa.theme`,
   * `musa.text-size`, `musa.vim`); this is the one test that says all three
   * come back, because the sheet is where all three are set together.
   */
  test("every choice outlives the window it was made in", async ({ page }) => {
    await run(page, "settings.open");
    await sheet(page).getByRole("button", { name: "Dark" }).click();
    await sheet(page).getByRole("button", { name: "Larger" }).click();
    await sheet(page).getByRole("button", { name: "On" }).click();

    await page.reload();
    await engraved(page);

    // Applied on the way up, before the sheet is asked about them: the theme
    // is on the root and the size is in the frame's type from the first paint.
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await run(page, "settings.open");
    await expect(inForce(page, "Theme")).toHaveText("Dark");
    await expect(inForce(page, "Text size")).toHaveText("Larger");
    await expect(inForce(page, "Vim mode")).toHaveText("On");
  });

  test("the sheet has no accessibility violations", async ({ page }) => {
    const { default: AxeBuilder } = await import("@axe-core/playwright");
    await run(page, "settings.open");
    await expect(sheet(page)).toBeVisible();

    const { violations } = await new AxeBuilder({ page }).disableRules(["svg-img-alt"]).analyze();
    expect(violations.map((violation) => violation.id)).toEqual([]);
  });
});

test("the frame at Larger, photographed", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await run(page, "settings.text.larger");
  await run(page, "settings.text.larger");
  await expect(page).toHaveScreenshot("larger-1440x900-light.png");
});

test("the frame at Larger, in the dark", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await engraved(page);
  await run(page, "settings.text.larger");
  await run(page, "settings.text.larger");
  await expect(page).toHaveScreenshot("larger-1440x900-dark.png");
});
