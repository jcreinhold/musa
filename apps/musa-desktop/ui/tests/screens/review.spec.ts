/**
 * Reviewing a take, photographed and driven.
 *
 * Progressive disclosure is the claim under test: a reading the search is
 * sure of shows as ordinary notation and is kept in one press, and a mark
 * appears only where the readings that survived actually disagree. The five
 * readings here are the project's own — `musa-desktop`'s review generator
 * composes them from prompt 203's measured corpus — so a control with no
 * measured ambiguity behind it has no fixture to appear in.
 *
 * What the accepted reading writes is `musa-project`'s review laws, not this
 * file's: nothing here touches the source, because Review does not either.
 */

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

import { stubShell } from "./shell";
import { openReview as review } from "./reviewing";

/** Every gesture the interface has made, in order. */
function acted(page: Page): Promise<Record<string, unknown>[]> {
  return page.evaluate(() => window.__musaReviewActs);
}

function choices(page: Page) {
  return page.getByRole("complementary", { name: "Choices" });
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
});

// ---------------------------------------------------------------- photographs

const SIZES = [
  { name: "1440x900", width: 1440, height: 900 },
  { name: "1100x720", width: 1100, height: 720 },
] as const;

/**
 * The clean case at both widths and both themes: this is what "keeping a take
 * is one press" looks like, and the diff says when it stops looking like it.
 */
for (const size of SIZES) {
  for (const theme of ["light", "dark"] as const) {
    test(`review clean ${size.name} ${theme}`, async ({ page }) => {
      await page.setViewportSize({ width: size.width, height: size.height });
      await page.emulateMedia({ colorScheme: theme });
      await page.goto("/");
      await review(page, "straight-known");
      await expect(page).toHaveScreenshot(`review-clean-${size.name}-${theme}.png`);
    });
  }
}

/** One photograph per ambiguity class the corpus measured, with its mark open. */
const CLASSES = [
  { reading: "swing-known", name: "placement" },
  { reading: "rolled-and-block-chords", name: "onset-group" },
  { reading: "crossing-voices", name: "voice" },
  { reading: "rubato-free", name: "pulse" },
] as const;

for (const asked of CLASSES) {
  test(`review ${asked.name}`, async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/");
    await review(page, asked.reading);
    // The first mark, opened: a mark that is never opened is a mark whose
    // readings the photograph cannot show.
    await page.keyboard.press("n");
    await expect(page).toHaveScreenshot(`review-${asked.name}.png`);
  });
}

/** 200 % text, where a fixed layout loses the margin or the leaf. */
test("review at 200 % text", async ({ page }) => {
  await page.setViewportSize({ width: 720, height: 450 });
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  await review(page, "crossing-voices");

  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow, "horizontal overflow in CSS pixels").toBeLessThanOrEqual(1);
  await expect(page).toHaveScreenshot("review-200-percent.png");
});

/** Reduced motion: the screen is the same screen, and nothing is animating. */
test("review with reduced motion", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
  await page.goto("/");
  await review(page, "swing-known");
  await expect(page).toHaveScreenshot("review-reduced-motion.png");
});

// ------------------------------------------------------------------- behaviour

test("a reading the search is sure of asks nothing and is kept in one press", async ({ page }) => {
  await page.goto("/");
  await review(page, "straight-known");

  await expect(choices(page)).toContainText("Nothing to decide — this reads one way");
  await expect(choices(page).getByRole("button", { name: /^bar/ })).toHaveCount(0);

  await page.getByRole("button", { name: "Accept", exact: true }).click();
  await expect(page.getByText("Kept — ready to place.")).toBeVisible();
  // Keeping settles what the phrase is. Placing it is prompt 208's
  // transaction, and no source has moved here.
  expect(await page.evaluate(() => window.__musaEdits)).toHaveLength(0);
});

test("a mark stands only where the surviving readings disagree, and names them", async ({ page }) => {
  await page.goto("/");
  await review(page, "swing-known");

  const marks = choices(page).getByRole("button", { expanded: false });
  await expect(marks.first()).toContainText("disagree about where this note falls");

  await marks.first().click();
  // Two readings of the same played note, in the project's own words —
  // no percentage, no score, no "confidence".
  const offered = choices(page).getByRole("button", { name: /^bar 1,/ });
  await expect(offered).toHaveCount(2);
  await expect(choices(page)).not.toContainText("%");
});

test("choosing a reading settles that mark and records the decision", async ({ page }) => {
  await page.goto("/");
  await review(page, "swing-known");

  const before = await choices(page).getByRole("button", { expanded: false }).count();
  await choices(page).getByRole("button", { expanded: false }).first().click();
  await choices(page)
    .getByRole("button", { name: /^bar 1,/ })
    .first()
    .click();

  await expect(choices(page).getByRole("button", { expanded: false })).toHaveCount(before - 1);
  await expect(choices(page).getByRole("group", { name: "What you decided" })).toBeVisible();
  await expect.poll(() => acted(page)).toHaveLength(1);
});

test("taking a decision back restores exactly the reading before it", async ({ page }) => {
  await page.goto("/");
  await review(page, "swing-known");

  const marks = () => choices(page).getByRole("button", { expanded: false });
  const before = await marks().count();
  await marks().first().click();
  await choices(page)
    .getByRole("button", { name: /^bar 1,/ })
    .first()
    .click();
  await expect(marks()).toHaveCount(before - 1);

  await page.getByRole("button", { name: "Take back" }).click();
  await expect(marks()).toHaveCount(before);
});

test("a cluster is asked about as music, not as a distance", async ({ page }) => {
  await page.goto("/");
  await review(page, "rolled-and-block-chords");

  await choices(page).getByRole("button", { expanded: false }).first().click();
  const cluster = choices(page).getByRole("group", { name: "What this cluster is" });
  await expect(cluster.getByRole("button", { name: "Make chord" })).toBeVisible();
  await expect(cluster.getByRole("button", { name: "Keep rolled" })).toBeVisible();

  await cluster.getByRole("button", { name: "Make chord" }).click();
  await expect.poll(() => acted(page)).toMatchObject([{ kind: "makeChord" }]);
});

test("crossing lines are assigned by line, and the lines are the four the core has", async ({ page }) => {
  await page.goto("/");
  await review(page, "crossing-voices");

  await choices(page)
    .getByRole("button", { name: /two lines cross here/ })
    .click();
  const lines = choices(page).getByRole("group", { name: "Assign a line" });
  await expect(lines.getByRole("button")).toHaveCount(4);
  await lines.getByRole("button", { name: "Line 2" }).click();
  await expect.poll(() => acted(page)).toMatchObject([{ kind: "assignVoice", voice: 1 }]);
});

test("a free take asks for the pulse, and taps answer it", async ({ page }) => {
  await page.goto("/");
  await review(page, "rubato-free");

  await expect(choices(page)).toContainText("tap a few beats while you listen");

  await page.getByRole("button", { name: "Tap the pulse" }).click();
  await page.getByRole("button", { name: "Tap", exact: true }).click();
  await page.getByRole("button", { name: /^Read \d+ taps$/ }).click();

  await expect.poll(() => acted(page)).toMatchObject([{ kind: "tap", downbeat: 0 }]);
  await expect(choices(page)).toContainText("Nothing to decide");
});

test("a take with no exact written form cannot be kept, and says why", async ({ page }) => {
  await page.goto("/");
  await review(page, "rubato-free");

  await expect(page.getByRole("button", { name: "Accept", exact: true })).toBeDisabled();
  await expect(choices(page).getByRole("group", { name: "What could not be written" })).toBeVisible();
});

test("audition is A or B, never both, and decides nothing", async ({ page }) => {
  await page.goto("/");
  await review(page, "straight-known");

  const hearing = page.getByRole("group", { name: "What to hear" });
  await expect(hearing.getByRole("button", { name: "Played" })).toHaveAttribute("aria-pressed", "true");
  await hearing.getByRole("button", { name: "Written" }).click();
  await expect(hearing.getByRole("button", { name: "Written" })).toHaveAttribute("aria-pressed", "true");
  await expect(hearing.getByRole("button", { name: "Played" })).toHaveAttribute("aria-pressed", "false");
  // Hearing it the other way is not a decision.
  await expect(choices(page).getByRole("group", { name: "What you decided" })).toHaveCount(0);
});

test("discarding drops the take and puts the workspace back", async ({ page }) => {
  await page.goto("/");
  await review(page, "straight-known");

  await page.getByRole("button", { name: "Discard" }).click();
  await expect(page.getByRole("heading", { name: "Review" })).toHaveCount(0);
  await expect(page.getByRole("application", { name: "Engraved score" })).toBeVisible();
});

// ------------------------------------------------------------------- keyboard

test("every review gesture is reachable from the keyboard alone", async ({ page }) => {
  await page.goto("/");
  await review(page, "swing-known");

  // `n` walks the questions; `t` opens the interval field on a selection.
  await page.keyboard.press("n");
  await expect(choices(page).getByRole("button", { expanded: true })).toHaveCount(1);

  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("8");
  await expect.poll(() => acted(page)).toMatchObject([{ kind: "transform" }]);

  await page.keyboard.press("u");
  await expect(choices(page).getByRole("group", { name: "What you decided" })).toHaveCount(0);

  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("t");
  await expect(page.getByLabel("Interval")).toBeFocused();
  await page.keyboard.type("up P5");
  await page.keyboard.press("Enter");
  await expect.poll(() => acted(page).then((all) => all.at(-1))).toMatchObject({ kind: "transform" });
});

test("focus survives the proposal being replaced", async ({ page }) => {
  await page.goto("/");
  await review(page, "swing-known");

  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  const before = await page.evaluate(() => document.activeElement?.id ?? "");
  await page.keyboard.press("8");
  // The take is immutable, so note 2 is note 2 before and after the decision.
  await expect(page.locator(`#${before}`)).toHaveAttribute("aria-pressed", "true");
});

// -------------------------------------------------------------- screen reader

test("every proposal note has a musical name, and a questioned one says so", async ({ page }) => {
  await page.goto("/");
  await review(page, "swing-known");

  const notes = page.getByRole("group", { name: "The proposed notation" }).getByRole("button");
  expect(await notes.count()).toBeGreaterThan(0);
  for (const name of await notes.evaluateAll((all) => all.map((each) => each.getAttribute("aria-label") ?? ""))) {
    // A name a reader can act on: pitch, written value, line, bar position.
    // Pitch, written value, line, bar position — spelled the way the
    // language spells them, so what is read out is what could be written.
    expect(name).toMatch(/^[a-g][sf]*-?\d+, \d+\/\d+\.?, line \d+, bar \d+/);
  }
  expect(
    await notes.evaluateAll((all) =>
      all.some((each) => (each.getAttribute("aria-label") ?? "").includes("needs a choice")),
    ),
  ).toBe(true);
});

for (const theme of ["light", "dark"] as const) {
  test(`review has no accessibility violations in ${theme}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: theme });
    await page.goto(`/?theme=${theme}`);
    await review(page, "crossing-voices");

    const { violations } = await new AxeBuilder({ page }).disableRules(["svg-img-alt"]).analyze();
    expect(violations.map((violation) => `${violation.id}: ${violation.help}`)).toEqual([]);
  });
}
