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
        // A real diagnostic points at a place, and the interface's whole
        // answer to one is to go there — so the stub points at the brace it
        // is complaining about rather than at nothing.
        diagnostics: compiles
          ? []
          : [
              {
                severity: "error",
                message: "expected `}`",
                span: { start: source.lastIndexOf("{"), end: source.lastIndexOf("{") + 1 },
              },
            ],
      };
      return current;
    }

    interface Span {
      start: number;
      end: number;
    }
    interface Note {
      id: string;
      origin: { generated: boolean; occurrence: string | null; definitionSpan: Span };
    }

    const notes = (): Note[] =>
      ((current.score as { events?: Note[] } | null)?.events ?? []) as Note[];

    /**
     * What an edit would change, by the core's own rule: every event spelled
     * by the same statement moves together. The stub can answer this honestly
     * because the snapshot carries `definitionSpan` — it is a lookup, not a
     * compilation.
     */
    function impactOf(edit: Record<string, unknown>): Record<string, unknown> {
      const target = notes().find((note) => note.id === (edit.event as string | undefined));
      const origin = target?.origin;
      if (!origin?.generated) {
        const events = target ? [target.id] : [];
        return {
          generated: false,
          motif: null,
          occurrence: null,
          occurrences: 0,
          events,
          specializable: false,
        };
      }
      const kin = notes().filter(
        (note) =>
          note.origin.definitionSpan.start === origin.definitionSpan.start &&
          note.origin.definitionSpan.end === origin.definitionSpan.end,
      );
      const occurrences = new Set(kin.map((note) => note.origin.occurrence));
      const expansions = (
        current.score as {
          occurrences?: { id: string; label: string; useSite: { start: number; end: number } }[];
        }
      ).occurrences;
      const label = (expansions ?? []).find((each) => each.id === origin.occurrence);
      // A `with` clause belongs to the call, so an occurrence whose call also
      // produced other occurrences — a `use` inside a `repeat` — has no "just
      // this one". Same rule as the core's, over the same facts.
      const runs = (expansions ?? []).filter((each) => each.useSite.start === label?.useSite.start)
        .length;
      return {
        generated: true,
        motif: label?.label.split(" ▸ ").pop() ?? null,
        occurrence: label?.label ?? null,
        occurrences: occurrences.size,
        events: kin.map((note) => note.id),
        specializable: runs === 1,
      };
    }

    /**
     * The states the document has been in, so undo can put one back. A score
     * edit here changes the revision and records what was asked for; that the
     * source it would produce is the right source is asserted by the Rust
     * editing laws, not here.
     */
    const history: Record<string, unknown>[] = [];
    window.__musaEdits = [];
    window.__musaRevision = current.revision as number;

    /** Every answer goes out through here, so the tests can watch the revision. */
    function answer(): Record<string, unknown> {
      window.__musaRevision = current.revision as number;
      return current;
    }

    const commands: Record<string, (args: Record<string, unknown>) => unknown> = {
      snapshot: () => current,
      open_project: () => current,
      new_project: () => current,
      edit_impact: (args) => impactOf(args.edit as Record<string, unknown>),
      apply: (args) => {
        const command = args.command as {
          kind: string;
          source?: string;
          edit?: Record<string, unknown>;
        };
        if (command.kind === "setSource") {
          history.push(current);
          setSource(command.source ?? "");
          return answer();
        }
        // A studio edit is the same kind of act as a score edit — intent in,
        // a rewritten source out — and the stub answers it the same way: it
        // records what was asked for and mints a revision. That the text it
        // would produce is the right text is `musa-project`'s studio laws.
        if (command.kind === "editScore" || command.kind === "editStudio") {
          history.push(current);
          window.__musaEdits.push(command.edit ?? {});
          const revision = (current.revision as number) + 1;
          current = { ...current, revision, scoreRevision: revision, unsaved: true };
          return answer();
        }
        // Formatting is the one command that rewrites the source without the
        // composer typing it. The stub indents by brace depth, which is the
        // rule `musa-language`'s formatter follows; that its output is
        // byte-for-byte the formatter's is a Rust test's business, not this
        // one's. What the interface is judged on here is that a source it did
        // not type still keeps the caret where it was.
        if (command.kind === "format") {
          history.push(current);
          let depth = 0;
          setSource(
            (current.source as string)
              .split("\n")
              .map((line) => {
                const trimmed = line.trim();
                const opens = (trimmed.match(/\{/g) ?? []).length;
                const closes = (trimmed.match(/\}/g) ?? []).length;
                const at = trimmed.startsWith("}") ? Math.max(depth - 1, 0) : depth;
                depth = Math.max(at + opens - closes + (trimmed.startsWith("}") ? 1 : 0), 0);
                return trimmed === "" ? "" : "    ".repeat(at) + trimmed;
              })
              .join("\n"),
          );
          return answer();
        }
        // Recovery is offered by the core and answered by the composer:
        // taking it makes the copy the source, declining it forgets it. The
        // stub answers both the way the session does.
        if (command.kind === "restoreRecovery") {
          history.push(current);
          const recovered = current.recovery as string | null;
          if (recovered !== null) setSource(recovered);
          current = { ...current, recovery: null };
          return answer();
        }
        if (command.kind === "discardRecovery") {
          current = { ...current, recovery: null };
          return answer();
        }
        if (command.kind === "undo") {
          const previous = history.pop();
          if (previous) current = previous;
          return answer();
        }
        return answer();
      },
      transport: (args) => {
        const command = args.command as {
          kind: string;
          frame?: number;
          start?: number;
          end?: number;
        };
        const playback = { ...(current.playback as Record<string, unknown>) };
        // Every transport command the facade offers, answered in the shape
        // prompt 18 guarantees. Anything less and a test would prove the
        // interface works against a stub that ignores what it asked for.
        if (command.kind === "play") playback.playing = true;
        if (command.kind === "pause" || command.kind === "stop") playback.playing = false;
        if (command.kind === "stop") playback.positionFrames = 0;
        if (command.kind === "seek") playback.positionFrames = command.frame ?? 0;
        if (command.kind === "setLoop") playback.loopRegion = [command.start ?? 0, command.end ?? 0];
        if (command.kind === "clearLoop") playback.loopRegion = null;
        current = { ...current, playback };
        // The tests read this to prove the interface asked for the loop it
        // drew, rather than drawing one it never sent.
        window.__musaLoop = (playback.loopRegion as [number, number] | null) ?? null;
        return current;
      },
      // A machine with a keyboard plugged in; the no-device case is
      // `musa-engine`'s test, since it is about the host and not about this.
      listen_to_midi: (args) => {
        current = { ...current, midiPort: args.listening === true ? "Stub Keyboard" : null };
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

    // A state the core would arrive at on its own — unsaved work found on
    // open, say — put into the session the way the shell puts it: as a
    // snapshot event carrying a new revision.
    Object.defineProperty(window, "__musaSet", {
      value: (patch: Record<string, unknown>) => {
        current = { ...current, ...patch, revision: (current.revision as number) + 1 };
        window.__musaEmit("musa://snapshot", current);
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
    /** Put the session into a state the core would have produced. */
    __musaSet: (patch: Record<string, unknown>) => void;
    /** The loop region the interface last asked the shell for. */
    __musaLoop: [number, number] | null;
    /** Every score edit the interface has asked for, in order. */
    __musaEdits: Record<string, unknown>[];
    /** The revision the stub last answered with, so undo can be seen to land. */
    __musaRevision: number;
  }
}
