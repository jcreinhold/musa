/**
 * The piece's own facts, edited where they are printed.
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

import { expect, test, type Locator, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";

/** Every edit the interface has asked the core for, in order. */
async function edits(page: Page): Promise<Record<string, unknown>[]> {
  return page.evaluate(() => window.__musaEdits);
}

async function settled(page: Page, count: number): Promise<void> {
  await expect
    .poll(() => page.evaluate(() => window.__musaEdits.length))
    .toBe(count);
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
  await expect(
    piece.getByRole("textbox", { name: "title", exact: true }),
  ).toHaveValue("Glass Mountain");
  await expect(piece.getByRole("textbox", { name: "arranger" })).toHaveValue(
    "",
  );
  await expect(
    piece.getByRole("textbox", { name: "arranger" }),
  ).toHaveAttribute("placeholder", "—");
  // Except the title, which has no empty state to stand in for: a piece with
  // no name has nothing for the file, the page head, or the frame to print.
  await expect(
    piece.getByRole("textbox", { name: "title", exact: true }),
  ).not.toHaveAttribute("placeholder");

  // And spelled the way the source spells them, because that is what a
  // composer types back.
  await expect(piece.getByRole("textbox", { name: "tempo" })).toHaveValue(
    "quarter = 72",
  );
  await expect(piece.getByRole("textbox", { name: "key" })).toHaveValue(
    "a minor",
  );
});

test("naming the arranger is one edit, in the language's own words", async ({
  page,
}) => {
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

test("clearing a role it did state takes the statement back off", async ({
  page,
}) => {
  const piece = page.getByRole("group", { name: "This piece" });
  await piece.getByRole("textbox", { name: "composer" }).fill("");
  await page.keyboard.press("Enter");

  // Adding and removing a line of front matter are the same gesture, and
  // neither needs a button.
  await settled(page, 1);
  expect((await edits(page))[0]).toMatchObject({
    kind: "setHeader",
    field: "composer",
    value: "",
  });
});

test("the meter is changed from the band it is printed in", async ({
  page,
}) => {
  // Scoped to the band: the inspector prints the same field, which is the
  // point — a composer changes the meter wherever they are reading it.
  const meter = page
    .locator(".readout")
    .getByRole("textbox", { name: "Meter" });
  await expect(meter).toHaveValue("4/4");
  await meter.fill("6/8");
  await page.keyboard.press("Enter");

  await settled(page, 1);
  expect((await edits(page))[0]).toMatchObject({
    kind: "setHeader",
    field: "meter",
    value: "6/8",
  });
});

test("the page says which of its lines are fields, on hover", async ({
  page,
}) => {
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

test("clicking the title on the page renames the piece there", async ({
  page,
}) => {
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
  await expect
    .poll(() => page.evaluate(() => window.__musaRevision))
    .toBe(before);
});

test("escape puts the printed line back and asks for nothing", async ({
  page,
}) => {
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

/**
 * How much of a field's own text is scrolled out of sight.
 *
 * Zero, always. This is the direct measurement of the defect it guards: the
 * copyright field used to read `© 2026. Licensed CC BY-` with the rest inside
 * the input, which no assertion about the *layout* could see, because the
 * layout was right — the field sat in its column and the column sat in the
 * margin. Only the field's own scroll extent knows.
 */
async function hidden(field: Locator): Promise<number> {
  return field.evaluate((node: HTMLTextAreaElement) =>
    // A pixel of slack: sub-pixel text metrics round the two apart on some
    // values even when every glyph is on screen.
    Math.max(
      node.scrollWidth - node.clientWidth,
      node.scrollHeight - node.clientHeight,
      0,
    ),
  );
}

test("a value longer than its column wraps rather than hiding the rest", async ({
  page,
}) => {
  const piece = page.getByRole("group", { name: "This piece" });
  const copyright = piece.getByRole("textbox", { name: "copyright" });
  const line = await copyright.evaluate((node) =>
    Number.parseFloat(globalThis.getComputedStyle(node).lineHeight),
  );

  // Two steps larger is the size at which this became routine rather than
  // rare, and the reason it is worth a test: the composer who most needs the
  // preference is the one who could no longer read their own copyright.
  for (const step of [1, 2]) {
    await page.evaluate((count) => {
      for (let taken = 0; taken < count; taken += 1)
        window.__musaEmit("musa://command", "settings.text.larger");
    }, step);
  }

  expect(await hidden(copyright)).toBeLessThanOrEqual(1);
  // Wrapped, specifically — a field that got taller is showing the words a
  // field that stayed one line tall was hiding.
  const box = await copyright.boundingBox();
  expect(box?.height ?? 0).toBeGreaterThan(line * 1.5);

  // And still inside the margin it was given. Wrapping is the fix; growing
  // the column would have been a different bug.
  const margin = await page.locator(".margin.right").boundingBox();
  expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(
    (margin?.x ?? 0) + (margin?.width ?? 0) + 1,
  );
});

test("a short value keeps its own width", async ({ page }) => {
  // The other half of the same rule. A field stretched to its column would
  // put the hairline under two characters and a stretch of nothing, and
  // `01-visual-language.md` §7 makes that hairline the whole affordance —
  // it has to sit under the value it belongs to.
  const piece = page.getByRole("group", { name: "This piece" });
  const meter = await piece
    .getByRole("textbox", { name: "meter" })
    .boundingBox();
  const copyright = await piece
    .getByRole("textbox", { name: "copyright" })
    .boundingBox();
  expect(meter?.width ?? 0).toBeLessThan((copyright?.width ?? 0) / 2);
});

test("a pasted line break is folded into the value, not into the source", async ({
  page,
}) => {
  // The field wraps now, so a break pasted into it would look like it
  // belonged there — and would reach the document as a break inside a
  // statement.
  const piece = page.getByRole("group", { name: "This piece" });
  const field = piece.getByRole("textbox", { name: "subtitle" });
  await field.click();
  await field.fill("");
  await page.keyboard.insertText("for violin\nand strings");
  await expect(field).toHaveValue("for violin and strings");
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
