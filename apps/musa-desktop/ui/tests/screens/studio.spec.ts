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

test("⌘2 shows the chosen part's patch as the chain the source spells", async ({
  page,
}) => {
  await inSound(page);

  // Glass Mountain's `glass_pad`, in the order `|>` runs it. Not a canvas:
  // a list, top to bottom, matching the text.
  const stages = page.locator(".sound-workspace .chain .processor");
  await expect(stages).toHaveText([
    "oscillator",
    "oscillator",
    "gain",
    "mix",
    "envelope",
    "lowpass",
  ]);

  // The values are the compiler's, in the units the language writes them in.
  await expect(page.locator(".sound-workspace .chain")).toContainText("1400");
  await expect(page.locator(".sound-workspace .chain")).toContainText("Hz");

  // And the one the `lfo` moves says so, rather than showing a control that
  // appears to disagree with what is heard (§13.7).
  await expect(page.locator(".sound-workspace .modulated")).toContainText(
    "modulated by lfo",
  );
});

test("a part says which patch plays it, and can be pointed at another", async ({
  page,
}) => {
  await inSound(page);

  await expect(page.locator(".sound-workspace .parts")).toContainText("violin");
  await expect(page.locator(".sound-workspace .parts")).toContainText(
    "glass_pad",
  );

  // One patch is declared, so the picker offers it and nothing invented.
  const picker = page.locator(".sound-workspace .picker select");
  await expect(picker).toHaveValue("glass_pad");
  await expect(picker.locator("option")).toHaveText(["glass_pad"]);
});

test("moving a parameter issues an edit against the source, not against a copy", async ({
  page,
}) => {
  await inSound(page);

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
  await expect(strips).toHaveText(["violin", "strings", "hall"]);

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
