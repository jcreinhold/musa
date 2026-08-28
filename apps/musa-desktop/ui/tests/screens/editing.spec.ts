/**
 * Editing what is already written (`03-interaction.md` §1, `04-provenance.md` §4).
 *
 * The score the composer edits here is Glass Mountain, where ten of the
 * violin's notes come from two occurrences of one motif — so the same
 * gesture, changing a note's pitch, is an ordinary edit in one place and a
 * question about generated music in another. That difference is the whole
 * feature.
 *
 * Nothing here writes a note that was not there. Notes arrive by playing them
 * and keeping the reading (`capture.spec.ts`) or by typing source; the score
 * page edits, extracts, and recovers.
 *
 * What these tests assert is the *commands the interface issued*: the stub is
 * not a compiler, and what a command does to the source is asserted by
 * `musa-syntax`'s and `musa-project`'s editing laws.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";

/** Focus the score pane, as `Tab` from the top of the document would. */
async function inScore(page: Page): Promise<void> {
  await page.getByRole("application", { name: "Engraved score" }).focus();
}

/** Every edit the interface has asked the core for, in order. */
async function edits(page: Page): Promise<Record<string, unknown>[]> {
  return page.evaluate(() => window.__musaEdits);
}

/**
 * Wait until the interface has asked for `count` of them. Every edit is a
 * round trip — the impact is asked for before the edit is sent — so the
 * assertions that follow have to let the gesture land first.
 */
async function settled(page: Page, count: number): Promise<void> {
  await expect.poll(() => page.evaluate(() => window.__musaEdits.length)).toBe(count);
}

/**
 * Select a note and respell it from the inspector, which is the pointer-free
 * way to change one pitch: the field takes the language's own spelling and
 * the core decides what it means.
 */
async function respell(page: Page, event: string, pitch: string): Promise<void> {
  await page.locator(`.engraving [id="${event}"] use`).click({ force: true });
  await page.getByRole("textbox", { name: "Pitch" }).fill(pitch);
  await page.keyboard.press("Enter");
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.goto("/");
  await engraved(page);
});

test("editing a generated note asks first, and states what it would change", async ({ page }) => {
  // `event-0` is the first note of the first `sigh()` — the case
  // `04-provenance.md` §4 is written about.
  await respell(page, "event-0", "g5");

  const choice = page.getByRole("group", { name: "Editing generated music" });
  await expect(choice).toBeVisible();
  // The count is the number of notes that *change* — one per occurrence, not
  // the ten notes the two expansions contain.
  await expect(choice).toContainText("changes 2 occurrences, 2 notes");
  await expect(choice).toContainText("sigh(e5)");
  // Nothing has been asked of the core yet: the question comes first.
  expect(await edits(page)).toHaveLength(0);
  // And the notes the count names are the notes that are haloed, so the
  // consequence can be read off the page rather than believed.
  await expect(page.locator(".overlay rect.selection")).toHaveCount(2);

  // Both answers are live: this call runs once, so it can carry an override.
  await expect(choice.getByRole("button", { name: /Just this occurrence/ })).toBeEnabled();

  await choice.getByRole("button", { name: /Edit the motif/ }).click();
  await expect(choice).toBeHidden();

  await settled(page, 1);
  const asked = await edits(page);
  expect(asked[0]).toMatchObject({
    kind: "changePitch",
    event: "event-0",
    pitch: "g5",
    mode: "editDefinition",
  });
  // And it is reported in the composer's words, not the command's.
  await expect(page.locator("p.notice")).toContainText("2 occurrences updated");
});

test("cancelling the choice changes nothing and puts the selection back", async ({ page }) => {
  await respell(page, "event-0", "g5");

  const choice = page.getByRole("group", { name: "Editing generated music" });
  await choice.getByRole("button", { name: "Cancel" }).click();

  await expect(choice).toBeHidden();
  expect(await edits(page)).toHaveLength(0);
  await expect(page.locator(".overlay rect.selection")).toHaveCount(1);
});

test("the other answer changes this occurrence and says which", async ({ page }) => {
  await respell(page, "event-0", "g5");

  const choice = page.getByRole("group", { name: "Editing generated music" });
  await choice.getByRole("button", { name: /Just this occurrence/ }).click();
  await expect(choice).toBeHidden();

  await settled(page, 1);
  const asked = await edits(page);
  expect(asked[0]).toMatchObject({
    kind: "changePitch",
    event: "event-0",
    pitch: "g5",
    mode: "specialize",
  });
  await expect(page.locator("p.notice")).toContainText("one note changed");
});

test("a range is lifted into a motif, named where the notes are", async ({ page }) => {
  // The cello's last three notes, which are authored: extraction is about
  // material a composer wrote twice, not about generated music.
  await page.locator('.engraving [id="event-e"] use').click({ force: true });
  await page.locator('.engraving [id="event-10"] use').click({ force: true, modifiers: ["Shift"] });
  await inScore(page);
  await page.keyboard.press("m");

  // A field in the margin, not a dialog: the notes it covers stay on screen
  // and stay haloed while the composer decides what to call them.
  const form = page.getByRole("form", { name: "Name this motif" });
  await expect(form).toBeVisible();
  await expect(form).toContainText("Extract 3 notes into a motif.");
  await expect(page.locator(".overlay rect.selection")).toHaveCount(3);

  await form.getByRole("textbox", { name: "Motif name" }).fill("cadence");
  await page.keyboard.press("Enter");

  await settled(page, 1);
  const [asked] = await edits(page);
  expect(asked).toMatchObject({
    kind: "extractMotif",
    name: "cadence",
    events: ["event-e", "event-f", "event-10"],
  });
  await expect(page.locator("p.notice")).toContainText("Extracted cadence()");
});

test("an edit is undone like any other revision", async ({ page }) => {
  const before = await page.evaluate(() => window.__musaRevision);

  // The cello's own note: authored music, so it is edited without a question.
  await respell(page, "event-e", "a3");
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBeGreaterThan(before);

  await inScore(page);
  await page.keyboard.press("Meta+z");
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBe(before);
});

test("work a crash left behind is offered, and taking it is one command", async ({ page }) => {
  const recovered = 'piece "Glass Mountain" {\n}\n';
  await page.evaluate((source) => window.__musaSet({ recovery: source }), recovered);

  const banner = page.getByRole("group", {
    name: "Unsaved work from the last session",
  });
  await expect(banner).toBeVisible();

  await banner.getByRole("button", { name: "Restore it" }).click();
  // The offer is gone and the recovered text is the document — one command,
  // and an ordinary undoable one.
  await expect(banner).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBeGreaterThan(0);
});

test("declining the offer keeps the file's own text", async ({ page }) => {
  await page.evaluate(() => window.__musaSet({ recovery: 'piece "Other" {}' }));
  const banner = page.getByRole("group", {
    name: "Unsaved work from the last session",
  });
  await banner.getByRole("button", { name: "Discard it" }).click();
  await expect(banner).toHaveCount(0);
  // The frame's title, not the engraved one: the page now prints the piece's
  // name too, and the question here is which document the session is holding.
  await expect(page.getByRole("heading", { name: "Glass Mountain" })).toBeVisible();
});

test("unsaved work says whether it is kept, and a saved piece says nothing", async ({ page }) => {
  // The fixture is a piece with unsaved edits and no recovery copy yet.
  await expect(page.getByText("Unsaved", { exact: true })).toBeVisible();

  await page.evaluate(() => window.__musaSet({ unsaved: true, autosaved: true }));
  await expect(page.getByText("Unsaved — recovery copy kept")).toBeVisible();

  // Saving is the only state that needs no words: the file has the work.
  await page.evaluate(() => window.__musaSet({ unsaved: false, autosaved: false }));
  await expect(page.getByText(/^Unsaved/)).toHaveCount(0);
});
