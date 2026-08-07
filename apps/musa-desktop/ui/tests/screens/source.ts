import { expect, type Locator, type Page } from "@playwright/test";

/**
 * Driving the source editor.
 *
 * The source is a CodeMirror document, not a text box: it has no `value`, it
 * renders only the lines that are on screen, and it takes text through the
 * same input events a keyboard produces. These helpers are the whole of what
 * the tests need to know about that.
 */

/**
 * The drawer's handle, which is not the workspace switcher's Source button:
 * one shows the text under the page, the other opens the workspace.
 */
export function drawer(page: Page): Locator {
  return page.locator(".drawer button.handle");
}

/** The editable surface, by the name it announces itself with. */
export function source(page: Page): Locator {
  return page.getByRole("textbox", { name: "Source" });
}

/**
 * What the document says now.
 *
 * Read off the rendered lines: CodeMirror renders one element per line, and
 * these documents are short enough that all of them are on screen.
 */
export function text(page: Page): Promise<string> {
  return source(page).evaluate((node) =>
    [...node.querySelectorAll(".cm-line")].map((line) => line.textContent ?? "").join("\n"),
  );
}

/** Replace the document, as a select-all and a retype would. */
export async function rewrite(page: Page, next: string): Promise<void> {
  const field = source(page);
  await field.click();
  await field.press("ControlOrMeta+a");
  await page.keyboard.insertText(next);
  await expect.poll(() => text(page)).toBe(next);
}

/**
 * Where the caret is, as an offset into the document.
 *
 * There is no `selectionStart` on a contenteditable, so this counts the text
 * before the selection's start the way the document does: line by line, with
 * one character for each line break.
 */
export function caret(page: Page): Promise<number> {
  return source(page).evaluate((node) => {
    const selection = node.ownerDocument.getSelection();
    if (!selection || selection.rangeCount === 0) return -1;
    const range = selection.getRangeAt(0);
    let offset = 0;
    for (const line of node.querySelectorAll(".cm-line")) {
      if (line === range.startContainer || line.contains(range.startContainer)) {
        const before = node.ownerDocument.createRange();
        before.setStart(line, 0);
        before.setEnd(range.startContainer, range.startOffset);
        return offset + before.toString().length;
      }
      offset += (line.textContent ?? "").length + 1;
    }
    return -1;
  });
}

/** What the editor has selected, as text. */
export function selected(page: Page): Promise<string> {
  return source(page).evaluate((node) => node.ownerDocument.getSelection()?.toString() ?? "");
}

/**
 * The provenance marks, in document order.
 *
 * A mark that spans lines is drawn as one element per line, so what a test
 * can ask about is the marked text, not the number of elements.
 */
export function marked(page: Page): Promise<string[]> {
  return source(page).evaluate((node) =>
    [...node.querySelectorAll(".cm-musa-origin")].map((mark) => mark.textContent ?? ""),
  );
}
