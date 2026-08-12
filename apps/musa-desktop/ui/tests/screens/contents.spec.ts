/**
 * The volume (`docs/interface/07-the-volume.md`).
 *
 * The claim under test is that a project is a bound book rather than a file
 * tree: it has a contents page with a running order, an editorial note listing
 * the material the pieces rest on, and — for a project of one — nothing at
 * all. Every string on that page was decided in Rust (`03-interaction.md` §7),
 * so what is asserted here is arrangement, routing, and restraint.
 */

import { expect, test, type Locator, type Page } from "@playwright/test";

import { engraved } from "./engraved";
import { stubShell } from "./shell";
import { rewrite, text, toggleSource } from "./source";

/** The running order, on the contents page. */
function order(page: Page): Locator {
  return page
    .getByRole("navigation", { name: "Running order" })
    .getByRole("button");
}

/** The running order, in the left margin of a piece. */
function inMargin(page: Page): Locator {
  return page.getByRole("navigation", { name: "Contents" }).getByRole("button");
}

/** The material, at the foot of the contents page. */
function material(page: Page): Locator {
  return page.getByRole("region", { name: "Material" }).getByRole("button");
}

/** Turn to the volume's front matter, the way `⌘0` does. */
async function contents(page: Page): Promise<void> {
  await page.keyboard.press("ControlOrMeta+0");
  await expect(order(page).first()).toBeVisible();
}

/** The album: two pieces and the library they draw on, as `examples/album/`. */
async function album(page: Page): Promise<void> {
  await stubShell(page, "glass-mountain", "annotated", true);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/");
  await engraved(page);
}

/**
 * A loose file is the state this whole design spends its budget on being
 * invisible in: no page, no margin section, no entry in the switcher, and no
 * `⌘0` that leads to a page with one line on it.
 */
test("a project of one shows no contents anywhere", async ({ page }) => {
  await stubShell(page);
  await page.goto("/");
  await engraved(page);

  const workspaces = page
    .getByRole("navigation", { name: "Workspace" })
    .getByRole("button");
  await expect(workspaces).toHaveCount(4);
  await expect(workspaces.filter({ hasText: "Contents" })).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: "Contents" })).toHaveCount(
    0,
  );

  // And the binding leads nowhere: the page stays on the leaf.
  await page.keyboard.press("ControlOrMeta+0");
  await expect(page.locator(".engraving svg.definition-scale")).toBeVisible();
  await expect(
    page.getByRole("navigation", { name: "Running order" }),
  ).toHaveCount(0);
});

test("the contents page prints the running order the manifest sets", async ({
  page,
}) => {
  await album(page);
  await contents(page);

  // The volume names itself, and its composer beneath.
  await expect(page.locator(".page .volume")).toHaveText("Album");
  await expect(page.locator(".page .composer")).toHaveText("musa");

  // Two pieces, in the manifest's order, each named as it names itself and
  // numbered as the volume counts.
  await expect(order(page)).toHaveCount(2);
  await expect(order(page).nth(0).locator(".title")).toHaveText(
    "Glass Mountain",
  );
  await expect(order(page).nth(0).locator(".file")).toHaveText(
    "pieces/01-first.musa",
  );
  await expect(order(page).nth(0).locator(".position")).toHaveText("01");
  await expect(order(page).nth(1).locator(".title")).toHaveText("Annotated");
  await expect(order(page).nth(1).locator(".file")).toHaveText(
    "pieces/02-second.musa",
  );
  await expect(order(page).nth(1).locator(".position")).toHaveText("02");

  // The piece in hand is the first, and it says so in ink and in a rule —
  // never colour alone (`03-interaction.md` §5).
  await expect(order(page).nth(0)).toHaveAttribute("aria-current", "page");
  await expect(order(page).nth(1)).not.toHaveAttribute("aria-current", "page");
  const underlined = await order(page)
    .nth(0)
    .locator(".title")
    .evaluate((node) => getComputedStyle(node).textDecorationLine);
  expect(underlined).toBe("underline");

  // The one typographic rule the row is built on: what the composer wrote is
  // set in the score's own text face, what the filesystem knows is set in
  // mono. Two kinds of knowledge, two faces, one line.
  const faces = await order(page)
    .nth(0)
    .evaluate((row) => ({
      title: getComputedStyle(row.querySelector(".title") as Element)
        .fontFamily,
      file: getComputedStyle(row.querySelector(".file") as Element).fontFamily,
      score: getComputedStyle(document.documentElement).getPropertyValue(
        "--f-score-text",
      ),
      mono: getComputedStyle(document.documentElement).getPropertyValue(
        "--f-mono",
      ),
    }));
  expect(named(faces.title)).toBe(named(faces.score));
  expect(named(faces.file)).toBe(named(faces.mono));
  expect(named(faces.title)).not.toBe(named(faces.file));
});

test("the margin lists the pieces, and the piece in hand is marked", async ({
  page,
}) => {
  await album(page);

  // The margin reads outside in — volume, then parts — and it lists pieces
  // only: material is reached from the contents page, deliberately.
  await expect(inMargin(page)).toHaveCount(2);
  await expect(inMargin(page).nth(0)).toHaveAttribute("aria-current", "page");
  await expect(inMargin(page).nth(0).locator(".name")).toHaveText(
    "Glass Mountain",
  );
  await expect(inMargin(page).nth(1).locator(".name")).toHaveText("Annotated");
});

test("choosing a piece draws it, and the selection is let go", async ({
  page,
}) => {
  await album(page);

  // Take hold of a note in the piece on screen, so there is a selection to
  // lose.
  await page
    .locator('.engraving [id="event-d"]')
    .first()
    .click({ force: true });
  await expect(page.locator(".overlay .selection")).toHaveCount(1);

  await contents(page);
  await order(page).nth(1).click();

  // The other piece is on the leaf, named by its own title.
  await expect(page.locator("h1.title")).toHaveText("Annotated");
  await expect(page.locator(".engraving svg.definition-scale")).toBeVisible();

  // And nothing is chosen: the selection named an event in the piece that was
  // on screen, and in this one it names nothing (`05-states.md` §9).
  await expect(page.locator(".overlay .selection")).toHaveCount(0);
  const chosen = await page.evaluate(
    () =>
      document
        .querySelector('[role="application"]')
        ?.getAttribute("aria-activedescendant") ?? "",
  );
  expect(chosen).toBe("");

  // The margin follows: the piece in hand is the one that was chosen.
  await expect(inMargin(page).nth(1)).toHaveAttribute("aria-current", "page");
});

test("a piece keeps its text across a turn away, and its row says edited", async ({
  page,
}) => {
  await album(page);
  await toggleSource(page);
  await rewrite(page, 'piece "Glass Mountain" {\n    c4 1;\n}');
  // The margin says so first, and waiting on it is also waiting for the edit
  // to have reached the core rather than the editor.
  await expect(inMargin(page).nth(0)).toContainText("edited");

  // Away, and back by the margin — the shortest route, and the one a composer
  // moving between two movements actually takes.
  await inMargin(page).nth(1).click();
  await expect(page.locator("h1.title")).toHaveText("Annotated");

  await contents(page);
  await expect(order(page).nth(0)).toContainText("edited");
  await expect(order(page).nth(1)).not.toContainText("edited");

  await order(page).nth(0).click();
  await expect
    .poll(() => text(page))
    .toBe('piece "Glass Mountain" {\n    c4 1;\n}');
});

test("a library opens in the text, and is not a blank window", async ({
  page,
}) => {
  await album(page);
  await contents(page);

  // Material is listed as apparatus, at the foot, unnumbered — it has no
  // position because it has no place in the running order.
  await expect(material(page)).toHaveCount(1);
  await expect(material(page).locator(".title")).toHaveText("motifs.musa");
  await expect(material(page).locator(".position")).toHaveText("");

  await material(page).click();

  // The Source workspace, because a library has no page. That is a fact about
  // the file, so it overrides the workspace that was asked for.
  await expect(page.locator(".source-workspace")).toBeVisible();
  await expect.poll(() => text(page)).toContain("motif rise()");
});

test("in use marks exactly the material the piece in hand imports", async ({
  page,
}) => {
  await album(page);
  await contents(page);
  await expect(material(page)).toContainText("in use");

  // With the library itself in hand nothing is in use by it: a library does
  // not draw on itself, and `in use` is the current piece's import set rather
  // than a property of the file.
  await material(page).click();
  await expect(page.locator(".source-workspace")).toBeVisible();

  await contents(page);
  await expect(material(page)).not.toContainText("in use");
  await expect(order(page).nth(0)).not.toHaveAttribute("aria-current", "page");
});

/**
 * A font stack, as both a custom property and a computed style spell it.
 *
 * `getComputedStyle` quotes any family whose name it thinks needs quoting, and
 * the token does not, so the two strings differ by punctuation while naming
 * the same faces. The comparison is about which face, not about quoting.
 */
function named(stack: string): string {
  return stack
    .replace(/["']/g, "")
    .replace(/\s*,\s*/g, ",")
    .trim();
}
