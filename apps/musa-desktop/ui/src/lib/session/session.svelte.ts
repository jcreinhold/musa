/**
 * The session: one snapshot, and every way the interface can ask for a new
 * one.
 *
 * The core owns the document; this class owns nothing musical. It holds the
 * latest snapshot the shell sent, the words currently in the top margin, and
 * the small amount of interface state that is genuinely about *editing* — the
 * draft the user is typing, and whether the source column has opened itself yet.
 *
 * Roadmap §14.2: state has one owner. When a command answers and an event
 * also arrives, both carry the same snapshot, and taking the newer revision
 * makes that harmless.
 */

import { mark } from "../perf";
import { bridge, inShell, isFailure } from "./bridge";
import type { ErrorDto } from "./generated/ErrorDto";
import type { ExportTargetDto } from "./generated/ExportTargetDto";
import type { TemplateDto } from "./generated/TemplateDto";
import type { EditDto } from "./generated/EditDto";
import type { StudioEditDto } from "./generated/StudioEditDto";
import type { EditImpact, MidiEntry, ProjectSnapshot } from "../state/snapshot";

/**
 * How long typing settles before the source is compiled
 * (`06-performance.md` §3). Long enough that a word is one compile, short
 * enough that a pause reads as immediate.
 */
export const SETTLE_MS = 180;

/**
 * A frame count on its way to the shell.
 *
 * `ts-rs` spells Rust's `u64` as `bigint`, but the bridge is JSON and
 * `JSON.stringify` refuses a `BigInt`. A frame count is exact as a number
 * well past the length of any performance, so the number is what crosses and
 * the cast is the honest way to say so in one place.
 */
function frames(count: number): bigint {
  return Math.max(Math.round(count), 0) as unknown as bigint;
}

/** What the top margin is currently saying, and in what voice. */
export interface Notice {
  tone: "result" | "failure";
  message: string;
}

/** How long a completed operation stays in the top margin (`05-states.md` §6). */
const NOTICE_MS = 3000;

/**
 * The half of the bridge the session uses. Named as an interface so a test
 * can answer it directly — a session that could only be exercised through a
 * running shell would be a session nothing ever tests.
 */
export type Link = Pick<
  typeof bridge,
  | "openProject"
  | "newProject"
  | "apply"
  | "editImpact"
  | "transport"
  | "exportTo"
  | "snapshot"
  | "on"
  | "askToOpen"
  | "askToSave"
  | "listenToMidi"
>;

/**
 * The commands whose answer is a new source.
 *
 * `setSource` is not one of them: it is the draft being sent, not a rewrite
 * arriving, and dropping the draft on its way back would undo whatever was
 * typed while it was in flight.
 */
const REWRITES = new Set([
  "format",
  "undo",
  "redo",
  "editScore",
  "editStudio",
  "restoreRecovery",
]);

/**
 * The file extension an export writes, for the targets whose name is not it.
 * The core decides the real extension; this is only what the save dialog
 * suggests before the export has run.
 */
const EXTENSIONS: Partial<Record<ExportTargetDto, string>> = {
  lilyPond: "ly",
  musicXml: "musicxml",
};

export class Session {
  snapshot = $state<ProjectSnapshot | null>(null);
  notice = $state<Notice | null>(null);

  /**
   * The text as the user has typed it, before the core has answered. Cleared
   * the moment a snapshot carrying it arrives, so the textarea shows the
   * document again rather than a copy of it.
   */
  draft = $state<string | null>(null);

  /**
   * The source column opens itself the first time a session's source goes invalid,
   * and thereafter respects whatever the user last chose (`05-states.md` §4).
   */
  sourceOpen = $state(false);

  /**
   * What to do with notes played in on a MIDI keyboard.
   *
   * A callback rather than state, because a played note is an event and not a
   * condition: it is written once, where the caret is at that moment. The
   * workspace sets this; the session only carries it.
   */
  played: ((entry: MidiEntry) => void) | null = null;
  #announcedProblems = false;

  #settle: ReturnType<typeof setTimeout> | undefined;
  #issued = 0;
  #applied = 0;
  #noticeTimer: ReturnType<typeof setTimeout> | undefined;

  readonly #link: Link | null;

  /**
   * Without a link the session is a viewer: it renders whatever snapshot it
   * was seeded with and every command is a no-op. That is the mode the
   * engraving goldens are taken in.
   */
  constructor(link: Link | null = inShell() ? bridge : null) {
    this.#link = link;
  }

  /** Whether anything is answering, as opposed to a fixture on screen. */
  get live(): boolean {
    return this.#link !== null;
  }

  /**
   * Which piece is on screen, or null before one is.
   *
   * Watched rather than read: everything the interface remembers about a
   * piece — the selection, the loop, the marks — names events that only
   * exist in it, so a change here is the signal to forget all of it.
   */
  get document(): number | null {
    return this.snapshot?.document ?? null;
  }

  /** The source to edit: the draft if there is one, else the document's. */
  get text(): string {
    return this.draft ?? this.snapshot?.source ?? "";
  }

  /** The current source has problems and the score on screen is older. */
  get stale(): boolean {
    return this.snapshot !== null && !this.snapshot.compiles;
  }

  /** The revision the engraving on screen belongs to. */
  get shownRevision(): number | null {
    return this.snapshot?.scoreRevision ?? null;
  }

  /**
   * Adopt a snapshot from either a command's answer or a `musa://snapshot`
   * event, whichever arrived second.
   *
   * Revisions are only comparable within one piece: every session starts at
   * revision 0, so a freshly opened score is *older* than whatever was on
   * screen by that measure. So the document is read first, and what happens
   * when it differs depends on who asked. `opening` is the interface saying
   * "this is the piece I asked for"; without it, a snapshot about some other
   * document is an answer to a question the composer has moved past — an
   * edit that was still compiling when they opened something else — and
   * adopting it would put the piece they closed back on the screen.
   */
  receive(snapshot: ProjectSnapshot, opening = false): void {
    mark("snapshot");
    const current = this.snapshot;
    if (current !== null && current.document !== snapshot.document) {
      if (!opening) return;
      this.#adopt();
    }
    if (current?.document === snapshot.document && snapshot.revision < current.revision) return;
    this.snapshot = snapshot;
    if (this.draft !== null && this.draft === snapshot.source) this.draft = null;
    if (!snapshot.compiles && !this.#announcedProblems) {
      this.#announcedProblems = true;
      this.sourceOpen = true;
    }
  }

  /**
   * Forget everything that was about the piece being replaced.
   *
   * A draft is text the composer typed into the *previous* document, and a
   * compile of it may still be in flight; both are meaningless the moment a
   * different piece arrives, and keeping the draft would show the old piece's
   * words over the new piece's score. The problems latch resets too, so a
   * second piece that does not compile opens the source column for the same
   * reason the first one did (`05-states.md` §4).
   */
  #adopt(): void {
    clearTimeout(this.#settle);
    this.draft = null;
    this.#issued = 0;
    this.#applied = 0;
    this.#announcedProblems = false;
  }

  /** Show a failure until it is superseded (`05-states.md` §7). */
  fail(thrown: unknown): void {
    const failure: ErrorDto | null = isFailure(thrown) ? thrown : null;
    this.say({
      tone: "failure",
      message: failure?.message ?? String(thrown),
    });
  }

  /** Put a line in the top margin; results fade, failures persist. */
  say(notice: Notice): void {
    clearTimeout(this.#noticeTimer);
    this.notice = notice;
    if (notice.tone === "result") {
      this.#noticeTimer = setTimeout(() => (this.notice = null), NOTICE_MS);
    }
  }

  dismiss(): void {
    clearTimeout(this.#noticeTimer);
    this.notice = null;
  }

  /**
   * Start listening and ask for whatever the shell already has. Outside the
   * shell this does nothing: the caller seeds a fixture instead.
   */
  async start(): Promise<() => void> {
    const link = this.#link;
    if (!link) return () => {};
    const stop = await Promise.all([
      link.on("musa://snapshot", (snapshot) => this.receive(snapshot)),
      link.on("musa://position", (playback) => {
        if (this.snapshot) this.snapshot = { ...this.snapshot, playback };
      }),
      link.on("musa://transport", (playback) => {
        if (this.snapshot) this.snapshot = { ...this.snapshot, playback };
      }),
      link.on("musa://midi", (entry) => this.played?.(entry)),
    ]);
    // No piece open yet is the launch state, not a failure.
    await link.snapshot().then(
      (snapshot) => this.receive(snapshot),
      () => undefined,
    );
    return () => {
      for (const unsubscribe of stop) unsubscribe();
    };
  }

  /** Type into the source. One compile per pause, never one per keystroke. */
  edit(source: string): void {
    mark("edit");
    this.draft = source;
    clearTimeout(this.#settle);
    this.#settle = setTimeout(() => void this.compile(source), SETTLE_MS);
  }

  /** Compile the draft now, without waiting for the pause. */
  async compile(source: string): Promise<void> {
    clearTimeout(this.#settle);
    const link = this.#link;
    if (!link || source === this.snapshot?.source) return;
    const issue = ++this.#issued;
    try {
      const snapshot = await link.apply({ kind: "setSource", source });
      // A compile that finished after a later one started is stale: the user
      // has typed on, and its snapshot describes text that no longer exists.
      if (issue < this.#applied) return;
      this.#applied = issue;
      this.receive(snapshot);
    } catch (thrown) {
      this.fail(thrown);
    }
  }

  async open(path?: string): Promise<void> {
    const link = this.#link;
    if (!link) return;
    try {
      const chosen = path ?? (await link.askToOpen());
      if (chosen === null) return;
      this.receive(await link.openProject(chosen), true);
      this.dismiss();
    } catch (thrown) {
      this.fail(thrown);
    }
  }

  async create(template: TemplateDto = "piece"): Promise<void> {
    const link = this.#link;
    if (!link) return;
    try {
      this.receive(await link.newProject(template, null), true);
      this.dismiss();
    } catch (thrown) {
      this.fail(thrown);
    }
  }

  async save(): Promise<void> {
    await this.run({ kind: "save" }, (snapshot) => `Saved ${snapshot.name}.`);
  }

  /**
   * Read a MIDI keyboard while note entry is on, and stop when it goes off.
   *
   * Turning it on names the keyboard that answered, because a composer who
   * plugged one in wants to know it was found — and a composer who did not
   * gets no message at all, since not having a keyboard is not a problem.
   *
   * `caret` is the event a played note would be written before, and is sent
   * again whenever it moves: what a note is spelled as depends on the key in
   * force there, and a piece modulates.
   */
  async listenToMidi(listening: boolean, caret: string | null = null): Promise<void> {
    const link = this.#link;
    if (!link) return;
    try {
      const snapshot = await link.listenToMidi(listening, caret);
      this.receive(snapshot);
      if (listening && snapshot.midiPort) {
        this.say({ tone: "result", message: `Playing in from ${snapshot.midiPort}.` });
      }
    } catch (thrown) {
      this.fail(thrown);
    }
  }

  /** Take the work a crash left behind, or decline it (roadmap §15.7). */
  async recover(take: boolean): Promise<void> {
    await this.run(
      { kind: take ? "restoreRecovery" : "discardRecovery" },
      take ? () => "Restored the unsaved work." : undefined,
    );
  }

  async format(): Promise<void> {
    await this.run({ kind: "format" }, () => "Formatted the source.");
  }

  /**
   * What a score edit would change, or null if the core cannot say.
   *
   * Asked before the edit, because `04-provenance.md` §4's choice has to
   * state its consequence in counts and the frontend does not know them.
   */
  async impact(edit: EditDto): Promise<EditImpact | null> {
    const link = this.#link;
    if (!link) return null;
    try {
      return await link.editImpact(edit);
    } catch (thrown) {
      this.fail(thrown);
      return null;
    }
  }

  /**
   * Issue a structured score edit. Reports whether it landed, so a caller
   * that was mid-choice knows whether to close it.
   */
  async editScore(edit: EditDto, said?: string): Promise<boolean> {
    const link = this.#link;
    if (!link) return false;
    try {
      this.receive(await link.apply({ kind: "editScore", edit }));
      if (said) this.say({ tone: "result", message: said });
      return true;
    } catch (thrown) {
      this.fail(thrown);
      return false;
    }
  }

  /**
   * Issue a structured studio edit — a knob, a fader, an assignment.
   *
   * The same call shape as a score edit, because it is the same kind of act:
   * the source is rewritten, transactionally, and the answer is the new
   * document (roadmap §11).
   */
  async editStudio(edit: StudioEditDto, said?: string): Promise<boolean> {
    const link = this.#link;
    if (!link) return false;
    try {
      this.receive(await link.apply({ kind: "editStudio", edit }));
      if (said) this.say({ tone: "result", message: said });
      return true;
    } catch (thrown) {
      this.fail(thrown);
      return false;
    }
  }

  async undo(): Promise<void> {
    await this.run({ kind: "undo" });
  }

  async redo(): Promise<void> {
    await this.run({ kind: "redo" });
  }

  async play(): Promise<void> {
    await this.move({ kind: "play" });
  }

  async stop(): Promise<void> {
    await this.move({ kind: "stop" });
  }

  /** `Space`: play, or stop if the transport is already running. */
  async toggle(): Promise<void> {
    await (this.snapshot?.playback.playing === true ? this.stop() : this.play());
  }

  /** `⇧Space`: play from a point in the performance, in frames. */
  async playFrom(frame: number): Promise<void> {
    await this.move({ kind: "seek", frame: frames(frame) });
    await this.play();
  }

  /** Loop a region of the performance, or stop looping. */
  async loop(region: [number, number] | null): Promise<void> {
    await this.move(
      region === null
        ? { kind: "clearLoop" }
        : { kind: "setLoop", start: frames(region[0]), end: frames(region[1]) },
    );
  }

  /** Write an export where the user chooses, and name the file it wrote. */
  async exportTo(target: ExportTargetDto): Promise<void> {
    const link = this.#link;
    if (!link) return;
    const extension = EXTENSIONS[target] ?? target;
    const stem = (this.snapshot?.name ?? "piece").replace(/\.musa$/, "");
    try {
      const path = await link.askToSave(`${stem}.${extension}`, extension);
      if (path === null) return;
      const written = await link.exportTo(target, path);
      const name = written.path.split(/[/\\]/).pop() ?? written.path;
      this.say({ tone: "result", message: `Exported ${name}.` });
    } catch (thrown) {
      this.fail(thrown);
    }
  }

  private async run(
    command: Parameters<Link["apply"]>[0],
    said?: (snapshot: ProjectSnapshot) => string,
  ): Promise<void> {
    const link = this.#link;
    if (!link) return;
    // Every command acts on what is on the screen. A draft still inside the
    // settle window has not reached the core yet, so it goes first — without
    // this, formatting a piece the composer had just retyped would answer
    // with the text they replaced.
    if (this.draft !== null) await this.compile(this.draft);
    try {
      const snapshot = await link.apply(command);
      // A command that rewrites the source — a format, an undo, a structured
      // edit — is the document speaking, and the draft was only ever text the
      // core had not seen yet. Keeping it would hide the answer behind the
      // question (roadmap §11: the source is canonical).
      if (REWRITES.has(command.kind)) {
        clearTimeout(this.#settle);
        this.draft = null;
      }
      this.receive(snapshot);
      if (said) this.say({ tone: "result", message: said(snapshot) });
    } catch (thrown) {
      this.fail(thrown);
    }
  }

  private async move(command: Parameters<Link["transport"]>[0]): Promise<void> {
    const link = this.#link;
    if (!link) return;
    try {
      this.receive(await link.transport(command));
    } catch (thrown) {
      this.fail(thrown);
    }
  }
}
