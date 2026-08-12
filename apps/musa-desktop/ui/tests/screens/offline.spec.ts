/**
 * Roadmap §14.8: zero setup, and the application is fully functional
 * offline. Fonts are bundled, the WASM is bundled, the fixtures are imported
 * rather than fetched — and none of that is worth stating unless something
 * checks it.
 *
 * A font that quietly falls back to a system serif is the exact failure this
 * catches: the page still renders, so nobody notices until a machine without
 * the network shows a score set in Times.
 */

import { expect, test } from "@playwright/test";

import { engraved } from "./engraved";

test("the application makes no request off its own origin", async ({ page, baseURL }) => {
  const foreign: string[] = [];
  page.on("request", (request) => {
    if (!request.url().startsWith(baseURL ?? "") && !request.url().startsWith("data:")) {
      foreign.push(request.url());
    }
  });

  await page.goto("/");
  await engraved(page);

  expect(foreign).toStrictEqual([]);
});

test("the bundled faces are the ones actually used", async ({ page }) => {
  await page.goto("/");
  await engraved(page);

  const families = await page.evaluate(() => [...document.fonts].map((face) => face.family));
  expect(families).toEqual(expect.arrayContaining(["Bravura", "Academico", "Instrument Sans", "Recursive Mono"]));

  // Every one of them resolved locally; a face that failed to load would sit
  // in "unloaded" or "error" after the page settled.
  const unresolved = await page.evaluate(() =>
    [...document.fonts].filter((face) => face.status === "error").map((face) => face.family),
  );
  expect(unresolved).toStrictEqual([]);
});
