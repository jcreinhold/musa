/**
 * The elaboration workbench (`08-elaboration.md`), through the screen.
 *
 * Two pieces drive it, both real: `stdlib-basics.musa`, whose composer used
 * terms they did not declare, and `kernel-splice.musa`, every note of which is
 * generated through a kernel quote. What is tested is that the interface reads
 * the compiler's facts and adds nothing of its own — a term it did not resolve
 * has no tooltip, a step that is not a place reveals nothing, and a reading is
 * never painted as a problem.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";
import { caret, marked, source, toggleSource } from "./source";

/** Open the Source workspace the way the keyboard does. */
async function inSource(page: Page): Promise<void> {
  await page.keyboard.press("ControlOrMeta+4");
  await expect(page.locator(".source-workspace")).toBeVisible();
}

/** Put the caret on the first occurrence of `word` in the document. */
async function caretOn(page: Page, word: string): Promise<void> {
  await source(page).getByText(word, { exact: false }).first().click();
}

/** The analysis panel, by the name it announces itself with. */
function findings(page: Page) {
  return page.getByRole("region", { name: "Analysis" });
}

test.describe("terms", () => {
  test.beforeEach(async ({ page }) => {
    await stubShell(page, "stdlib-basics");
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await engraved(page);
    await inSource(page);
  });

  test("hovering a term shows the declaration's own words", async ({ page }) => {
    await source(page).getByText("compose_music", { exact: false }).first().hover();
    const tooltip = page.locator(".cm-musa-term-doc");
    await expect(tooltip).toBeVisible();
    // The signature is the source's, and the detail a reader has to ask for.
    await expect(tooltip.locator(".signature")).toContainText("compose_music");
    await expect(tooltip.locator("summary")).toHaveText("language detail");
  });

  test("hovering a word the compiler did not resolve shows nothing", async ({ page }) => {
    await source(page).getByText("Standard Library Basics", { exact: false }).first().hover();
    await expect(page.locator(".cm-musa-term-doc")).toHaveCount(0);
  });

  /*
   * The keyboard reaches both readings of one resolved name, because a
   * composer who never touches a pointer must be able to follow a name and
   * gather its uses (`08-elaboration.md` §7).
   */
  test("⌘⇧D opens the module a term the composer did not declare came from", async ({ page }) => {
    await caretOn(page, "naturals");
    await page.keyboard.press("ControlOrMeta+Shift+D");

    // A library document, plainly labelled and not editable.
    const head = page.locator(".head.library");
    await expect(head).toBeVisible();
    await expect(head).toContainText("read-only");
    await expect(source(page)).toHaveAttribute("aria-readonly", "true");
    await expect(source(page)).toContainText("fn naturals");
  });

  test("closing the module comes back to the piece, unchanged", async ({ page }) => {
    await caretOn(page, "naturals");
    await page.keyboard.press("ControlOrMeta+Shift+D");
    await expect(page.locator(".head.library")).toBeVisible();

    await page.locator(".head.library").getByRole("button").click();
    await expect(page.locator(".head.library")).toHaveCount(0);
    await expect(source(page)).toContainText("Standard Library Basics");
    await expect(source(page)).not.toHaveAttribute("aria-readonly", "true");
  });

  test("⌘⇧D on a term declared here moves the caret rather than opening a module", async ({ page }) => {
    await caretOn(page, "answer");
    await page.keyboard.press("ControlOrMeta+Shift+D");
    await expect(page.locator(".head.library")).toHaveCount(0);
    await expect(source(page)).toContainText("let answer");
  });
});

test.describe("an expansion through a kernel quote", () => {
  test.beforeEach(async ({ page }) => {
    await stubShell(page, "kernel-splice");
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await engraved(page);
  });

  test("Origin prints every step, and names each by what it is", async ({ page }) => {
    await page
      .locator(String.raw`.engraving [id="event-0"] use`)
      .first()
      .click({ force: true });
    const path = page.locator(".inspector .path");
    await expect(path).toBeVisible();

    const segments = path.getByRole("button");
    // Plural, and left plural: `assert … ▸ assembled ▸ splice at …`.
    expect(await segments.count()).toBeGreaterThan(1);
    // The kind is in the accessible name, so a screen reader hears what a
    // sighted reader sees in the row's shape (`03-interaction.md` §5).
    await expect(segments.first()).toHaveAccessibleName(/^assertion /);
    await expect(path.locator(".segment.splice")).toHaveAccessibleName(/^kernel quotation /);
  });

  /*
   * §4: a step that is not a place has nothing to reveal. The splice happened
   * at a time in a kernel term, not at an offset in the file, and a row that
   * moved the caret to the nearest brace would be inventing a source map.
   */
  test("a step with no place in the source moves no caret", async ({ page }) => {
    // Compose with its source column showing: the Origin row and the text have
    // to be on screen together for "nothing moved" to mean anything. The note
    // is chosen first, while the page still has the whole width.
    await page
      .locator(String.raw`.engraving [id="event-0"] use`)
      .first()
      .click({ force: true });
    await toggleSource(page);
    await expect(source(page)).toBeVisible();

    const path = page.locator(".inspector .path");
    const splice = path.locator(".segment.splice").first();
    const term = path.locator(".segment.occurrence").first();
    await expect(splice).toBeVisible();

    // The step beside it is a place, and moves the caret — so a caret that
    // does not move for the splice is the rule working, not the wiring absent.
    await term.click();
    await expect.poll(() => marked(page)).not.toEqual([]);
    const before = await caret(page);
    await splice.click();
    expect(await caret(page)).toBe(before);
  });
});

test.describe("readings", () => {
  test.beforeEach(async ({ page }) => {
    await stubShell(page, "glass-mountain");
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await engraved(page);
    await inSource(page);
  });

  test("nothing is read until it is asked for", async ({ page }) => {
    await expect(findings(page)).toBeVisible();
    await expect(findings(page).locator(".method")).toHaveCount(0);
  });

  test("an ambiguous reading keeps both answers, and neither is an error", async ({ page }) => {
    await findings(page).getByRole("button", { name: "tonal", exact: true }).click();

    // Method and assumptions first: a reader who does not accept them can stop
    // there, which is the whole reason they are printed (§5).
    await expect(findings(page).locator(".method")).toBeVisible();
    await expect(findings(page).locator(".assumptions li").first()).toBeVisible();

    // Two key regions, both standing, because two keys explain this passage.
    const regions = findings(page).locator("li .standing .word");
    expect(await regions.count()).toBeGreaterThan(1);

    // And nothing in the panel is a diagnostic.
    await expect(findings(page).locator(".diagnostic")).toHaveCount(0);
  });

  test("a finding takes the selection to the notes it is about", async ({ page }) => {
    await findings(page).getByRole("button", { name: "tonal", exact: true }).click();
    await findings(page).locator("button.finding").first().click();
    await expect(page.locator(".overlay rect.selection").first()).toBeVisible();
  });

  test("a reading of a score that has changed says so, and stays on screen", async ({ page }) => {
    await findings(page).getByRole("button", { name: "tonal", exact: true }).click();
    await expect(findings(page).locator(".method")).toBeVisible();
    const summaries = await findings(page).locator("button.finding").count();
    expect(summaries).toBeGreaterThan(0);

    // A compile the composer's typing produced: a newer score, same reading.
    await page.evaluate(() => window.__musaSet({ scoreRevision: 999 }));
    await expect(findings(page).locator(".stale")).toBeVisible();
    await expect(findings(page).locator("button.finding")).toHaveCount(summaries);
  });

  test("an analysis with nothing to report says so", async ({ page }) => {
    await findings(page).getByRole("button", { name: "cadences", exact: true }).click();
    await expect(findings(page).locator(".nothing")).toHaveText("Nothing found.");
  });

  test("every reading the compiler offers can be asked for by keyboard", async ({ page }) => {
    const buttons = findings(page).getByRole("button");
    expect(await buttons.count()).toBe(6);
    for (const button of await buttons.all()) await expect(button).toBeEnabled();
  });
});

/*
 * The photographs. Two widths, because what gives way differs: at 1440 the
 * source column holds its measure and the findings sit under it, at 1100 it
 * trades characters for the page (`01-visual-language.md` §8).
 */
const SIZES = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1100x720", width: 1100, height: 720 },
] as const;

for (const size of SIZES) {
  test(`a reading, photographed at ${size.name}`, async ({ page }) => {
    await stubShell(page, "glass-mountain");
    await page.setViewportSize({ width: size.width, height: size.height });
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/");
    await engraved(page);
    await inSource(page);
    await findings(page).getByRole("button", { name: "tonal", exact: true }).click();
    await expect(findings(page).locator(".method")).toBeVisible();
    await expect(page).toHaveScreenshot(`findings-${size.name}-light.png`);
  });

  test(`a bundled module, photographed at ${size.name}`, async ({ page }) => {
    await stubShell(page, "stdlib-basics");
    await page.setViewportSize({ width: size.width, height: size.height });
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/");
    await engraved(page);
    await inSource(page);
    await caretOn(page, "naturals");
    await page.keyboard.press("ControlOrMeta+Shift+D");
    await expect(page.locator(".head.library")).toBeVisible();
    await expect(page).toHaveScreenshot(`library-${size.name}-light.png`);
  });
}
