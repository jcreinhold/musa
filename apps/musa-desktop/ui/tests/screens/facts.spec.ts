/**
 * The piece's own facts, edited where they are printed (prompt 54).
 *
 * Three places print the same eight statements — the inspector when nothing
 * is selected, the top band's tempo, key and meter, and five lines on the
 * engraved page — and all three are the same field reaching the same command.
 *
 * As in `entry.spec.ts`, what these assert is the *command the interface
 * issued*: the stub is not a compiler, so what a `setHeader` does to the text
 * is asserted by `musa-project`'s editing laws — that an unnamed role is
 * inserted in the formatter's order, that emptying one deletes the statement,
 * that the title refuses to be emptied, and that a value which does not
 * compile leaves the session untouched.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";

/** Every edit the interface has asked the core for, in order. */
async function edits(page: Page): Promise<Record<string, unknown>[]> {
  return page.evaluate(() => window.__musaEdits);
}

async function settled(page: Page, count: number): Promise<void> {
  await expect.poll(() => page.evaluate(() => window.__musaEdits.length)).toBe(count);
}

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await stubShell(page);
  await page.goto("/");
  await engraved(page);
});

test("with nothing selected the inspector is the piece", async ({ page }) => {
  const piece = page.getByRole("group", { name: "This piece" });
  await expect(piece).toBeVisible();

  // Every field, in the order they are written — including the one this piece
  // has not filled in. That empty row is the discovery surface: it is how a
  // composer finds out a piece can name an arranger at all.
  await expect(piece.getByRole("textbox")).toHaveCount(8);
  await expect(piece.getByRole("textbox", { name: "title", exact: true })).toHaveValue("Glass Mountain");
  await expect(piece.getByRole("textbox", { name: "arranger" })).toHaveValue("");
  await expect(piece.getByRole("textbox", { name: "arranger" })).toHaveAttribute(
    "placeholder",
    "—",
  );
  // Except the title, which has no empty state to stand in for: a piece with
  // no name has nothing for the file, the page head, or the frame to print.
  await expect(piece.getByRole("textbox", { name: "title", exact: true })).not.toHaveAttribute(
    "placeholder",
  );

  // And spelled the way the source spells them, because that is what a
  // composer types back.
  await expect(piece.getByRole("textbox", { name: "tempo" })).toHaveValue("quarter = 72");
  await expect(piece.getByRole("textbox", { name: "key" })).toHaveValue("a minor");
});

test("naming the arranger is one edit, in the language's own words", async ({ page }) => {
  const piece = page.getByRole("group", { name: "This piece" });
  await piece.getByRole("textbox", { name: "arranger" }).fill("J. Reinhold");
  await page.keyboard.press("Enter");

  await settled(page, 1);
  expect((await edits(page))[0]).toMatchObject({
    kind: "setHeader",
    field: "arranger",
    value: "J. Reinhold",
  });
});

test("clearing a role it did state takes the statement back off", async ({ page }) => {
  const piece = page.getByRole("group", { name: "This piece" });
  await piece.getByRole("textbox", { name: "composer" }).fill("");
  await page.keyboard.press("Enter");

  // Adding and removing a line of front matter are the same gesture, and
  // neither needs a button.
  await settled(page, 1);
  expect((await edits(page))[0]).toMatchObject({ kind: "setHeader", field: "composer", value: "" });
});

test("the meter is changed from the band it is printed in", async ({ page }) => {
  // Scoped to the band: the inspector prints the same field, which is the
  // point — a composer changes the meter wherever they are reading it.
  const meter = page.locator(".readout").getByRole("textbox", { name: "Meter" });
  await expect(meter).toHaveValue("4/4");
  await meter.fill("6/8");
  await page.keyboard.press("Enter");

  await settled(page, 1);
  expect((await edits(page))[0]).toMatchObject({ kind: "setHeader", field: "meter", value: "6/8" });
});

test("the page says which of its lines are fields, on hover", async ({ page }) => {
  const hairline = page.locator(".engraving .hairline");
  await expect(hairline).toHaveCount(0);

  // Drawn, not declared: SVG text paints `text-decoration` with the glyph's
  // own fill, so a transparent rest state is not available and the hairline
  // has to be the interface's own rule laid over the page.
  await page.locator('.engraving [id="front-composer"]').hover({ force: true });
  await expect(hairline).toHaveCount(1);

  // And the music is not a field: hovering a note says nothing of the kind.
  await page.locator('.engraving [id="event-4"] use').hover({ force: true });
  await expect(hairline).toHaveCount(0);
});

test("clicking the title on the page renames the piece there", async ({ page }) => {
  const before = await page.evaluate(() => window.__musaRevision);
  await page.locator('.engraving [id="front-title"]').click({ force: true });

  // A field over the printed line, in the page's own face — not a panel
  // somewhere else with the title in it.
  const field = page.locator(".engraving input.front");
  await expect(field).toBeFocused();
  await expect(field).toHaveValue("Glass Mountain");

  await field.fill("Obsidian");
  await page.keyboard.press("Enter");
  await expect(field).toHaveCount(0);

  await settled(page, 1);
  expect((await edits(page))[0]).toMatchObject({
    kind: "setHeader",
    field: "title",
    value: "Obsidian",
  });

  // And it is one revision, so it is one ⌘Z — a rename is an ordinary edit.
  const after = await page.evaluate(() => window.__musaRevision);
  expect(after).toBeGreaterThan(before);
  await page.keyboard.press("Meta+z");
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBe(before);
});

test("escape puts the printed line back and asks for nothing", async ({ page }) => {
  await page.locator('.engraving [id="front-composer"]').click({ force: true });
  const field = page.locator(".engraving input.front");
  await field.fill("someone else");
  await page.keyboard.press("Escape");

  await expect(field).toHaveCount(0);
  expect(await edits(page)).toHaveLength(0);
  // Escaping the field does not also clear the selection: the nearest thing
  // first, which is the rule `03-interaction.md` §3 states for the key.
  await expect(page.getByRole("group", { name: "This piece" })).toBeVisible();
});

test("clicking a note is still clicking a note", async ({ page }) => {
  await page.locator('.engraving [id="event-4"] use').click({ force: true });
  await expect(page.locator(".engraving input.front")).toHaveCount(0);
  await expect(page.locator(".overlay rect.selection")).toHaveCount(1);
});

test("the piece view, photographed", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await expect(page).toHaveScreenshot("facts-piece-1440x900-light.png");
});

test("the piece view in the dark", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await engraved(page);
  await expect(page).toHaveScreenshot("facts-piece-1440x900-dark.png");
});

test("the page mid-rename, photographed", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.locator('.engraving [id="front-title"]').click({ force: true });
  await expect(page.locator(".engraving input.front")).toBeFocused();
  await expect(page).toHaveScreenshot("facts-rename-1440x900-light.png");
});

test("the page mid-rename in the dark", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await engraved(page);
  await page.locator('.engraving [id="front-title"]').click({ force: true });
  await expect(page.locator(".engraving input.front")).toBeFocused();
  await expect(page).toHaveScreenshot("facts-rename-1440x900-dark.png");
});
