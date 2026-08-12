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
export type Piece =
  | "glass-mountain"
  | "large-score"
  | "annotated"
  | "open-form"
  | "open-form-again"
  /** Terms declared in bundled modules, used in a piece (prompt 124). */
  | "stdlib-basics"
  /** An expansion path that runs through a kernel quote. */
  | "kernel-splice"
  /** A claim the compiler refused: a piece that does not compile. */
  | "refused-claim";

function snapshotOf(piece: Piece): string {
  return fileURLToPath(new URL(`../../fixtures/${piece}.snapshot.json`, import.meta.url));
}

function fixtureFile(name: string): string {
  return fileURLToPath(new URL(`../../fixtures/${name}`, import.meta.url));
}

/**
 * Install the stub. Call before the page navigates.
 *
 * The piece is the workload: Glass Mountain is `06-performance.md`'s small
 * case, `large-score` its 100-bar, 4-part large case. `opens` is the piece a
 * File → Open answers with — a *second* document, which is the only way to
 * exercise what happens when one piece replaces another.
 */
export async function stubShell(
  page: Page,
  piece: Piece = "glass-mountain",
  opens: Piece = "annotated",
  /**
   * Whether the two seeded pieces are filed together as one project.
   *
   * A volume is the only state that has a contents page, a running order, or
   * a `⌘0`: with one file, the interface shows none of them
   * (`07-the-volume.md`). The stub files the two seeds and one library under
   * an album, which is exactly the shape `examples/album/` has.
   */
  volume = false,
): Promise<void> {
  const seeds = {
    // Document ids are the shell's to mint, and the committed fixtures carry
    // none (`musa_project::DocumentId::NONE`), so the stub mints them here —
    // two pieces, two documents, each counting its own revisions from zero.
    open: {
      ...(JSON.parse(readFileSync(snapshotOf(piece), "utf8")) as object),
      document: 1,
    },
    other: {
      ...(JSON.parse(readFileSync(snapshotOf(opens), "utf8")) as object),
      document: 2,
    },
    // The second reading of the open work, always loaded: it is what "read
    // it again" answers with, and the stub has no other way to produce one.
    // For a determinate piece nothing ever asks for it.
    again: {
      ...(JSON.parse(readFileSync(snapshotOf("open-form-again"), "utf8")) as object),
      document: 1,
    },
    // What the elaboration screens are answered with: one committed reading,
    // the bundled modules as documents, and the compiler's own list of what
    // it can be asked. All three are written by `musa-project`'s tests.
    report: JSON.parse(readFileSync(fixtureFile("pivot-ambiguity.analysis.json"), "utf8")) as Record<string, unknown>,
    modules: JSON.parse(readFileSync(fixtureFile("library-documents.json"), "utf8")) as Record<
      string,
      Record<string, unknown>
    >,
    kinds: JSON.parse(
      readFileSync(
        fileURLToPath(new URL("../../src/lib/session/generated/analysis-kinds.json", import.meta.url)),
        "utf8",
      ),
    ) as { kind: string; method: string }[],
    volume,
  };
  await page.addInitScript(
    (
      both: Record<"open" | "other" | "again", Record<string, unknown>> & {
        volume: boolean;
        report: Record<string, unknown>;
        modules: Record<string, Record<string, unknown>>;
        kinds: { kind: string; method: string }[];
      },
    ) => {
      const seed = both.open;
      const handlers = new Map<number, (payload: unknown) => void>();
      const events = new Map<string, number[]>();
      let next = 1;

      /**
       * The project, when the seeds are filed as one: two pieces and the
       * library they draw on, named as `examples/album/` names them.
       */
      const FILES = ["pieces/01-first.musa", "pieces/02-second.musa", "library/motifs.musa"] as const;
      const titleOf = (snapshot: Record<string, unknown>): string =>
        ((snapshot.score as { title?: string } | null)?.title ?? (snapshot.name as string)) || "";

      /** Every piece of the project that has been opened, by file name. */
      // A volume just opened has been read from disk and nothing has been typed
      // into it, so every piece starts saved — which is what makes `edited` on
      // the contents page mean something when it appears.
      const held: Record<string, Record<string, unknown>> = both.volume
        ? {
            [FILES[0]]: { ...seed, unsaved: false },
            [FILES[1]]: { ...both.other, unsaved: false },
            // Material: no score, and none coming. The interface routes on
            // `kind`, which is the whole reason it is on the wire.
            [FILES[2]]: {
              ...both.other,
              document: 3,
              unsaved: false,
              name: "motifs.musa",
              kind: "material",
              score: null,
              mei: null,
              scoreRevision: null,
              source: "library {\n    motif rise() {\n        c4/4\n    }\n}\n",
            },
          }
        : {};
      let showing: string = FILES[0];
      let current = { ...(both.volume ? (held[FILES[0]] ?? seed) : seed) };

      /**
       * The listing, as `Project::snapshot` states it: what is in hand, what is
       * edited, and which material the piece in hand imports. Null when there
       * is no project, which is what a loose file has.
       */
      function listing(): Record<string, unknown> | null {
        if (!both.volume) return null;
        const at = (file: string) => (file === showing ? current : (held[file] ?? {}));
        const entry = (file: string, used: boolean) => ({
          title: file === FILES[2] ? "motifs.musa" : titleOf(at(file)),
          file,
          current: file === showing,
          unsaved: at(file).unsaved === true,
          used,
        });
        return {
          name: "Album",
          composer: "musa",
          pieces: [entry(FILES[0], false), entry(FILES[1], false)],
          // A piece draws on the library; the library does not draw on itself.
          material: [entry(FILES[2], showing !== FILES[2])],
        };
      }

      const balanced = (source: string): boolean =>
        (source.match(/\{/g) ?? []).length === (source.match(/\}/g) ?? []).length;

      /** The 1-based line and column of the end of `source`, as Rust states it. */
      function place(source: string): { line: number; column: number } {
        const lines = source.split("\n");
        return {
          line: lines.length,
          column: (lines[lines.length - 1] ?? "").length + 1,
        };
      }

      function setSource(source: string): Record<string, unknown> {
        const compiles = balanced(source);
        const revision = (current.revision as number) + 1;
        current = {
          ...current,
          source,
          revision,
          compiles,
          // Text the composer typed is text that is not on disk yet. The core
          // marks it, and the contents page prints the mark as `edited`, so a
          // stub that skipped it would let a running order that never updates
          // pass.
          unsaved: true,
          // A valid edit produces a new score; an invalid one leaves the last
          // valid score, its MEI, and its revision untouched — roadmap §14.7,
          // and the whole point of `05-states.md` §4. The MEI the stub returns
          // is always the fixture's, because the stub is not a compiler; what
          // it reproduces faithfully is that a new score revision means the
          // engraver has a page to lay out again, which is what B2 measures.
          scoreRevision: compiles ? revision : current.scoreRevision,
          // A real diagnostic points at a place, and the interface's whole
          // answer to one is to go there — so the stub points at the end of the
          // text, where the missing brace goes, rather than at nothing. It
          // carries the rest of the shape too: the label, the
          // location the core computed, and the one certain fix, because the
          // control that applies a fix is only reachable through a diagnostic
          // that has one.
          diagnostics: compiles
            ? []
            : [
                {
                  severity: "error",
                  code: "syntax",
                  message: "missing `}`",
                  labels: [
                    {
                      span: { start: source.length, end: source.length },
                      at: place(source),
                      text: "it goes here",
                      primary: true,
                    },
                  ],
                  help: null,
                  note: null,
                  fixes: [
                    {
                      title: "add `}`",
                      edits: [
                        {
                          span: { start: source.length, end: source.length },
                          replacement: "}",
                        },
                      ],
                    },
                  ],
                  span: { start: source.length, end: source.length },
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
        origin: {
          generated: boolean;
          occurrence: string | null;
          definitionSpan: Span;
        };
      }

      const notes = (): Note[] => ((current.score as { events?: Note[] } | null)?.events ?? []) as Note[];

      /**
       * What an edit would change, by the core's own rule: every event spelled
       * by the same statement moves together. The stub can answer this honestly
       * because the snapshot carries `definitionSpan` — it is a lookup, not a
       * compilation.
       */
      /**
       * The token the edit would replace, and what it would put there.
       *
       * A compact note statement is a pitch followed by `/` and a duration.
       * The core finds those tokens by parsing; the stub finds them by shape,
       * which is enough to prove the interface marks what it is told to and
       * shows what it is given.
       */
      function writesOf(edit: Record<string, unknown>, span: Span | undefined): Record<string, unknown>[] {
        const kind = edit.kind as string;
        const value = (kind === "changePitch" ? edit.pitch : edit.duration) as string | undefined;
        if (!span || value === undefined) return [];
        const text = (current.source as string).slice(span.start, span.end);
        const written = /^(\s*)(\S+?)\/(\S+?)\s*$/.exec(text);
        if (!written) return [];
        const [, lead, pitch, duration] = written;
        const at = span.start + (lead ?? "").length;
        return kind === "changePitch"
          ? [{ start: at, end: at + (pitch ?? "").length, text: value }]
          : [
              {
                start: at + (pitch ?? "").length + 1,
                end: at + (pitch ?? "").length + 1 + (duration ?? "").length,
                text: value,
              },
            ];
      }

      function impactOf(edit: Record<string, unknown>): Record<string, unknown> {
        const target = notes().find((note) => note.id === (edit.event as string | undefined));
        const origin = target?.origin;
        const writes = writesOf(edit, origin?.definitionSpan);
        if (!origin?.generated) {
          const events = target ? [target.id] : [];
          return {
            generated: false,
            motif: null,
            occurrence: null,
            occurrences: 0,
            events,
            specializable: false,
            writes,
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
            occurrences?: {
              id: string;
              label: string;
              useSite: { start: number; end: number };
            }[];
          }
        ).occurrences;
        const label = (expansions ?? []).find((each) => each.id === origin.occurrence);
        // A `with` clause belongs to the call, so an occurrence whose call also
        // produced other occurrences — a `use` inside a `repeat` — has no "just
        // this one". Same rule as the core's, over the same facts.
        const runs = (expansions ?? []).filter((each) => each.useSite.start === label?.useSite.start).length;
        return {
          generated: true,
          motif: label?.label.split(" ▸ ").pop() ?? null,
          occurrence: label?.label ?? null,
          occurrences: occurrences.size,
          events: kin.map((note) => note.id),
          specializable: runs === 1,
          writes,
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
      window.__musaSaves = 0;
      window.__musaRevision = current.revision as number;

      /** Every answer goes out through here, so the tests can watch the revision. */
      function answer(): Record<string, unknown> {
        current = { ...current, contents: listing() };
        if (both.volume) held[showing] = current;
        window.__musaRevision = current.revision as number;
        return current;
      }

      /** The part of a score this stub has anything to say about. */
      interface Decision {
        path: string;
        pinned: boolean;
        [key: string]: unknown;
      }
      interface Score {
        performance: number | null;
        decisions: Decision[];
        [key: string]: unknown;
      }

      /** Which of the two committed readings of the open work is on screen. */
      let reading = 0;

      /**
       * Restate the reading in force: which performance it is, and which of its
       * decisions the composer kept. It mints a revision, because a different
       * reading is a different score to lay out — and because keeping a
       * decision has to be undoable like everything else.
       */
      function reperform(performance: number, keeps: (decision: Decision) => boolean): Record<string, unknown> {
        const score = current.score as Score | null;
        const revision = (current.revision as number) + 1;
        current = {
          ...current,
          revision,
          scoreRevision: revision,
          score: score && {
            ...score,
            performance,
            decisions: (score.decisions ?? []).map((decision) => ({
              ...decision,
              pinned: keeps(decision),
            })),
          },
        };
        return answer();
      }

      const commands: Record<string, (args: Record<string, unknown>) => unknown> = {
        snapshot: () => answer(),
        // Turning to another file of the project. The piece left behind keeps
        // its text, its unsaved mark and its revision, so the stub holds the
        // sessions rather than rereading a fixture — which is the promise the
        // Rust `Project` makes and the one this exercises.
        show_piece: (args) => {
          const file = args.file as string;
          held[showing] = current;
          if (file in held) showing = file;
          history.length = 0;
          current = { ...(held[showing] ?? current) };
          return answer();
        },
        save_all: () => {
          for (const file of Object.keys(held)) held[file] = { ...held[file], unsaved: false };
          current = { ...current, unsaved: false };
          return answer();
        },
        // Opening replaces the document, and the piece that arrives is at its
        // own revision 0 — lower than whatever the edited piece had reached,
        // which is exactly the case the interface has to get right.
        open_project: () => {
          history.length = 0;
          current = { ...both.other };
          return answer();
        },
        new_project: () => {
          history.length = 0;
          current = { ...both.other, name: "Untitled.musa", unsaved: true };
          return answer();
        },
        edit_impact: (args) => impactOf(args.edit as Record<string, unknown>),
        // A reading. The stub holds one real report and answers every other
        // question with the honest empty one — which is a state the panel has
        // to render, and the only one a stub can produce truthfully.
        analyze: (args) => {
          const kind = args.kind as string;
          // The reading is of the score that compiled, which is the number the
          // core would answer with and the one that makes it go stale.
          const revision = (current.scoreRevision as number | null) ?? (current.revision as number);
          if (kind === both.report.kind) return { ...both.report, revision };
          const known = both.kinds.find((offered) => offered.kind === kind);
          if (!known)
            throw {
              kind: "backend",
              message: `\`${kind}\` is not an analysis`,
            };
          return {
            revision,
            kind,
            method: known.method,
            profile: null,
            assumptions: [],
            findings: [],
          };
        },
        // A bundled module, read-only. The place comes back in the module's own
        // units; that translating it is really the core's arithmetic and not a
        // coincidence of ASCII is `musa_project::library`'s laws.
        library_document: (args) => {
          const uri = args.uri as string;
          const module = both.modules[uri];
          if (!module) throw { kind: "backend", message: `\`${uri}\` is not bundled` };
          const start = args.start as number | null;
          const end = args.end as number | null;
          return {
            ...module,
            span: start === null || start === undefined ? null : { start, end },
          };
        },
        apply: (args) => {
          const command = args.command as {
            kind: string;
            source?: string;
            edit?: Record<string, unknown>;
            performance?: number;
            decision?: string;
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
            current = {
              ...current,
              revision,
              scoreRevision: revision,
              unsaved: true,
            };
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
          // Saving is unobservable unless the stub says it happened: the core
          // clears the unsaved flag, and a test that wants to prove `:w` is the
          // project's save has to be able to see one.
          if (command.kind === "save") {
            window.__musaSaves += 1;
            current = { ...current, unsaved: false };
            return answer();
          }
          if (command.kind === "undo") {
            const previous = history.pop();
            if (previous) current = previous;
            return answer();
          }
          // Reading an open work again. The stub cannot compile, so
          // it does the one honest thing open to it: it swaps between the two
          // committed readings of the same source, which is what those two
          // fixtures are for.
          //
          // Unless a decision is kept. This piece asks exactly one question, so
          // "a decision is kept" and "the reading is settled" are the same
          // sentence here: the music stands and only the number moves. That is
          // the rule the whole control exists to make visible, and a stub that
          // ignored it would let a screen that ignores it pass.
          if (command.kind === "newPerformance") {
            history.push(current);
            const performance = Number(command.performance ?? 0);
            const decisions = (current.score as Score | null)?.decisions ?? [];
            if (!decisions.some((decision) => decision.pinned)) {
              reading = 1 - reading;
              const next = reading === 0 ? both.open : both.again;
              current = {
                ...next,
                document: current.document,
                name: current.name,
              };
            }
            return reperform(performance, (decision) => decision.pinned);
          }
          if (command.kind === "keep" || command.kind === "release") {
            history.push(current);
            const keep = command.kind === "keep";
            const which = command.decision as string;
            return reperform((current.score as Score | null)?.performance ?? 0, (decision) =>
              decision.path === which ? keep : decision.pinned,
            );
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
          // Every transport command the facade offers, answered in the shape the
          // engine guarantees. Anything less and a test would prove the
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
          current = {
            ...current,
            midiPort: args.listening === true ? "Stub Keyboard" : null,
          };
          return current;
        },
        export: () => ({ path: "/tmp/glass-mountain.mei" }),
        // The file dialogs are the platform's, not musa's, so the stub answers
        // them the way a composer who picked a file would: with a path.
        "plugin:dialog|open": () => "/tmp/second.musa",
        "plugin:dialog|save": () => "/tmp/out.mei",
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
              : Promise.reject({
                  kind: "backend",
                  message: `no such command: ${command}`,
                });
          },
        },
      });

      // A state the core would arrive at on its own — unsaved work found on
      // open, say — put into the session the way the shell puts it: as a
      // snapshot event carrying a new revision.
      Object.defineProperty(window, "__musaSet", {
        value: (patch: Record<string, unknown>) => {
          current = {
            ...current,
            ...patch,
            revision: (current.revision as number) + 1,
          };
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
    },
    seeds,
  );
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
    /** How many times the interface has asked the shell to save. */
    __musaSaves: number;
  }
}
