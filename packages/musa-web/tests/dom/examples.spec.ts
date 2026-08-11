/**
 * The example smokes (prompt 150): every committed example page typesets
 * without console errors, the CDN page does it with no module scripts, and
 * the build-time page needs no JavaScript at all. The examples cannot rot
 * silently.
 */

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { expect, test } from "@playwright/test";

function pageErrors(page: import("@playwright/test").Page): string[] {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  return errors;
}

const MODULE_EXAMPLES = ["blog", "docs", "interactive"] as const;

for (const name of MODULE_EXAMPLES) {
  test(`examples/${name}.html typesets every score`, async ({ page }) => {
    const errors = pageErrors(page);
    await page.goto(`/examples/${name}.html`);
    if (name !== "docs") await page.evaluate(() => window.typesetDone);
    // docs.html auto-starts; the others resolve typesetDone.
    await expect(page.locator("musa-score svg").first()).toBeVisible({ timeout: 30_000 });
    const scores = await page.locator("musa-score").count();
    const svgs = await page.locator("musa-score svg").count();
    // Every score except a deliberately empty one engraved something.
    expect(svgs).toBeGreaterThanOrEqual(scores - 1);
    expect(errors).toEqual([]);
  });
}

test("examples/dynamic.html typesets an appended snippet", async ({ page }) => {
  const errors = pageErrors(page);
  await page.goto("/examples/dynamic.html");
  await page.evaluate(() => window.typesetDone);
  await page.click("#add");
  await expect(page.locator("musa-score svg").first()).toBeVisible({ timeout: 30_000 });
  expect(errors).toEqual([]);
});

test("examples/interactive.html reports an event id on click", async ({ page }) => {
  const errors = pageErrors(page);
  await page.goto("/examples/interactive.html");
  await page.evaluate(() => window.typesetDone);
  await page.locator("#tied .musa-event").first().dispatchEvent("click");
  await expect(page.locator("#report")).toContainText(/^event [0-9a-f]+$/);
  expect(errors).toEqual([]);
});

test("examples/docs.html shows an error box for the broken snippet", async ({ page }) => {
  const errors = pageErrors(page);
  await page.goto("/examples/docs.html");
  await expect(page.locator(".musa-error").first()).toBeVisible({ timeout: 30_000 });
  expect(errors).toEqual([]);
});

test("examples/cdn.html typesets with no module scripts on the page", async ({ page }) => {
  const errors = pageErrors(page);
  await page.goto("/examples/cdn.html");
  await expect(page.locator("musa-score svg").first()).toBeVisible({ timeout: 60_000 });
  // The dev server injects its own /@vite/client module; the page itself
  // authors none.
  expect(await page.locator('script[type="module"]:not([src*="@vite"])').count()).toBe(0);
  expect(errors).toEqual([]);
});

test.describe("the build-time recipe", () => {
  test.beforeAll(() => {
    execFileSync("node", ["typeset.mjs"], {
      cwd: fileURLToPath(new URL("../../examples/build-time/", import.meta.url)),
    });
  });

  test("emits SVG and needs no JavaScript", async ({ browser }) => {
    const out = readFileSync(
      fileURLToPath(new URL("../../examples/build-time/out/index.html", import.meta.url)),
      "utf8",
    );
    expect(out).toContain("<svg");
    expect(out).not.toContain('type="text/musa"');

    const context = await browser.newContext({ javaScriptEnabled: false });
    const page = await context.newPage();
    await page.goto("/examples/build-time/out/index.html");
    await expect(page.locator(".musa-rendered svg").first()).toBeVisible();
    await context.close();
  });
});

declare global {
  interface Window {
    typesetDone?: Promise<void>;
  }
}
