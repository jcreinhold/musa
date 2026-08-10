/**
 * The DOM typesetting contract (prompt 141): scanning, shadow-root
 * placement, idempotence, error boxes, per-snippet isolation, the watcher,
 * and auto-start.
 */

import { expect, test } from "@playwright/test";

/** Wait for the page's typeset() promise, then assert no console errors. */
async function settled(page: import("@playwright/test").Page, path: string) {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  await page.goto(path);
  await page.evaluate(() => (window as unknown as { typesetDone?: Promise<void> }).typesetDone);
  return errors;
}

test.describe("typeset", () => {
  test("script-tag, element, and inline snippets all engrave", async ({ page }) => {
    const errors = await settled(page, "/tests/dom/pages/basic.html");
    expect(errors).toEqual([]);

    // The script tag is untouched and gains a sibling container.
    const scriptSvg = page.locator("#snippet-script + .musa-rendered > svg");
    await expect(scriptSvg).toHaveCount(1);

    // The element renders into its shadow root…
    const shadowSvg = await page
      .locator("#snippet-element")
      .evaluate((host) => host.shadowRoot?.querySelectorAll(".musa-content > svg").length ?? 0);
    expect(shadowSvg).toBe(1);

    // …and its source stays in the light DOM.
    const source = await page.locator("#snippet-element").textContent();
    expect(source).toContain('piece "answer"');

    // The inline element engraves too.
    const inlineSvg = await page
      .locator("#snippet-inline")
      .evaluate((host) => host.shadowRoot?.querySelectorAll(".musa-content > svg").length ?? 0);
    expect(inlineSvg).toBe(1);

    // Material is an empty state, not an error.
    const material = await page
      .locator("#snippet-material")
      .evaluate((host) => ({
        empty: host.shadowRoot?.querySelector(".musa-empty")?.textContent,
        error: host.shadowRoot?.querySelector(".musa-error"),
      }));
    expect(material.empty).toBe("no score");
    expect(material.error).toBeNull();
  });

  test("processed markers make re-scanning idempotent; reprocess re-typesets", async ({ page }) => {
    await settled(page, "/tests/dom/pages/basic.html");
    await expect(page.locator("[data-musa-processed]")).toHaveCount(4);

    // A second typeset adds nothing.
    await page.evaluate(async () => {
      const specifier = "/src/index.ts";
      const { typeset } = await import(specifier);
      await typeset();
    });
    const shadowSvg = await page
      .locator("#snippet-element")
      .evaluate((host) => host.shadowRoot?.querySelectorAll(".musa-content > svg").length ?? 0);
    expect(shadowSvg).toBe(1);

    // Reprocess replaces rather than duplicates.
    await page.evaluate(async () => {
      const specifier = "/src/index.ts";
      const { typeset } = await import(specifier);
      await typeset(document, { reprocess: true });
    });
    const reprocessed = await page
      .locator("#snippet-element")
      .evaluate((host) => host.shadowRoot?.querySelectorAll(".musa-content > svg").length ?? 0);
    expect(reprocessed).toBe(1);
  });

  test("an invalid score gets an error box where the score would be", async ({ page }) => {
    const errors = await settled(page, "/tests/dom/pages/errors.html");
    expect(errors).toEqual([]);

    const box = page.locator("#broken .musa-error");
    await expect(box).toHaveCount(1);
    await expect(box.locator(".musa-error-message")).toContainText(/.+/);
    await expect(box.locator(".musa-error-code")).toContainText(/.+/);
    // The offending span is underlined in the source excerpt.
    const span = box.locator(".musa-error-span");
    await expect(span).toHaveCount(1);
    expect((await span.textContent())?.trim().length).toBeGreaterThan(0);

    // Per-snippet isolation: the good score typesets anyway.
    const goodSvg = await page
      .locator("#good")
      .evaluate((host) => host.shadowRoot?.querySelectorAll(".musa-content > svg").length ?? 0);
    expect(goodSvg).toBe(1);
  });

  test("the watcher typesets snippets appended after load", async ({ page }) => {
    const errors = await settled(page, "/tests/dom/pages/watch.html");
    expect(errors).toEqual([]);

    await page.click("#add");
    await expect
      .poll(async () =>
        page
          .locator("#appended")
          .evaluate((host) => host.shadowRoot?.querySelectorAll(".musa-content > svg").length ?? 0),
      )
      .toBe(1);
  });

  test("auto-start typesets with no explicit call", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(String(error)));
    await page.goto("/tests/dom/pages/autostart.html");
    await expect
      .poll(async () =>
        page
          .locator("#auto")
          .evaluate((host) => host.shadowRoot?.querySelectorAll(".musa-content > svg").length ?? 0),
      )
      .toBe(1);
    expect(errors).toEqual([]);
  });
});
