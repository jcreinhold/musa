/**
 * A stubbed shell, for driving the interface in a headless browser.
 *
 * `06-frame-budgets.md` §2 sanctions this: the harness runs against the Vite
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
  /** An expansion path that runs through an event track quote. */
  | "events-splice"
  /** Instruments, assets, buses, clips, and fixed cues for Sound/Mix. */
  | "sound-workbench"
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
 * The piece is the workload: Glass Mountain is `06-frame-budgets.md`'s small
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
                  causes: [],
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
        kind: string;
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
      window.__musaGroupEdits = [];
      window.__musaSaves = 0;
      window.__musaReviewActs = [];
      window.__musaRevision = current.revision as number;

      /** Restate the capture facts, leaving everything else where it was. */
      function capture(patch: Record<string, unknown>): Record<string, unknown> {
        current = {
          ...current,
          midiCapture: { ...(current.midiCapture as Record<string, unknown>), ...patch },
        };
        return answer();
      }

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

      /** Plan identities, minted per preview and never reused. */
      let plans = 0;

      /**
       * The reading under review, and the states it has been in.
       *
       * The stub is not a transcriber. What it answers is the *shape* the
       * project guarantees: one immutable proposal, marks only where the
       * retained readings disagree, one decision settles one mark, taking a
       * decision back restores exactly the reading before it, and keeping it
       * seals the review without touching the source. That the notation it
       * would compose is the right notation is `musa-project`'s review laws.
       */
      let review: Record<string, unknown> | null = null;
      const readings: Record<string, unknown>[] = [];

      /**
       * A refusal the project would give and the stub cannot reach on its
       * own — that the phrase would not compile where it is going, most of
       * all. Set for one call and consumed by it.
       */
      let refusePlacement: string | null = null;

      /** The lines a placement would write, under the names it was given. */
      function planned(names: string[]): Record<string, unknown> {
        if (!review) throw { kind: "backend", message: "there is no take under review" };
        if (review.sealed !== true) throw { kind: "document", message: "this reading has not been accepted yet" };
        const held = (review.voice as string | null) ?? "voice";
        const count = review.voiceCount as number;
        const offered = Array.from({ length: count }, (_, at) => (at === 0 ? held : `captured${at > 1 ? at : ""}`));
        const chosen = names.length > 0 ? names : offered;
        if (chosen.length !== count) {
          throw {
            kind: "document",
            message: `this phrase is written in ${count} lines and ${names.length} were named; name each line before keeping it`,
          };
        }
        for (const [at, name] of chosen.entries()) {
          if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
            throw { kind: "document", message: `\`${name}\` is not a name a voice can have` };
          }
          if (chosen.slice(0, at).includes(name)) {
            throw { kind: "document", message: `two lines of this phrase were both pointed at \`${name}\`` };
          }
        }
        const voices = chosen.map((name, at) => ({
          proposalVoice: at,
          name,
          added: name !== held,
          bars: 1,
        }));
        const added = voices.filter((line) => line.added).length;
        const notes = (review.notes as unknown[]).length;
        const where = voices.map((line) => line.name).join(" and ");
        const bars = voices.length === 1 ? "1 bar" : `${voices.length} bars`;
        return {
          takeName: review.takeName,
          capturedAt: review.revision,
          revision: current.revision,
          policy: review.policy,
          part: review.part,
          voices,
          source: `${current.source as string}\n`,
          summary:
            `${notes === 1 ? "1 note" : `${notes} notes`} in ${bars} into ${review.part}\u2019s ${where}` +
            (added === 0 ? "" : added === 1 ? ", adding one line" : `, adding ${added} lines`),
        };
      }

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
        // One musical command against a selection, previewed. The stub is not
        // a compiler: what it answers is the *shape* the core guarantees —
        // the same statement moves its events together, a pitch command
        // leaves rests alone, and the plan is named by an identity that
        // accepting it consumes. That the source it would write is the right
        // source is `musa-project`'s group-edit laws.
        group_edit_plan: (args) => {
          const edit = args.edit as {
            events: string[];
            intent: { kind: string; steps?: number; duration?: string; ratio?: string; interval?: string };
            mode: string;
          };
          const rhythmic = edit.intent.kind === "setEachDuration" || edit.intent.kind === "scaleDurations";
          const wanted = new Set(edit.events);
          const chosen = notes().filter((note) => wanted.has(note.id));
          const applies = chosen.filter((note) => rhythmic || note.kind !== "rest");
          if (applies.length === 0) {
            throw {
              kind: "uneditable",
              message: "nothing in this selection has a pitch to move",
            };
          }
          const spans = new Map<string, Note[]>();
          for (const note of applies) {
            const key = `${note.origin.definitionSpan.start}:${note.origin.definitionSpan.end}`;
            spans.set(key, [
              ...(spans.get(key) ?? []),
              ...notes().filter(
                (kin) =>
                  kin.origin.definitionSpan.start === note.origin.definitionSpan.start &&
                  kin.origin.definitionSpan.end === note.origin.definitionSpan.end,
              ),
            ]);
          }
          const changed = [...new Set([...spans.values()].flat().map((note) => note.id))];
          const summary =
            edit.intent.kind === "setEachDuration"
              ? `Write ${edit.intent.duration} on ${applies.length} notes`
              : edit.intent.kind === "scaleDurations"
                ? `Scale ${applies.length} durations by ${edit.intent.ratio}`
                : edit.intent.kind === "transposeBy"
                  ? `Transpose ${applies.length} notes by ${edit.intent.interval}`
                  : `Move ${applies.length} notes ${edit.intent.kind === "moveDiatonically" ? "a step" : "a semitone"}`;
          plans += 1;
          return {
            plan: plans,
            revision: current.revision as number,
            summary,
            changed,
            unchanged: chosen.filter((note) => !applies.includes(note)).map((note) => note.id),
            definitions: [...spans.entries()].map(([key, kin]) => {
              const [start, end] = key.split(":");
              const first = kin[0];
              return {
                definition: { start: Number(start), end: Number(end) },
                motif: first?.origin.generated ? "sigh" : null,
                occurrences: [...new Set(kin.map((note) => note.origin.occurrence).filter(Boolean))],
                selected: kin.filter((note) => wanted.has(note.id)).map((note) => note.id),
                events: kin.map((note) => note.id),
                before: "c5",
                after: "d5",
              };
            }),
            bars: [],
            warnings: [],
            specializable: applies.every((note) => note.origin.generated),
          };
        },
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
            plan?: number;
            revision?: number;
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
          // rule `musa-syntax`'s formatter follows; that its output is
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
          // Accepting a previewed transformation: one transaction, one entry
          // in the history, and the plan is spent.
          if (command.kind === "applyGroupEdit") {
            history.push(current);
            window.__musaGroupEdits.push({ plan: command.plan, revision: command.revision });
            const revision = (current.revision as number) + 1;
            current = {
              ...current,
              revision,
              scoreRevision: revision,
              unsaved: true,
            };
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
        // `musa-playback`'s test, since it is about the host and not about this.
        listen_to_midi: (args) => {
          current = {
            ...current,
            midiPort: args.listening === true ? "Stub Keyboard" : null,
          };
          return current;
        },
        /*
         * The rest of the capture surface. The stub is not a transcriber and
         * has no keyboard plugged into it: what it answers is the state
         * machine the facade guarantees — listening, capturing, a take to
         * review — so that a screen showing one of those states is showing a
         * state the core can actually be in. An unanswered command here is a
         * failure notice on every screen that opens, which is what these
         * were missing.
         */
        select_midi_input: (args) => {
          current = { ...current, midiPort: String(args.id) };
          return current;
        },
        start_midi_capture: () => capture({ state: "capturing", captureEvents: 0, captureMicros: 0 }),
        stop_midi_capture: () => capture({ state: "review" }),
        keep_recent_midi: () => capture({ state: "review" }),
        clear_recent_midi: () => capture({ state: "listen", recentEvents: 0, recentMicros: 0 }),
        set_recent_midi: (args) => capture({ recentEnabled: args.enabled === true }),
        review_begin: () => {
          if (!review) throw { kind: "backend", message: "there is no take to review" };
          readings.length = 0;
          return review;
        },
        review_read: () => review,
        review_act: (args) => {
          if (!review) throw { kind: "backend", message: "there is no take to review" };
          if (review.sealed === true) throw { kind: "backend", message: "this reading has been kept" };
          const action = args.action as { kind: string; ambiguity?: string; choice?: string; notes?: number[] };
          readings.push(review);
          window.__musaReviewActs.push(action);
          const marks = review.ambiguities as { id: string; kind: string; choices: { id: string }[] }[];
          // Choosing settles exactly the one mark it answers, and the reading
          // that produced it becomes the current one.
          const settled =
            action.kind === "choose"
              ? marks.filter((mark) => mark.id !== action.ambiguity)
              : action.kind === "tap"
                ? marks.filter((mark) => mark.kind !== "pulse")
                : marks;
          const chosen = marks.find((mark) => mark.id === action.ambiguity);
          review = {
            ...review,
            ambiguities: settled.map((mark) =>
              mark.id === action.ambiguity
                ? {
                    ...mark,
                    choices: mark.choices.map((choice) => ({ ...choice, current: choice.id === action.choice })),
                  }
                : mark,
            ),
            history: [
              ...(review.history as string[]),
              action.kind === "choose"
                ? ((chosen?.choices.find((choice) => choice.id === action.choice) as { label?: string })?.label ??
                  "Chose a reading.")
                : action.kind === "tap"
                  ? "Took the pulse from the taps."
                  : `Wrote ${(action.notes ?? []).length} notes again.`,
            ],
            changed: action.notes ?? [],
          };
          return review;
        },
        review_undo: () => {
          const previous = readings.pop();
          if (!previous) throw { kind: "backend", message: "there is nothing to take back" };
          review = previous;
          return review;
        },
        review_audition: (args) => {
          if (!review) throw { kind: "backend", message: "there is no take to review" };
          review = { ...review, audition: args.mode };
          return review;
        },
        review_accept: () => {
          if (!review) throw { kind: "backend", message: "there is no take to review" };
          review = { ...review, sealed: true };
          return review;
        },
        review_discard: () => {
          review = null;
          readings.length = 0;
          return null;
        },
        review_placement_plan: (args) => planned((args.voices as string[]) ?? []),
        // Keeping is one revision or none: the plan is recomputed, refused as
        // a whole, or written as a whole. That the text it writes is the
        // right text is `musa-project`'s placement laws, not this file's.
        review_place: (args) => {
          const plan = planned((args.voices as string[]) ?? []);
          const notes = ((review?.notes as unknown[]) ?? []).length;
          if (refusePlacement !== null) {
            const message = refusePlacement;
            refusePlacement = null;
            throw { kind: "document", message };
          }
          history.push(current);
          const revision = (current.revision as number) + 1;
          // The stub cannot compile, so the notes it reports as written are
          // the last notes of the score it has. What the tests judge is that
          // the interface selects exactly what the report names, which is
          // true of any ids the project could return.
          const all = (((current.score as Score | null)?.events ?? []) as { id: string }[]).map((event) => event.id);
          const wrote = all.slice(Math.max(0, all.length - notes));
          current = { ...current, revision, scoreRevision: revision, unsaved: true };
          answer();
          review = null;
          readings.length = 0;
          return {
            revision,
            summary: plan.summary,
            part: plan.part,
            voices: (plan.voices as { name: string }[]).map((line) => line.name),
            events: wrote,
          };
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
          window.__musaRevision = current.revision as number;
          window.__musaEmit("musa://snapshot", current);
        },
      });

      // The take a review reads. The project composes this from a capture;
      // the stub is handed one, because a headless browser has no keyboard
      // plugged into it.
      Object.defineProperty(window, "__musaReview", {
        value: (facts: Record<string, unknown> | null) => {
          review = facts;
          readings.length = 0;
        },
      });

      // The one refusal the stub cannot reach on its own: whether the
      // document the phrase would write compiles is the compiler's answer,
      // and there is no compiler here.
      Object.defineProperty(window, "__musaRefusePlacement", {
        value: (message: string | null) => {
          refusePlacement = message;
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
    /** Every group transformation it has committed, in order. */
    __musaGroupEdits: Record<string, unknown>[];
    /** Seed the take a review reads, or clear it. */
    __musaReview: (facts: Record<string, unknown> | null) => void;
    /** Every review gesture the interface has made, in order. */
    __musaReviewActs: Record<string, unknown>[];
    /** Make the next placement be refused, the way the compiler would. */
    __musaRefusePlacement: (message: string | null) => void;
    /** The revision the stub last answered with, so undo can be seen to land. */
    __musaRevision: number;
    /** How many times the interface has asked the shell to save. */
    __musaSaves: number;
  }
}
