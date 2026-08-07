/**
 * A stubbed shell, for driving the interface in a headless browser.
 *
 * `06-performance.md` §2 sanctions this: the harness runs against the Vite
 * dev server with a stubbed IPC layer where a window is impractical, and it is
 * impractical here — `tauri-driver` has no macOS support, so a real webview
 * cannot be automated on the machine this is developed on.
 *
 * The stub is not a compiler and does not pretend to be one: it answers with
 * the *shape* the core guarantees — a new revision, the same score, and
 * diagnostics when the braces do not balance. That every one of those
 * guarantees is true of the real core is what the Rust tests assert.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { Page } from "@playwright/test";

/** The pieces the stub can open, both written by `musa-project`'s tests. */
export type Piece = "glass-mountain" | "large-score";

function snapshotOf(piece: Piece): string {
  return fileURLToPath(new URL(`../../fixtures/${piece}.snapshot.json`, import.meta.url));
}

/**
 * Install the stub. Call before the page navigates.
 *
 * The piece is the workload: Glass Mountain is `06-performance.md`'s small
 * case, `large-score` its 100-bar, 4-part large case.
 */
export async function stubShell(page: Page, piece: Piece = "glass-mountain"): Promise<void> {
  const valid: unknown = JSON.parse(readFileSync(snapshotOf(piece), "utf8"));
  await page.addInitScript((seed: Record<string, unknown>) => {
    const handlers = new Map<number, (payload: unknown) => void>();
    const events = new Map<string, number[]>();
    let next = 1;
    let current = { ...seed };

    const balanced = (source: string): boolean =>
      (source.match(/\{/g) ?? []).length === (source.match(/\}/g) ?? []).length;

    function setSource(source: string): Record<string, unknown> {
      const compiles = balanced(source);
      const revision = (current.revision as number) + 1;
      current = {
        ...current,
        source,
        revision,
        compiles,
        // A valid edit produces a new score; an invalid one leaves the last
        // valid score, its MEI, and its revision untouched — roadmap §14.7,
        // and the whole point of `05-states.md` §4. The MEI the stub returns
        // is always the fixture's, because the stub is not a compiler; what
        // it reproduces faithfully is that a new score revision means the
        // engraver has a page to lay out again, which is what B2 measures.
        scoreRevision: compiles ? revision : current.scoreRevision,
        diagnostics: compiles ? [] : [{ severity: "error", message: "expected `}`", span: null }],
      };
      return current;
    }

    const commands: Record<string, (args: Record<string, unknown>) => unknown> = {
      snapshot: () => current,
      open_project: () => current,
      new_project: () => current,
      apply: (args) => {
        const command = args.command as { kind: string; source?: string };
        return command.kind === "setSource" ? setSource(command.source ?? "") : current;
      },
      transport: (args) => {
        const command = args.command as { kind: string };
        const playback = current.playback as Record<string, unknown>;
        current = { ...current, playback: { ...playback, playing: command.kind === "play" } };
        return current;
      },
      export: () => ({ path: "/tmp/glass-mountain.mei" }),
    };

    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {
        transformCallback(callback: (payload: unknown) => void) {
          const id = next++;
          handlers.set(id, callback);
          return id;
        },
        invoke(command: string, args: Record<string, unknown> = {}) {
          if (command === "plugin:event|listen") {
            const name = args.event as string;
            const id = args.handler as number;
            events.set(name, [...(events.get(name) ?? []), id]);
            return Promise.resolve(id);
          }
          if (command === "plugin:event|unlisten") return Promise.resolve(null);
          const answer = commands[command];
          return answer
            ? Promise.resolve(answer(args))
            : Promise.reject({ kind: "backend", message: `no such command: ${command}` });
        },
      },
    });

    // The tests raise shell events — a menu selection, a position tick —
    // through the same path the shell does.
    Object.defineProperty(window, "__musaEmit", {
      value: (name: string, payload: unknown) => {
        for (const id of events.get(name) ?? []) {
          handlers.get(id)?.({ event: name, id, payload });
        }
      },
    });
  }, valid as Record<string, unknown>);
}

declare global {
  interface Window {
    __musaEmit: (name: string, payload: unknown) => void;
  }
}
