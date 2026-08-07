/**
 * Writing music, from the keyboard (`03-interaction.md` §3, prompt 25).
 *
 * The score the composer edits here is Glass Mountain, where ten of the
 * violin's notes come from two occurrences of one motif — so the same
 * gesture, a letter key, is an ordinary insertion in one place and a question
 * about generated music in another. That difference is the whole feature.
 *
 * What these tests assert is the *commands the interface issued*: the stub is
 * not a compiler, and what a command does to the source is asserted by
 * `musa-language`'s and `musa-project`'s editing laws.
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
 * assertions that follow have to let the keystrokes land first.
 */
async function settled(page: Page, count: number): Promise<void> {
  await expect.poll(() => page.evaluate(() => window.__musaEdits.length)).toBe(count);
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.goto("/");
  await engraved(page);
});

test("a four-note melody is typed, one letter a note", async ({ page }) => {
  await inScore(page);

  // `N` turns the letters into pitches. The mode is never invisible: the
  // duration the next note would take sits in the top margin the whole time.
  await page.keyboard.press("n");
  const indicator = page.getByRole("button", { name: /^Notes/ });
  await expect(indicator).toHaveAttribute("aria-pressed", "true");

  // A number is the duration, and it persists across the notes that follow —
  // a composer sets eighths once and types the phrase.
  await page.keyboard.press("8");
  for (const letter of ["c", "d", "e", "f"]) await page.keyboard.press(letter);

  await settled(page, 4);
  const asked = await edits(page);
  expect(asked.map((edit) => (edit.note as { pitch: string }).pitch)).toEqual([
    "c4",
    "d4",
    "e4",
    "f4",
  ]);
  for (const edit of asked) {
    expect(edit.kind).toBe("insertNote");
    expect(edit.note).toMatchObject({ kind: "note", duration: "1/8" });
  }

  // Escape leaves the mode rather than the selection: one key, the nearest
  // thing first.
  await page.keyboard.press("Escape");
  await expect(indicator).toHaveAttribute("aria-pressed", "false");
});

test("the octave and the accidental follow the letters until they are changed", async ({
  page,
}) => {
  await inScore(page);
  await page.keyboard.press("n");

  await page.keyboard.press("Meta+ArrowUp");
  await page.keyboard.press("Shift+ArrowUp");
  await page.keyboard.press("g");
  await page.keyboard.press("a");

  await settled(page, 2);
  const asked = await edits(page);
  expect(asked.map((edit) => (edit.note as { pitch: string }).pitch)).toEqual(["gs5", "as5"]);
});

test("editing a generated note asks first, and states what it would change", async ({ page }) => {
  // `event-0` is the first note of the first `sigh()` — the case
  // `04-provenance.md` §4 is written about.
  await page.locator('.engraving [id="event-0"] use').click({ force: true });
  await inScore(page);
  await page.keyboard.press("n");
  await page.keyboard.press("g");

  const choice = page.getByRole("group", { name: "Editing generated music" });
  await expect(choice).toBeVisible();
  // The count is the number of notes that *change* — one per occurrence, not
  // the ten notes the two expansions contain.
  await expect(choice).toContainText("changes 2 occurrences, 2 notes");
  await expect(choice).toContainText("sigh()");
  // Nothing has been asked of the core yet: the question comes first.
  expect(await edits(page)).toHaveLength(0);
  // And the notes the count names are the notes that are haloed, so the
  // consequence can be read off the page rather than believed.
  await expect(page.locator(".overlay rect.selection")).toHaveCount(2);

  // The second answer is honest about not existing yet rather than absent.
  await expect(choice.getByRole("button", { name: /Just this occurrence/ })).toBeDisabled();

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
  await page.locator('.engraving [id="event-0"] use').click({ force: true });
  await inScore(page);
  await page.keyboard.press("n");
  await page.keyboard.press("g");

  const choice = page.getByRole("group", { name: "Editing generated music" });
  await choice.getByRole("button", { name: "Cancel" }).click();

  await expect(choice).toBeHidden();
  expect(await edits(page)).toHaveLength(0);
  await expect(page.locator(".overlay rect.selection")).toHaveCount(1);
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

test("undo takes an entered note back", async ({ page }) => {
  await inScore(page);
  const before = await page.evaluate(() => window.__musaRevision);

  await page.keyboard.press("n");
  await page.keyboard.press("c");
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBeGreaterThan(before);

  // Undo is the ordinary one: an entered note is a document revision like any
  // other, not a special entry buffer that has to be committed.
  await page.keyboard.press("Meta+z");
  await expect.poll(() => page.evaluate(() => window.__musaRevision)).toBe(before);
});

test("a keyboard is named while entry is on, and its notes are written", async ({ page }) => {
  await inScore(page);
  await page.keyboard.press("n");

  // The keyboard is read while notes are being entered, and named where the
  // composer is looking — not announced in a settings pane they would have to
  // go and find.
  await expect(page.getByText("Stub Keyboard", { exact: true })).toBeVisible();

  await page.keyboard.press("8");
  await page.evaluate(() => window.__musaEmit("musa://midi", { pitches: ["ef4"] }));
  await settled(page, 1);
  const [written] = await edits(page);
  expect(written?.kind).toBe("insertNote");
  // The pitch is the core's spelling — the interface never decides whether a
  // black key is a sharp or a flat — and the duration is the one entry is set
  // to, because a keyboard cannot say how long a note is notated for.
  expect(written?.note).toEqual({ kind: "note", pitch: "ef4", duration: "1/8" });

  // Several keys held together arrive as one chord, already grouped.
  await page.evaluate(() => window.__musaEmit("musa://midi", { pitches: ["c4", "e4", "g4"] }));
  await settled(page, 2);
  const asked = await edits(page);
  expect(asked[1]?.note).toEqual({
    kind: "chord",
    pitches: ["c4", "e4", "g4"],
    duration: "1/8",
  });

  // Leaving entry stops the keyboard being read, and the name goes with it.
  await page.keyboard.press("Escape");
  await expect(page.getByText("Stub Keyboard", { exact: true })).toHaveCount(0);
});

test("notes played with entry off are not written", async ({ page }) => {
  await inScore(page);
  await page.evaluate(() => window.__musaEmit("musa://midi", { pitches: ["c4"] }));
  await expect.poll(() => page.evaluate(() => window.__musaEdits.length)).toBe(0);
});

test("work a crash left behind is offered, and taking it is one command", async ({ page }) => {
  const recovered = 'piece "Glass Mountain" {\n}\n';
  await page.evaluate((source) => window.__musaSet({ recovery: source }), recovered);

  const banner = page.getByRole("group", { name: "Unsaved work from the last session" });
  await expect(banner).toBeVisible();

  await banner.getByRole("button", { name: "Restore it" }).click();
  // The offer is gone and the recovered text is the document — one command,
  // and an ordinary undoable one.
  await expect(banner).toHaveCount(0);
  await expect
    .poll(() => page.evaluate(() => window.__musaRevision))
    .toBeGreaterThan(0);
});

test("declining the offer keeps the file's own text", async ({ page }) => {
  await page.evaluate(() => window.__musaSet({ recovery: 'piece "Other" {}' }));
  const banner = page.getByRole("group", { name: "Unsaved work from the last session" });
  await banner.getByRole("button", { name: "Discard it" }).click();
  await expect(banner).toHaveCount(0);
  await expect(page.getByText("Glass Mountain")).toBeVisible();
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
