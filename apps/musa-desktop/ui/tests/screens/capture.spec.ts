/**
 * Capture and Keep that, end to end: played, read, accepted, written.
 *
 * The claim under test is the one prompt 208 closes. A played phrase is a
 * performance until a composer says otherwise, so nothing is written by
 * capturing, by reading, or by accepting — the source moves exactly once,
 * when the phrase is kept, and one undo puts it back. Everything before that
 * point is evidence the project holds and can drop.
 *
 * That the text it writes is the right text is `musa-project`'s placement
 * laws. What is judged here is the flow: which door leads where, what each
 * state says, and that a refusal changes nothing at all.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { reading } from "./reviewing";
import { stubShell } from "./shell";

/** The capture facts a keyboard that is plugged in and quiet would give. */
const LISTENING = {
  state: "listen",
  recentEnabled: true,
  recentEvents: 0,
  recentMicros: 0,
  recentTruncated: false,
  captureEvents: 0,
  captureMicros: 0,
  recentEventLimit: 4096,
  recentTimeLimitMicros: 30_000_000,
  captureEventLimit: 65_536,
  captureTimeLimitMicros: 600_000_000,
  unsupportedAuditionEvents: 0,
  losses: { queueOverflow: 0, refusedSysex: 0, malformed: 0, unsupportedSystem: 0 },
} as const;

/** Put a keyboard on the capture line, in whatever state the test needs. */
async function keyboard(page: Page, patch: Record<string, unknown> = {}): Promise<void> {
  await engraved(page);
  await page.evaluate((facts) => window.__musaSet({ midiPort: "Stub Keyboard", midiCapture: facts }), {
    ...LISTENING,
    ...patch,
  } as Record<string, unknown>);
}

/** The revision the stub last answered with. */
function revision(page: Page): Promise<number> {
  return page.evaluate(() => window.__musaRevision);
}

/** Seed a take and open it, from whichever door the test came through. */
async function take(page: Page, which: "capture" | "keep that", facts = reading("straight-known")): Promise<void> {
  await page.evaluate((seed) => window.__musaReview(seed), facts);
  if (which === "capture") {
    await page.getByRole("button", { name: "Capture", exact: true }).click();
    await page.getByRole("button", { name: "Finish" }).click();
  } else {
    await page.getByRole("button", { name: "Keep that" }).click();
  }
  await page.getByRole("button", { name: "Review the take" }).click();
  await expect(page.getByRole("heading", { name: "Review" })).toBeVisible();
}

test.beforeEach(async ({ page }) => {
  await stubShell(page);
});

// ------------------------------------------------------------- the whole path

test("a phrase is played, read, accepted, and only then written", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  const before = await revision(page);

  await take(page, "capture");
  // Capturing and reading write nothing. The take is evidence the project
  // holds, and evidence is not notation.
  expect(await revision(page)).toBe(before);

  await page.getByRole("button", { name: "Accept" }).click();
  // Accepting settles the notation and says where it would go — and still
  // writes nothing, so there is nothing yet to undo.
  await expect(page.getByRole("status")).toContainText("4 notes in 1 bar into p’s v");
  await expect(page.getByRole("group", { name: "Where this phrase goes" })).toContainText("into p’s v");
  expect(await revision(page)).toBe(before);
  expect(await page.evaluate(() => window.__musaEdits.length)).toBe(0);

  await page.getByRole("button", { name: "Keep", exact: true }).click();
  // One revision, and the review is over: there is no take any more, because
  // the notes are ordinary source originating where they were written.
  await expect(page.getByRole("heading", { name: "Review" })).toBeHidden();
  expect(await revision(page)).toBe(before + 1);
  // The phrase is selected, so the next thing typed continues it.
  await expect(page.locator(".overlay rect.selection")).toHaveCount(4);
});

test("Keep that reaches the same phrase by the same path", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });

  await take(page, "keep that");
  await page.getByRole("button", { name: "Accept" }).click();
  // The differential law, as the interface can see it: the phrase that
  // arrived by the recent buffer is written into the same line, in the same
  // words, as the phrase that arrived by an explicit capture.
  await expect(page.getByRole("status")).toContainText("4 notes in 1 bar into p’s v");
});

test("discarding drops the take and leaves the piece alone", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  const before = await revision(page);

  await take(page, "capture");
  await page.getByRole("button", { name: "Discard" }).click();

  await expect(page.getByRole("heading", { name: "Review" })).toBeHidden();
  expect(await revision(page)).toBe(before);
});

test("one undo puts the piece back where it was", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  const before = await revision(page);

  await take(page, "capture");
  await page.getByRole("button", { name: "Accept" }).click();
  await page.getByRole("button", { name: "Keep", exact: true }).click();
  await expect.poll(() => revision(page)).toBe(before + 1);

  await page.keyboard.press("Meta+z");
  // Undo stores source revisions, not takes: what comes back is the piece,
  // and the phrase that was kept is simply not in it.
  await expect.poll(() => revision(page)).toBe(before);
});

// ------------------------------------------------------------ what it says

test("the keyboard says it never writes notes, and it never does", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 3, recentMicros: 900_000 });

  // A keyboard auditions whenever it is safely connected — there is no mode
  // to arm — and it is named where the composer is looking rather than in a
  // settings pane they would have to find. The line says what it is for, so
  // nobody has to discover by playing that the notes are not going anywhere.
  await expect(page.getByText(/Stub Keyboard$/)).toBeVisible();
  await expect(page.locator("span.port")).toHaveAttribute("title", /never writes notes/);

  // Playing it writes nothing. A played note is a performance and a written
  // note is notation, and the interface never silently turns one into the
  // other: Capture holds the take and Review is where it becomes source.
  await expect(page.locator("button.recent")).toContainText("Recent phrase on \u00b7 0.9 s");
  expect(await page.evaluate(() => window.__musaEdits.length)).toBe(0);
});

test("the capture line says which clock the take will get", async ({ page }) => {
  await page.goto("/");
  await keyboard(page);

  // Played free, Review has to ask for the pulse; counted in by the
  // transport, the take carries the piece's own.
  const capture = page.getByRole("button", { name: "Capture", exact: true });
  await expect(capture).toHaveAttribute("title", /Played free/);
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await expect(capture).toHaveAttribute("title", /Counted in/);
});

test("an empty recent buffer says what to do instead of going quiet", async ({ page }) => {
  await page.goto("/");
  await keyboard(page);

  const keep = page.getByRole("button", { name: "Keep that" });
  await expect(keep).toBeDisabled();
  await expect(keep).toHaveAttribute("title", /play a phrase/);
  await expect(page.locator("button.recent")).toContainText("play something");
});

test("an overflowed buffer says it is a suffix, and still keeps", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 4096, recentMicros: 30_000_000, recentTruncated: true });

  // It never guesses by deleting opening notes silently: the line says the
  // phrase begins where the memory does.
  await expect(page.locator("button.recent")).toContainText("Recent suffix only");
  await expect(page.getByRole("button", { name: "Keep that" })).toBeEnabled();
});

test("a keyboard that goes away says the take survived it", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  await take(page, "capture");
  await page.getByRole("button", { name: "Discard" }).click();

  await page.evaluate(() => window.__musaSet({ midiPort: null, midiCapture: { state: "disconnected" } }));
  await expect(page.getByText("Keyboard disconnected — capture preserved")).toBeVisible();
});

test("a free take is where the pulse is adjusted, not the capture line", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  await take(page, "capture", reading("rubato-free"));

  // The boundary a musician adjusts is adjusted against what they hear, so
  // it lives beside the audition and not beside the Capture button.
  await expect(page.getByRole("button", { name: "Tap the pulse" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Played", exact: true })).toBeVisible();
});

// ------------------------------------------------------------ what it refuses

test("a piece that moved on refuses the phrase and keeps the take", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  const before = await revision(page);
  await take(page, "capture", { ...reading("straight-known"), current: false });

  await expect(page.getByText("The piece changed while you were reading this.")).toBeVisible();
  // Accept is refused too: a reading of a document nobody has is not a
  // reading anybody can keep.
  await expect(page.getByRole("button", { name: "Accept" })).toBeDisabled();
  expect(await revision(page)).toBe(before);
});

test("a phrase that would not compile changes nothing, and can be kept after", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  const before = await revision(page);
  await take(page, "capture");
  await page.getByRole("button", { name: "Accept" }).click();

  await page.evaluate(() => window.__musaRefusePlacement("the phrase would not compile there: no such part"));
  await page.getByRole("button", { name: "Keep", exact: true }).click();

  // The source, the undo stack, and the review are all exactly as they were.
  await expect(page.getByRole("heading", { name: "Review" })).toBeVisible();
  await expect(page.getByRole("alert")).toContainText("would not compile there");
  expect(await revision(page)).toBe(before);

  // And the way out is the same button: a refusal is a state, not a dead end.
  await page.getByRole("button", { name: "Keep", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Review" })).toBeHidden();
  await expect.poll(() => revision(page)).toBe(before + 1);
});

// ------------------------------------------------------------- naming a line

test("a phrase in two lines is named before it is written", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  await take(page, "capture", reading("crossing-voices"));

  await page.getByRole("button", { name: "Accept" }).click();
  const where = page.getByRole("group", { name: "Where this phrase goes" });
  // The line the caret was in, and one the part does not have yet.
  await expect(page.getByRole("status")).toContainText("adding one line");
  await expect(page.getByRole("textbox", { name: "Line 1" })).toHaveValue("v");
  await expect(page.getByRole("textbox", { name: "Line 2" })).toHaveValue("captured");
  await expect(where).toContainText("into a new line");
});

/**
 * The one new picture prompt 208 adds: the phrase is settled, and the last
 * question is which line of the part each voice joins.
 */
test("review keeping", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  await take(page, "capture", reading("crossing-voices"));

  await page.getByRole("button", { name: "Accept" }).click();
  await expect(page.getByRole("group", { name: "Where this phrase goes" })).toBeVisible();
  await expect(page).toHaveScreenshot("review-keeping-1440x900-light.png");
});

test("two lines pointed at one name are refused before anything is written", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  const before = await revision(page);
  await take(page, "capture", reading("crossing-voices"));
  await page.getByRole("button", { name: "Accept" }).click();

  await page.getByRole("textbox", { name: "Line 2" }).fill("v");
  await expect(page.getByRole("alert")).toContainText("both pointed at `v`");
  // The naming stays on screen: taking it away from the person typing is
  // how a typo becomes a dead end.
  await expect(page.getByRole("textbox", { name: "Line 2" })).toHaveValue("v");
  expect(await revision(page)).toBe(before);

  await page.getByRole("textbox", { name: "Line 2" }).fill("lower");
  await expect(page.getByRole("status")).toContainText("v and lower");
  await page.getByRole("button", { name: "Keep", exact: true }).click();
  await expect.poll(() => revision(page)).toBe(before + 1);
});

test("a name the language cannot write is refused before the parser sees it", async ({ page }) => {
  await page.goto("/");
  await keyboard(page, { recentEvents: 12, recentMicros: 4_000_000 });
  await take(page, "capture", reading("crossing-voices"));
  await page.getByRole("button", { name: "Accept" }).click();

  await page.getByRole("textbox", { name: "Line 2" }).fill("2 lower");
  await expect(page.getByRole("alert")).toContainText("is not a name a voice can have");
});
