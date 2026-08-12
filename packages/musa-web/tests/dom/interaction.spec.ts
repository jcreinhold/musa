/**
 * The provenance contract: clicks carry the compiler's
 * EventId, tie pieces are one event, unmapped elements never fire, and
 * `highlight` marks every piece of an event.
 */

import { expect, test } from "@playwright/test";

declare global {
  interface Window {
    typesetDone?: Promise<void>;
    clicks: { id: string; container: string; target: string | null }[];
    hovers: { id: string | null; target: string | null }[];
  }
}

test.beforeEach(async ({ page }) => {
  await page.goto("/tests/dom/pages/interactive.html");
  await page.evaluate(() => window.typesetDone);
});

/** The tied event's two piece ids, from the engraved SVG. */
async function tiePieces(
  page: import("@playwright/test").Page,
): Promise<[string, string]> {
  return page.locator("#score-a").evaluate((host) => {
    const ids = [
      ...(host.shadowRoot?.querySelectorAll('[id^="event-"]') ?? []),
    ].map((el) => el.getAttribute("id"));
    const second = ids.find((id) => /-t2$/.test(id ?? ""));
    const first = second?.replace(/-t2$/, "");
    if (first === undefined || second === undefined)
      throw new Error(`no tie in ${ids.join()}`);
    return [first, second] as [string, string];
  });
}

/** The compiler id an svg id stands for (strip the tie-piece suffix). */
function compilerId(svgId: string): string {
  return svgId.replace(/^event-/, "").replace(/-t\d+$/, "");
}

test("a click on a note fires the callback with the exact EventId and its container", async ({
  page,
}) => {
  const note = page.locator("#score-a .musa-event").first();
  const svgId = await note.getAttribute("id");
  // dispatchEvent, not click(): the group's bbox centre is a staff line in
  // Verovio's paint order; what is under test is delegation, not hit-testing.
  await note.dispatchEvent("click");
  const clicks = await page.evaluate(() => window.clicks);
  expect(clicks).toHaveLength(1);
  expect(clicks[0]?.id).toBe(compilerId(svgId ?? ""));
  expect(clicks[0]?.container).toBe("score-a");
  expect(clicks[0]?.target).toBe(svgId);
});

test("both pieces of a tie fire the same EventId", async ({ page }) => {
  const [first, second] = await tiePieces(page);
  await page.locator(`#score-a [id="${first}"]`).first().dispatchEvent("click");
  await page
    .locator(`#score-a [id="${second}"]`)
    .first()
    .dispatchEvent("click");
  const clicks = await page.evaluate(() => window.clicks);
  expect(clicks).toHaveLength(2);
  expect(clicks[0]?.id).toBe(compilerId(first));
  expect(clicks[1]?.id).toBe(compilerId(first));
  expect(clicks[1]?.target).toBe(second);
});

test("unmapped elements never fire", async ({ page }) => {
  // Click the staff between the notes: no event element there.
  const box = await page.locator("#score-a .musa-content > svg").boundingBox();
  if (box === null) throw new Error("no svg");
  await page.mouse.click(box.x + 2, box.y + 2);
  expect(await page.evaluate(() => window.clicks)).toHaveLength(0);
});

test("highlight marks every piece of a tied event and clears on null", async ({
  page,
}) => {
  const [first, second] = await tiePieces(page);
  await page.evaluate(async (id) => {
    const specifier = "/src/index.ts";
    const { highlight } = await import(specifier);
    highlight(id);
  }, compilerId(first));
  await expect(page.locator(`#score-a [id="${first}"]`).first()).toHaveClass(
    /musa-event-active/,
  );
  await expect(page.locator(`#score-a [id="${second}"]`).first()).toHaveClass(
    /musa-event-active/,
  );

  await page.evaluate(async () => {
    const specifier = "/src/index.ts";
    const { highlight } = await import(specifier);
    highlight(null);
  });
  await expect(page.locator("#score-a .musa-event-active")).toHaveCount(0);
});

test("two scores share a page with per-container context", async ({ page }) => {
  await page.locator("#score-b .musa-event").first().dispatchEvent("click");
  const clicks = await page.evaluate(() => window.clicks);
  expect(clicks).toHaveLength(1);
  expect(clicks[0]?.container).toBe("score-b");
});
