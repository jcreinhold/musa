/**
 * Structure goldens for the engraved page (`02-engraving.md` §9).
 *
 * The raster goldens beside these say what the page looks like; these say what
 * it is made of. A structural diff is the one that is readable: "17 fewer
 * beams" is a sentence, a changed PNG is not, and a `color="black"` that crept
 * back in shows up here as a word rather than as a grey smear in a screenshot.
 *
 * The digest is taken from the sanitized SVG the worker actually produced, so
 * it covers the sanitizer, the id contract, and Verovio's own output together.
 * Refresh with `npx playwright test structure --update-snapshots` after a
 * deliberate change — and read the diff before committing it.
 */

import { expect, test, type Page } from "@playwright/test";

import { engraved } from "./engraved";

const FIXTURES = ["glass-mountain", "counterpoint", "twinkle"] as const;

/**
 * What the first page is made of: element counts by tag, the notation classes
 * Verovio marked them with, and the paint attributes that survived
 * sanitization.
 */
async function structure(page: Page): Promise<string> {
  return page.evaluate(() => {
    // Verovio's own tree, not the overlay beside it: the overlay is musa's
    // drawing and carries a build-dependent style-scope class, which would
    // make this golden change every time a stylesheet did.
    const root = document.querySelector(".engraving .ink > svg");
    if (!root) return "no page";
    const tags = new Map<string, number>();
    const classes = new Map<string, number>();
    const paints = new Set<string>();
    let events = 0;
    for (const element of root.querySelectorAll("*")) {
      tags.set(element.tagName, (tags.get(element.tagName) ?? 0) + 1);
      for (const name of element.classList)
        classes.set(name, (classes.get(name) ?? 0) + 1);
      for (const attribute of ["fill", "stroke", "color"]) {
        const value = element.getAttribute(attribute);
        if (value !== null) paints.add(`${attribute}=${value}`);
      }
      if (element.id.startsWith("event-")) events += 1;
    }
    const lines = (counted: Map<string, number>) =>
      [...counted]
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([name, count]) => `  ${name} ${count}`)
        .join("\n");
    return [
      `events ${events}`,
      "tags",
      lines(tags),
      "classes",
      lines(classes),
      "paint",
      [...paints]
        .sort((a, b) => a.localeCompare(b))
        .map((paint) => `  ${paint}`)
        .join("\n"),
      "",
    ].join("\n");
  });
}

/**
 * What two independent loads agree on.
 *
 * Verovio mints fresh random tokens for elements the MEI did not name — they
 * arrive as both the `id` and the `class` of a `<g>` — so a digest taken once
 * differs from itself. The golden holds only what survives being engraved
 * twice, which is exactly the part of the structure that is a fact about the
 * music rather than about this load.
 */
async function agreed(page: Page, url: string): Promise<string> {
  const digests: string[] = [];
  for (const _attempt of [0, 1]) {
    await page.goto(url);
    await engraved(page);
    digests.push(await structure(page));
  }
  const [first = "", second = ""] = digests;
  const seen = new Set(second.split("\n"));
  return first
    .split("\n")
    .filter((line) => seen.has(line))
    .join("\n");
}

for (const fixture of FIXTURES) {
  test(`${fixture} page 1 structure`, async ({ page }) => {
    await page.setViewportSize({ width: 900, height: 1000 });
    const digest = await agreed(page, `/?score=${fixture}&view=sheet`);
    // Stated separately from the snapshot, because these are the whole point
    // of the sanitizer and a snapshot refresh must not be able to quietly
    // bless their loss (§3). Every paint that survives is `currentColor`; a
    // literal one is a page that cannot be themed.
    expect(digest, "the ids the editor selects by").not.toContain("events 0");
    for (const paint of digest.split("paint\n")[1]?.split("\n") ?? []) {
      if (paint.trim() !== "") expect(paint).toContain("currentColor");
    }

    expect(digest).toMatchSnapshot(`${fixture}-structure.txt`);
  });
}
