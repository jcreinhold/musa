import type { Page } from "@playwright/test";

/**
 * Wait until the page is worth photographing: the fonts are resident and the
 * worker has laid the score out and handed back a page. Without this the
 * goldens race the WASM cold start and capture an empty leaf.
 */
export async function engraved(page: Page): Promise<void> {
  await page.waitForFunction(() => document.fonts.status === "loaded");
  await page.waitForSelector(".engraving svg.definition-scale", { state: "attached" });
  await page.waitForFunction(() => {
    const svg = document.querySelector(".engraving svg");
    return svg !== null && svg.getBoundingClientRect().height > 1;
  });
}
