/**
 * Vim mode for the source column, and the three things it must not own
 * (prompt 55).
 *
 * The keymap lives inside the one CodeMirror instance and nowhere else:
 * nothing about the language, the score, the command model, or the document
 * changes. What does change is three commands that would otherwise be the
 * editor's private business, and are the project's:
 *
 * - **`u` and `⌃r` are the project's undo and redo.** `SourceEditor` keeps no
 *   history of its own because `⌘Z` is the project's undo over revisions, and
 *   two stacks over one document disagree about what the document is. A vim
 *   mode that quietly reintroduced a second history would be a worse bug than
 *   not having vim mode.
 * - **`:w` saves the project** — one save, one path, the same command the menu
 *   runs.
 * - **`:q` does nothing.** Closing a document from inside its text is a
 *   footgun, and the window already has a close. Defined rather than left
 *   undefined so it fails quietly instead of complaining.
 *
 * The vim module's command tables are global to the page, so registration
 * happens once and the handlers are read out of a box the live editor writes.
 * One editor is mounted at a time — Compose's or the Source workspace's — and
 * whichever it is, it is the one that answers.
 */

import type { Extension } from "@codemirror/state";
import { Vim, vim } from "@replit/codemirror-vim";

/** What the three rebound commands reach. */
export interface ProjectCommands {
  undo(): void;
  redo(): void;
  save(): void;
}

let project: ProjectCommands | null = null;
let registered = false;

function register(): void {
  if (registered) return;
  registered = true;
  Vim.defineAction("musaUndo", () => project?.undo());
  Vim.defineAction("musaRedo", () => project?.redo());
  Vim.mapCommand("u", "action", "musaUndo", {}, { context: "normal" });
  Vim.mapCommand("<C-r>", "action", "musaRedo", {}, { context: "normal" });
  Vim.defineEx("write", "w", () => project?.save());
  Vim.defineEx("quit", "q", () => {});
}

/** Point the rebound commands at the live session. */
export function serve(commands: ProjectCommands): void {
  register();
  project = commands;
}

/**
 * The extension, with the package's own status line.
 *
 * A modal editor that does not say which mode it is in is the one thing worse
 * than a modeless one, and `:` needs somewhere to be typed. It is the
 * package's default surface, restyled to the tokens like everything else — no
 * plugin surface beyond it.
 */
export function modal(): Extension {
  register();
  return vim({ status: true });
}
