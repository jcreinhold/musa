/**
 * The structural outline.
 *
 * `annotated.musa` is the piece that has structure to navigate: two form
 * markers, two named phrases, and a harmony lane above the staff. The outline
 * is a table of contents for it — every row takes you to the place it names,
 * and nothing in it edits anything, because annotations are written in the
 * source.
 *
 * What is asserted here is what a composer would notice: the rows are in
 * playing order, choosing one moves the selection to the right note and puts
 * the source caret on the statement that wrote it, and the chord symbols
 * reach the engraving.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { pane, source } from "./source";
import { stubShell } from "./shell";

/** The outline's rows, as they read. */
async function rows(page: Page): Promise<string[]> {
  return page
    .getByRole("navigation", { name: "Structure" })
    .getByRole("button")
    .allInnerTexts()
    .then((texts) => texts.map((text) => text.replace(/\s+/g, " ").trim()));
}

/** The event the score pane says is current. */
async function current(page: Page): Promise<string> {
  return page.evaluate(
    () =>
      document
        .querySelector('[role="application"]')
        ?.getAttribute("aria-activedescendant") ?? "",
  );
}

test.beforeEach(async ({ page }) => {
  await stubShell(page, "annotated");
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await engraved(page);
});

test("the outline lists the piece's structure in playing order", async ({
  page,
}) => {
  // Sections and the phrases inside them, interleaved the way they are
  // reached — not sections first and phrases after.
  expect(await rows(page)).toEqual([
    "Exposition 1",
    "antecedent 1",
    "Development 3",
    "consequent 3",
  ]);
});

test("choosing a section selects the note it names", async ({ page }) => {
  await page
    .getByRole("navigation", { name: "Structure" })
    .getByText("Development")
    .click();
  // `event-5` is the first note of bar 3, where the marker is written.
  await expect.poll(() => current(page)).toBe("event-5");
});

test("choosing a marker opens the source at the statement that wrote it", async ({
  page,
}) => {
  // The source column is shut until something asks for it.
  await expect(source(page)).toBeHidden();
  await page
    .getByRole("navigation", { name: "Structure" })
    .getByText("Exposition")
    .click();
  await expect(pane(page)).toBeVisible();
  await expect(source(page)).toBeVisible();
  await expect(source(page)).toContainText('section "Exposition" at 1:1;');
});

test("the outline says where the selection is", async ({ page }) => {
  const development = page
    .getByRole("navigation", { name: "Structure" })
    .getByText("Development");
  await development.click();
  // Every passage the selection is inside is marked — the section and the
  // phrase within it — and the ones it has left are not.
  await expect.poll(() => rows(page)).toHaveLength(4);
  const active = await page.evaluate(() =>
    [...document.querySelectorAll("nav[aria-label='Structure'] .active")].map(
      (element) => element.textContent?.replace(/\s+/g, " ").trim() ?? "",
    ),
  );
  expect(active).toEqual(["Development 3", "consequent 3"]);
});

test("the chord symbols are engraved above the staff", async ({ page }) => {
  const harmony = await page.evaluate(() =>
    [...document.querySelectorAll(".engraving .harm")].map(
      (element) => element.textContent?.trim() ?? "",
    ),
  );
  expect(harmony.join(" ")).toContain("am");
});
