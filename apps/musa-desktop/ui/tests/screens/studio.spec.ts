/**
 * The Sound and Mix workspaces (roadmap §14.4).
 *
 * One claim runs through all of it: these screens are structured editors of
 * studio source, not a second authority (§11). So every test checks both
 * halves of that — what is drawn is what the compiler resolved, and what a
 * control does is issue an edit against the document rather than change a
 * value the interface is keeping for itself.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";

test.beforeEach(async ({ page }) => {
  await stubShell(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await engraved(page);
});

async function inSound(page: Page): Promise<void> {
  await page.keyboard.press("ControlOrMeta+2");
  await expect(page.locator(".sound-workspace")).toBeVisible();
}

async function inMix(page: Page): Promise<void> {
  await page.keyboard.press("ControlOrMeta+3");
  await expect(page.locator(".mix-workspace")).toBeVisible();
}

/** The studio edits the interface has issued, in order. */
function edits(page: Page): Promise<Record<string, unknown>[]> {
  return page.evaluate(() => window.__musaEdits);
}

test("⌘2 shows the chosen part's patch as the chain the source spells", async ({ page }) => {
  await inSound(page);

  await page.locator(".sound-workspace .machine summary").click();

  // Glass Mountain's `glass_pad`, in the order `|>` runs it. Not a canvas:
  // a list, top to bottom, matching the text.
  const stages = page.locator(".sound-workspace .chain .processor");
  await expect(stages).toHaveText(["oscillator", "oscillator", "gain", "mix", "envelope", "lowpass"]);

  // The values are the compiler's, in the units the language writes them in.
  await expect(page.locator(".sound-workspace .chain")).toContainText("1400");
  await expect(page.locator(".sound-workspace .chain")).toContainText("Hz");

  // And the one the `lfo` moves says so, rather than showing a control that
  // appears to disagree with what is heard (§13.7).
  await expect(page.locator(".sound-workspace .modulated")).toContainText("modulated by lfo");
});

test("a part says which patch plays it, and can be pointed at another", async ({ page }) => {
  await inSound(page);

  await expect(page.locator(".sound-workspace .parts")).toContainText("violin");
  await expect(page.locator(".sound-workspace .parts")).toContainText("glass_pad");

  // The edition default and the source declaration are the checked choices.
  const picker = page.locator(".sound-workspace .picker select");
  await expect(picker).toHaveValue("glass_pad");
  await expect(picker.locator("option")).toHaveText(["glass_pad"]);
  await expect(page.locator(".sound-workspace .contract")).toContainText("written in source");
});

test("choosing an instrument writes the source-owned sound sentence", async ({ page }) => {
  await inSound(page);

  const picker = page.locator(".sound-workspace .picker select");
  await picker.selectOption("glass_pad");

  await expect
    .poll(async () => (await edits(page)).at(-1))
    .toMatchObject({
      kind: "chooseSound",
      part: "violin",
      instrument: "glass_pad",
    });
});

test("moving a parameter issues an edit against the source, not against a copy", async ({ page }) => {
  await inSound(page);
  await page.locator(".sound-workspace .machine summary").click();

  const cutoff = page.locator("#glass_pad-5-cutoff");
  await cutoff.fill("900");
  await cutoff.dispatchEvent("change");

  await expect
    .poll(async () => (await edits(page)).at(-1))
    .toMatchObject({
      kind: "setParam",
      container: "patch",
      name: "glass_pad",
      param: "cutoff",
    });
});

test("⌘3 shows every part with what it sends where", async ({ page }) => {
  await inMix(page);

  const strips = page.locator(".mix-workspace .strip-name");
  await expect(strips).toHaveText(["violin part output", "strings part output", "hall", "main"]);

  // The levels the piece wrote, on one scale — decibels — whichever unit
  // each was written in.
  await expect(page.locator(".mix-workspace")).toContainText("-18.0");
  await expect(page.locator(".mix-workspace")).toContainText("-14.0");
  // And where each signal goes, in the studio's own words.
  await expect(page.locator(".mix-workspace")).toContainText("master");
});

test("a send fader writes the level it was moved to", async ({ page }) => {
  await inMix(page);

  const fader = page.locator(".mix-workspace .send input[type=range]").first();
  await fader.fill("-6");
  await fader.dispatchEvent("change");

  await expect
    .poll(async () => (await edits(page)).at(-1))
    .toMatchObject({
      kind: "setSendLevel",
      source: "violin",
      bus: "hall",
      decibels: -6,
    });
});

test("recorded media and verified assets stay distinct from part outputs", async ({ browser }) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  await stubShell(page, "sound-workbench");
  await page.goto("/");
  await engraved(page);

  await inSound(page);
  await expect(page.locator(".sound-workspace .picker select")).toHaveValue("glass");
  await expect(page.locator(".sound-workspace .contract")).toContainText("neutral");
  await expect(page.locator(".sound-workspace .assets p")).toHaveCount(3);
  await expect(page.locator(".sound-workspace .assets")).toContainText("wav@1");
  await expect(page.locator(".sound-workspace .assets")).toContainText("sfz@1");
  await expect(page.locator(".sound-workspace .assets")).toContainText("sf2@1");

  await inMix(page);
  await expect(page.getByRole("heading", { name: "lead part output" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "pulse media source" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "recording media source" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "main", exact: true })).toBeVisible();
  await expect(page.locator(".mix-workspace")).toContainText("clip · loop");
  await expect(page.locator(".mix-workspace")).toContainText("fixed cue");
  await expect(page.locator(".mix-workspace")).toContainText("send to room");

  await expect(page).toHaveScreenshot("sound-mix-workbench.png", { animations: "disabled" });
  await context.close();
});
