/**
 * The shape `musa-project` serializes (`ProjectSnapshot`).
 *
 * Written by hand to match `crates/musa-project/src/snapshot.rs` and
 * `facts.rs`; the committed fixture is generated from those types by a Rust
 * test, so a drift between this file and the facade shows up as a red test
 * rather than as a screen that silently renders nothing.
 *
 * The frontend owns nothing musical. Everything here was computed by the
 * core (docs/rules/desktop/03-interaction.md §7).
 */

import type { HeaderFieldDto } from "../session/generated/HeaderFieldDto";

export interface Fraction {
  numerator: number;
  denominator: number;
}

export interface Span {
  start: number;
  end: number;
}

/**
 * Where a diagnostic points, as a person would say it. 1-based, columns in
 * characters. Computed in Rust — `03-interaction.md` §7 does not let the
 * frontend count lines.
 */
export interface Position {
  line: number;
  column: number;
}

/** One place a diagnostic points at, and what is true about it. */
export interface Label {
  span: Span;
  at: Position;
  /** What is wrong *here*, in a few words. Never the message again. */
  text: string;
  primary: boolean;
}

/** One replacement inside a fix. An empty span inserts. */
export interface FixEdit {
  span: Span;
  replacement: string;
}

/** An edit that resolves a diagnostic, offered only when it is certain. */
export interface Fix {
  /** What applying it does, phrased as the action: ``add `;` ``. */
  title: string;
  edits: FixEdit[];
}

export interface Diagnostic {
  severity: "error" | "warning";
  /** Stable, kebab-case: the argument to `musa explain`. */
  code: string;
  message: string;
  labels: Label[];
  help: string | null;
  note: string | null;
  fixes: Fix[];
  /** The primary label's span, for the editor's lint decoration. */
  span: Span | null;
}

/**
 * What kind of expansion one step of a path was.
 *
 * Origin does not say "something happened here" — a composer reading a
 * generated note asks *which* construct made it, and the answer chooses the
 * word the row prints and the icon beside it.
 */
export type StepKind = "occurrence" | "instance" | "transform" | "assertion" | "splice" | "specialization";

/**
 * One step of an expansion path, as the Origin row reads it.
 *
 * `span` is what makes a step navigable: the call site, instance site, or
 * override site the step names. Null where the step is not a place in the
 * source — a transform is an argument to a block, and an assertion is a claim
 * about music rather than a point in it.
 */
export interface StepFact {
  label: string;
  kind: StepKind;
  span: Span | null;
}

export interface OriginFacts {
  generated: boolean;
  /** Containment order, outside in: `transpose down P5 ▸ sigh()`. */
  path: StepFact[];
  noteIndex: number | null;
  line: number;
  span: Span;
  /**
   * The statement that spells this event: the note inside the `motif` body
   * when generated, the same as `span` when authored. What an edit-definition
   * edit rewrites (`04-provenance.md` §4).
   */
  definitionSpan: Span;
  /** The occurrence that produced this event, when it was generated. */
  occurrence: string | null;
  /**
   * The decision this event was played under, as an index into
   * {@link ScoreFacts.decisions}. Null in a determinate piece, and null for
   * an event that no open construct covers.
   */
  decision: number | null;
}

/**
 * What an edit would change, computed by the core before it is made.
 *
 * The counts of `04-provenance.md` §4's inline choice, and the events it
 * haloes. The frontend derives none of it: which notes one statement spelled
 * is a fact about expansion.
 */
export interface EditImpact {
  generated: boolean;
  /** The motif an edit-definition edit would rewrite. */
  motif: string | null;
  /** The occurrence the edited note belongs to, labelled as the inspector labels it. */
  occurrence: string | null;
  /** How many occurrences would change. */
  occurrences: number;
  /** Every event that would change. */
  events: string[];
  /**
   * Whether this note can be changed on its own. False when its call runs
   * more than once: an override belongs to the call, not to one run of it.
   */
  specializable: boolean;
  /**
   * The text the edit would write, and where — one entry per replacement.
   * This is what a live pointer gesture marks in the source before it commits
   *, and it is the same edit the command would apply.
   */
  writes: CandidateEdit[];
}

/** One replacement a pending edit would make. */
export interface CandidateEdit {
  start: number;
  end: number;
  text: string;
}

/**
 * One expansion that ran: the unit Origin view brackets, traces, and selects.
 *
 * Identity is the whole expansion path, so two `use sigh()` statements are two
 * occurrences even though they read the same on the page.
 */
export interface OccurrenceFacts {
  id: string;
  /** Containment order, outside in. */
  path: StepFact[];
  /** The path as one line, for a margin bracket's label. */
  label: string;
  /** The motif's name, when a motif produced this. */
  motif: string | null;
  /** Where that motif is declared, for revealing it in the source column. */
  declaration: Span | null;
  /** The `use` statement that ran. */
  useSite: Span;
  /** 1-based line of the `use` statement. */
  line: number;
  /** Every event this expansion produced, in score order. */
  events: string[];
}

export interface EventFacts {
  /** The engraved `xml:id` — the same identity the SVG carries. */
  id: string;
  part: string;
  voice: string;
  kind: "note" | "rest" | "chord";
  pitch: string | null;
  pitches: string[];
  /** The same pitches as the source spells them (`g#4`), for editing. */
  pitchSpellings: string[];
  duration: Fraction;
  durationSpelling: string;
  bar: number;
  beat: Fraction;
  /**
   * The key and the clef in force **here**, written out. A piece modulates
   * and a part changes clef, so these are per event; the core answers, and
   * the frontend does not look for the latest change itself
   * (`03-interaction.md` §7).
   */
  key: string | null;
  clef: string | null;
  /**
   * When this event sounds, in the frames the engine reports positions in.
   * Written time and sounding time are different layers; the frontend is
   * given the second rather than deriving it (`03-interaction.md` §7).
   */
  onsetFrames: number;
  endFrames: number;
  origin: OriginFacts;
}

export interface VoiceFacts {
  name: string;
  generated: boolean;
}

export interface PartFacts {
  name: string;
  voices: VoiceFacts[];
}

/**
 * One row of the structural outline: a form marker or a named phrase.
 *
 * The two are anchored differently in the score — one to a time, one to a run
 * of notes — but both resolve to a place, and the core resolves it: `event` is
 * the notehead to bring into view, not a coordinate the frontend guessed at.
 */
export interface OutlineFacts {
  kind: "section" | "phrase";
  name: string;
  bar: number;
  beat: Fraction;
  /** The event to reveal. Null for a marker past the last note. */
  event: string | null;
  onsetFrames: number;
  /** Where the passage it names ends, in the same frames. */
  endFrames: number;
  line: number;
  span: Span;
}

export interface ScoreFacts {
  title: string;
  tempoBpm: number;
  tempoBeat: Fraction;
  key: string | null;
  meterCount: number;
  meterUnit: number;
  parts: PartFacts[];
  events: EventFacts[];
  occurrences: OccurrenceFacts[];
  /** The piece's structure, in the order it is played. */
  outline: OutlineFacts[];
  /**
   * Every statement the piece can make about itself, in the order they are
   * written, whether or not it makes them. A field the piece is
   * silent about is present with a null value — the inspector shows those
   * rows too, and that list is where a composer finds out a piece can name an
   * arranger at all.
   */
  header: HeaderFact[];
  /**
   * Which performance this reading is, or null when the piece asked nothing
   *. A determinate piece has no performance number, and every
   * part of the interface that speaks about realization is silent when this
   * is null.
   */
  performance: number | null;
  /** Every question the piece left open, in the order it asked them. */
  decisions: DecisionFact[];
}

/**
 * One question an open work asked, and the answer this reading gave it.
 *
 * The strings are written by the core, not assembled here: what a decision
 * is called and how its answer reads are musical facts
 * (`docs/rules/desktop/03-interaction.md` §7).
 */
export interface DecisionFact {
  /** The decision's identity, stable across edits elsewhere. Not shown. */
  path: string;
  /** What was left open: `the fill, first choice`. */
  asked: string;
  /** What this reading decided: `4 passes`. */
  answered: string;
  /** Whether the composer kept this one, so a new performance leaves it. */
  pinned: boolean;
  /** The construct that asked, for revealing it in the source column. */
  span: Span;
}

/** One of the piece's own facts, spelled the way the source spells it. */
export interface HeaderFact {
  field: HeaderFieldDto;
  /**
   * `quarter = 72`, not `♩ = 72`. This is exactly what a `setHeader` edit
   * takes back, so reading a field and writing it unchanged changes nothing.
   */
  value: string | null;
}

/**
 * One parameter of one stage, as the Sound workspace reads it.
 *
 * `value` is in the unit's base — seconds, hertz, decibels, or a plain ratio —
 * and `unit` is how the language writes it. The frontend converts nothing:
 * `30 ms` arrives as `0.03` with unit `s`, and an edit sends `0.05` back and
 * the core decides how to spell it.
 */
export interface ParamFacts {
  name: string;
  value: number;
  unit: string;
  /** What a control may write, in that unit. */
  minimum: number;
  maximum: number;
  /** False when the patch never wrote it and this is the declared default. */
  written: boolean;
  span: Span | null;
  /** The signal modulating it, when one does (§13.7). */
  modulatedBy: string | null;
}

export interface StageFacts {
  index: number;
  processor: string;
  label: string | null;
  params: ParamFacts[];
}

export interface ContainerFacts {
  kind: "patch" | "bus" | "signal";
  name: string;
  stages: StageFacts[];
}

export interface AssignmentFacts {
  part: string;
  /** Null for a part the studio never mentions: it keeps the built-in voice. */
  patch: string | null;
}

export interface SendFacts {
  source: string;
  bus: string;
  decibels: number;
  span: Span | null;
}

export interface RouteFacts {
  source: string;
  destination: string;
}

/** The studio, as the Sound and Mix workspaces read it (roadmap §14.4). */
export interface StudioFacts {
  declared: boolean;
  patches: ContainerFacts[];
  buses: ContainerFacts[];
  signals: ContainerFacts[];
  assignments: AssignmentFacts[];
  sends: SendFacts[];
  routes: RouteFacts[];
}

export interface PlaybackState {
  playing: boolean;
  positionFrames: number;
  totalFrames: number;
  sampleRate: number;
  loopRegion: [number, number] | null;
}

/** Notes played in on a MIDI keyboard, spelled and grouped by the core. */
export interface MidiEntry {
  /** One pitch for a note, several for a chord, as the language spells them. */
  pitches: string[];
}

/** A project's running order and its shared material. */
export interface ContentsFacts {
  /** What to call the project: the manifest's name, else the folder's. */
  name: string;
  composer: string | null;
  /** The pieces, in the order they are meant to be read. */
  pieces: EntryFacts[];
  /** The libraries the pieces draw on. */
  material: EntryFacts[];
}

/** One file in a project, as the contents page prints it. */
export interface EntryFacts {
  /** What the composer called it — set in Academico on the page. */
  title: string;
  /** What the filesystem calls it — set in mono, and the name to turn to. */
  file: string;
  current: boolean;
  unsaved: boolean;
  /** Material the piece in hand imports. Always false for a piece. */
  used: boolean;
}

export interface ProjectSnapshot {
  /**
   * Which piece this is. Compared before `revision`, which counts within a
   * document and starts again at zero in the next one.
   */
  document: number;
  name: string;
  /**
   * Which of the three things this file is. Material has no score and never
   * will (roadmap §16), which is a different fact from "no score yet"; a
   * kernel document is a term in the interchange format
   * (`docs/rules/language/01-surface.md` §7) and engraves like a piece, having
   * arrived at the same timeline by a shorter road.
   */
  kind: "piece" | "material" | "kernel";
  source: string;
  revision: number;
  compiles: boolean;
  unsaved: boolean;
  /** Unsaved work has a recovery copy beside the file (roadmap §15.7). */
  autosaved: boolean;
  /** Work a previous session left behind, waiting to be taken or declined. */
  recovery: string | null;
  /** The MIDI keyboard being read, while note entry is on. */
  midiPort: string | null;
  diagnostics: Diagnostic[];
  mei: string | null;
  score: ScoreFacts | null;
  studio: StudioFacts | null;
  scoreRevision: number | null;
  playback: PlaybackState;
  /** The project this piece is one of, or null when nothing is open. */
  contents: ContentsFacts | null;
  /** Every declaration in scope (`08-elaboration.md` §1). */
  terms: TermFacts[];
  /** Every resolved name, for definition and references. */
  names: NameFacts[];
}

/** What a name names. The compiler's own list. */
export type NameKind =
  "value" | "function" | "motif" | "bar" | "fragment" | "part" | "voice" | "patch" | "module" | "template";

/** One type, as a reader is shown it. */
export interface TypeFacts {
  /** As it is spelled in source: `NoteName`, `List<Pitch>`. */
  name: string;
  /**
   * The one line separating this type from the one it is confused with.
   * The disclosure of `08-elaboration.md` §1, never the first sentence.
   */
  distinction: string | null;
}

/** One parameter of a callable declaration. */
export interface ParameterFacts {
  name: string;
  /** The substring of {@link TermFacts.signature} this parameter occupies. */
  label: string;
  ty: TypeFacts;
  /** The default as written, when the caller may omit it. */
  default: string | null;
}

/**
 * Where a declaration is written.
 *
 * Two shapes, because they are two different things. `open` carries a span
 * into the source this snapshot also carries, in the code units the frontend
 * counts in. `library` carries a module and an **opaque** range: those numbers
 * index a document the frontend has never seen, and the only thing it may do
 * with them is hand them back to `libraryDocument`, which restates them in
 * that module's own measure.
 */
export type TermSite = { where: "open"; span: Span } | { where: "library"; uri: string; start: number; end: number };

/** One declaration, as the term tooltip and the completion list read it. */
export interface TermFacts {
  name: string;
  kind: NameKind;
  /** `fn triad(root: NoteName) -> ChordClass`, as the source spells it. */
  signature: string;
  /** The comment block above the declaration, as one paragraph. */
  summary: string | null;
  result: TypeFacts | null;
  parameters: ParameterFacts[];
  /** What to write instead, when the declaration says it is deprecated. */
  deprecation: string | null;
  readOnly: boolean;
  site: TermSite;
}

/** One resolved name: where it is declared, and everywhere it is used. */
export interface NameFacts {
  name: string;
  kind: NameKind;
  /** The declaration's name token, when it is in this document. */
  declaration: Span | null;
  /** The module it is declared in, when it is not. */
  external: string | null;
  /** Every resolved use, in the order the resolver met them. */
  uses: Span[];
}

/** A bundled module opened for reading (`08-elaboration.md` §3). */
export interface LibraryDocument {
  uri: string;
  /** The module as the language spells it: `pitch`, `theory::cadence`. */
  name: string;
  text: string;
  /** What to reveal in it, in this text's own measure. */
  span: Span | null;
}

/**
 * What one analysis saw.
 *
 * An observation, never a verdict: nothing here is a diagnostic, and nothing
 * here is painted red (`08-elaboration.md` §5).
 */
export interface AnalysisFacts {
  /**
   * The score revision this is a reading of.
   *
   * The core's, not a number this side kept: an analysis reads the last valid
   * compile, and only the core knows which one that was. A report whose
   * revision is behind the snapshot's is a reading of an older score, and says
   * so rather than being shown as current (`08-elaboration.md` §8).
   */
  revision: number;
  /** Which analysis ran, by its own name. */
  kind: string;
  /** One line saying what it does. */
  method: string;
  /** The style profile it read against, when it read against one. Absent for
   * the kinds that read the notation alone. */
  profile?: string | null;
  /** What it took for granted, in its own words. */
  assumptions: string[];
  findings: FindingFacts[];
}

/** One thing an analysis saw, and what it has to say for it. */
export interface FindingFacts {
  code: string;
  /** How it stands: a word, never a colour and never a percentage. */
  standing: string;
  summary: string;
  at: Fraction;
  bar: number;
  beat: Fraction;
  evidence: EvidenceFacts;
  /** Each criterion, whether it held, and what it cites. */
  grounds: GroundFacts[];
  rule: RuleFacts | null;
}

/** The rule a finding was read under. */
export interface RuleFacts {
  id: string;
  states: string;
  strength: string;
  cites: string;
}

/** One criterion behind a finding. An unsatisfied ground is the information. */
export interface GroundFacts {
  criterion: string;
  satisfied: boolean;
  cites: string;
}

/** One note a finding points at. */
export interface NoteFacts {
  part: string;
  voice: string;
  /** The engraved event's identity — what a selection is made of. */
  event: string;
  span: Span;
  line: number;
}

/**
 * Where a finding can be seen. Tagged, because the reader's first question is
 * which of the four it is: three are places and the fourth is not.
 */
export type EvidenceFacts =
  | ({ kind: "event" } & NoteFacts)
  | { kind: "passage"; from: Fraction; to: Fraction; notes: NoteFacts[] }
  | { kind: "annotation"; span: Span; line: number }
  | { kind: "inForce" };

/**
 * The project's listing, when there is more than one file to choose between.
 *
 * A project of one is a loose `.musa` file, and the interface shows it nothing
 * — no contents page, no running order, no `⌘0`. The rule lives here rather
 * than in each screen so that "is this a volume" is asked one way everywhere
 * (`07-the-volume.md`).
 */
export function volumeOf(snapshot: ProjectSnapshot | null | undefined): ContentsFacts | null {
  const listing = snapshot?.contents ?? null;
  if (!listing) return null;
  return listing.pieces.length + listing.material.length > 1 ? listing : null;
}

/** The event with this id, or undefined. */
export function eventById(snapshot: ProjectSnapshot, id: string): EventFacts | undefined {
  return snapshot.score?.events.find((event) => event.id === id);
}
