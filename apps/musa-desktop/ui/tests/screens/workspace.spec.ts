/**
 * The Source workspace (roadmap §14.4).
 *
 * The claim being tested is the one the workspace exists to make: the text
 * and the page are two views of one document. So every test here crosses
 * between them — a note chosen on the page moves the caret, a caret moved in
 * the text chooses the note, and a source the composer did not type (a
 * format) leaves the selection where it was.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";
import { marked, rewrite, selected, source, text } from "./source";

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await engraved(page);
});

/** Open the Source workspace the way the keyboard does. */
async function inSource(page: Page): Promise<void> {
  await page.keyboard.press("ControlOrMeta+4");
  await expect(page.locator(".source-workspace")).toBeVisible();
}

test("⌘4 opens the Source workspace, with the page still on it", async ({ page }) => {
  await inSource(page);

  // Text and page, side by side, and the switcher says which of the four
  // workspaces this is (`03-interaction.md` §3).
  await expect(source(page)).toBeVisible();
  await expect(page.locator(".source-workspace .engraving")).toBeVisible();
  const workspaces = page.getByRole("navigation", { name: "Workspace" }).getByRole("button");
  await expect(workspaces).toHaveCount(4);
  await expect(workspaces.nth(3)).toHaveAttribute("aria-current", "page");

  // And back, by the same map.
  await page.keyboard.press("ControlOrMeta+1");
  await expect(page.locator(".source-workspace")).toHaveCount(0);
});

test("choosing a note on the page puts the caret in the text it came from", async ({ page }) => {
  await inSource(page);

  // `g#4` is written once in Glass Mountain, so the note and the word are
  // unambiguously each other.
  await page.locator('.engraving [id="event-d"]').first().click({ force: true });

  // The note's own text is marked, and the keyboard stays on the page:
  // choosing a note is not asking to type.
  await expect.poll(() => marked(page)).toContain("g#4 1;");
  expect(
    await page.evaluate(() => document.activeElement?.closest('[role="application"]') !== null),
    "the keyboard is still on the page",
  ).toBe(true);
});

test("moving the caret in the text chooses the note on the page", async ({ page }) => {
  await inSource(page);

  // Click the word itself: the tokenizer sets it in its own element, which is
  // what makes the text clickable note by note.
  await page.locator(".cm-content").getByText("g#4", { exact: true }).click();

  await expect(page.locator(".overlay .selection")).toHaveCount(1);
  const chosen = await page.evaluate(() => {
    const pane = document.querySelector('[role="application"]');
    return pane?.getAttribute("aria-activedescendant") ?? "";
  });
  expect(chosen).toBe("event-d");
});

test("formatting keeps the selection on the word it was on", async ({ page }) => {
  await inSource(page);
  await rewrite(page, 'piece "A" {\ntempo quarter = 72;\nmeter 4/4;\n}');

  // Double-click selects the word, the way a composer would take hold of it.
  await page.locator(".cm-content").getByText("quarter", { exact: true }).dblclick();
  await expect.poll(() => selected(page)).toBe("quarter");

  await page.keyboard.press("ControlOrMeta+Shift+f");

  // The source is one the composer did not type — every line has moved — and
  // the selection is still on the word.
  await expect.poll(() => text(page)).toContain("    tempo quarter = 72;");
  await expect.poll(() => selected(page)).toBe("quarter");
});

test("a block folds, and the fold is the block the braces make", async ({ page }) => {
  await inSource(page);
  // The fold marker beside the `motif` line, clicked where a composer would
  // click it: in the gutter, on that line.
  const declaration = page.locator(".cm-content").getByText("motif", { exact: true });
  const line = await declaration.boundingBox();
  const gutter = await page.locator(".cm-foldGutter").boundingBox();
  await page.mouse.click(
    (gutter?.x ?? 0) + (gutter?.width ?? 0) / 2,
    (line?.y ?? 0) + (line?.height ?? 0) / 2,
  );

  // The five notes inside it are off the screen; the declaration stays.
  await expect(page.locator(".cm-content").getByText("rest", { exact: true })).toHaveCount(0);
  await expect(declaration).toBeVisible();
});

test("origin view marks the declaration and the use, in the workspace too", async ({ page }) => {
  await inSource(page);
  await page.locator('.engraving [id="event-4"]').first().click({ force: true });

  const marks = await marked(page);
  expect(marks.join("\n")).toContain("motif sigh");
  expect(marks.at(-1)).toBe("use sigh();");
});

/**
 * Where the caret is, and that it can be seen.
 *
 * A theme rule that styles an element no extension ever creates is silent —
 * the source shipped for a while with a `.cm-cursor` rule and no drawn cursor
 * behind it, which on a graphite sheet left a black caret on near-black. So
 * the assertion is on the drawn element and its colour, not on the stylesheet.
 */
test("the caret is drawn, and drawn in the hue that means 'here'", async ({ page }) => {
  await inSource(page);
  await source(page).locator(".cm-line").nth(3).click();

  const cursor = page.locator(".cm-cursor-primary");
  await expect(cursor).toHaveCount(1);
  const drawn = await cursor.evaluate((node) => {
    const style = getComputedStyle(node);
    return {
      color: style.borderLeftColor,
      width: style.borderLeftWidth,
      shown: style.display,
      plate: getComputedStyle(document.documentElement).getPropertyValue("--plate").trim(),
    };
  });
  expect(drawn.shown).toBe("block");
  expect(drawn.width).toBe("2px");
  expect(drawn.color).toBe(hex(drawn.plate));

  // And the line it is on says so a second way, so the answer does not rest on
  // colour alone (`03-interaction.md` §5): its number is set in full ink.
  const gutter = page.locator(".cm-lineNumbers .cm-activeLineGutter");
  await expect(gutter).toHaveText("4");
  const numbers = await gutter.evaluate((node) => ({
    color: getComputedStyle(node).color,
    ink: getComputedStyle(document.documentElement).getPropertyValue("--ink").trim(),
    others: getComputedStyle(
      node.parentElement?.querySelector(".cm-gutterElement:not(.cm-activeLineGutter)") ?? node,
    ).color,
  }));
  expect(numbers.color).toBe(hex(numbers.ink));
  expect(numbers.others).not.toBe(numbers.color);
});

/** `#rrggbb` as `getComputedStyle` reports it. */
function hex(value: string): string {
  const channels = [1, 3, 5].map((at) => Number.parseInt(value.slice(at, at + 2), 16));
  return `rgb(${channels.join(", ")})`;
}
