/**
 * Origin view (`04-provenance.md`), the application's signature.
 *
 * `glass-mountain.musa` is the canonical case the specification names: ten of
 * the violin's notes come from two occurrences of one five-note motif, one of
 * them transposed down a fifth. Every assertion here is against that fact, so
 * a change that breaks provenance breaks this file rather than a screenshot.
 *
 * The rule the lens is judged by is that it changes ink and nothing else: no
 * reflow, no insertion into the engraving, no page that moves under the
 * reader (§5). That is asserted directly, by measuring the notes.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import {
  caret,
  toggleSource,
  marked,
  rewrite,
  selected,
  source,
} from "./source";
import { stubShell } from "./shell";

/** Where the generated notes are, and what colour they are in, right now. */
async function generated(
  page: Page,
): Promise<{ box: DOMRect; color: string }[]> {
  return page.evaluate(() =>
    [
      ...document.querySelectorAll(
        '.engraving .arriving g[data-generated="true"]',
      ),
    ].map((element) => ({
      box: element.getBoundingClientRect().toJSON() as DOMRect,
      color: globalThis.getComputedStyle(element).color,
    })),
  );
}

/** Wait out the 120 ms cross-fade, so a colour is the settled one (B9). */
async function settled(page: Page): Promise<void> {
  await page.waitForTimeout(250);
}

/** Focus the score pane, so the unmodified keys are the score's (§3). */
async function inScore(page: Page): Promise<void> {
  await page.getByRole("application", { name: "Engraved score" }).focus();
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await engraved(page);
});

test("holding the lens re-inks the generated music and moves nothing", async ({
  page,
}) => {
  const before = await generated(page);
  expect(
    before.length,
    "glass-mountain's violin is generated",
  ).toBeGreaterThanOrEqual(10);

  await inScore(page);
  await page.keyboard.down("o");
  await expect(page.locator(".overlay g.bracket").first()).toBeVisible();
  await settled(page);
  const held = await generated(page);

  expect(held.map((note) => note.color)).not.toEqual(
    before.map((note) => note.color),
  );
  // The one thing the lens may never do: move the music (§5).
  expect(held.map((note) => note.box)).toEqual(before.map((note) => note.box));

  await page.keyboard.up("o");
  await expect(page.locator(".overlay g.bracket")).toHaveCount(0);
  await settled(page);
  const released = await generated(page);
  expect(released.map((note) => note.color)).toEqual(
    before.map((note) => note.color),
  );
});

test("every occurrence gets a bracket that names it", async ({ page }) => {
  await inScore(page);
  await page.keyboard.down("o");

  const labels = page.locator(".overlay g.bracket text.label");
  // Two occurrences of one motif, one of them transposed (§1).
  await expect(labels).toHaveCount(2);
  await expect(labels.nth(0)).toHaveText("sigh()");
  await expect(labels.nth(1)).toHaveText("transpose down P5 ▸ sigh()");
});

test("hovering a generated note draws one trace to its bracket", async ({
  page,
}) => {
  await inScore(page);
  await page.keyboard.down("o");
  await expect(page.locator(".overlay g.bracket").first()).toBeVisible();

  await expect(page.locator(".overlay line.trace")).toHaveCount(0);
  await page
    .locator('.engraving .arriving [id="event-4"]')
    .first()
    .hover({ force: true });
  // One line, never two: a trace per notehead would be a diagram, not an answer.
  await expect(page.locator(".overlay line.trace")).toHaveCount(1);
});

test("clicking a generated note while held selects the whole occurrence", async ({
  page,
}) => {
  await inScore(page);
  await page.keyboard.down("o");
  await page
    .locator('.engraving .arriving [id="event-4"]')
    .first()
    .click({ force: true });

  // The five notes one `use sigh()` produced, not the notehead under the
  // pointer (§2) — six haloes, because one of them is tied across a barline
  // and is therefore drawn twice.
  await expect(page.locator(".overlay rect.selection")).toHaveCount(6);
  await page.keyboard.up("o");
  await expect(page.locator(".overlay rect.selection")).toHaveCount(6);
});

test("a bracket is itself the control for its occurrence", async ({ page }) => {
  await inScore(page);
  await page.keyboard.down("o");
  const bracket = page.locator(".overlay g.bracket rect.hit").nth(1);
  await bracket.click({ force: true });
  await expect(page.locator(".overlay rect.selection")).toHaveCount(6);
  await expect(page.locator(".inspector")).toContainText("transpose down P5");
});

test("the lens can be pinned for anyone who cannot hold a key", async ({
  page,
}) => {
  const toggle = page.getByRole("button", { name: "Origin" });
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".overlay g.bracket").first()).toBeVisible();

  // Pinned means kept: it survives the key it does not depend on.
  await inScore(page);
  await page.keyboard.down("o");
  await page.keyboard.up("o");
  await expect(page.locator(".overlay g.bracket").first()).toBeVisible();

  await toggle.click();
  await expect(page.locator(".overlay g.bracket")).toHaveCount(0);
});

test("the source says where too, and the parts list dims what is authored", async ({
  page,
}) => {
  await toggleSource(page);
  await page
    .locator('.engraving .arriving [id="event-4"]')
    .first()
    .click({ force: true });

  // The declaration and the use statement, both marked, neither invented here.
  await expect.poll(() => marked(page)).not.toHaveLength(0);
  const marks = await marked(page);
  expect(marks.join("\n")).toContain("motif sigh");
  expect(marks.at(-1)).toBe("use sigh();");

  await inScore(page);
  await page.keyboard.down("o");
  // Strings are authored; the violin's lead is not.
  await expect(page.locator(".parts .voice.aside").first()).toBeVisible();
  await expect(page.locator(".parts .voice.generated.aside")).toHaveCount(0);
});

test("the origin row's line number opens the source at the use statement", async ({
  page,
}) => {
  await page
    .locator('.engraving .arriving [id="event-4"]')
    .first()
    .click({ force: true });
  await page.locator(".inspector button.segment.line").click();

  await expect(source(page)).toBeVisible();
  await expect.poll(() => selected(page)).toBe("use sigh();");
});

test("a diagnostic is a place in the source, not a notification", async ({
  page,
}) => {
  await toggleSource(page);
  await rewrite(page, 'piece "Glass Mountain" {');
  // Wait for the *compiler's* answer, not for a list of the right length: the
  // piece opens with warnings of its own, and a count alone cannot tell them
  // from the error the edit just caused.
  const text = 'piece "Glass Mountain" {';
  await expect(page.locator(".diagnostics li")).toHaveCount(1);
  // Backticks are the compiler's spelling; the page sets what they quote
  // in the mono face instead of printing them.
  await expect(page.locator(".diagnostics .message")).toHaveText("missing }");

  // The place is stated the way a person says it — `1:25`, never `24`.
  await expect(page.locator(".diagnostics .where")).toHaveText(
    `1:${text.length + 1}`,
  );
  await page.locator(".diagnostics button.problem").click();

  // The caret goes where the compiler is pointing, and the field keeps focus
  // so the composer can simply type the fix (`05-states.md` §5).
  await expect(source(page)).toBeFocused();
  await expect.poll(() => caret(page)).toBe(text.length);
});

for (const theme of ["light", "dark"] as const) {
  test(`origin view ${theme}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: theme });
    await inScore(page);
    await page.keyboard.down("o");
    await expect(page.locator(".overlay g.bracket").first()).toBeVisible();
    await expect(page).toHaveScreenshot(`origin-${theme}.png`);
  });
}
