/**
 * The shape `musa-project` serializes (`ProjectSnapshot`).
 *
 * Written by hand to match `crates/musa-project/src/snapshot.rs` and
 * `facts.rs`; the committed fixture is generated from those types by a Rust
 * test, so a drift between this file and the facade shows up as a red test
 * rather than as a screen that silently renders nothing.
 *
 * The frontend owns nothing musical. Everything here was computed by the
 * core (docs/interface/03-interaction.md §7).
 */

export interface Fraction {
  numerator: number;
  denominator: number;
}

export interface Span {
  start: number;
  end: number;
}

export interface Diagnostic {
  severity: "error" | "warning";
  message: string;
  span: Span | null;
}

export interface OriginFacts {
  generated: boolean;
  /** Containment order, outside in: `["transpose down P5", "sigh()"]`. */
  path: string[];
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
  path: string[];
  /** The path as one line, for a margin bracket's label. */
  label: string;
  /** The motif's name, when a motif produced this. */
  motif: string | null;
  /** Where that motif is declared, for revealing it in the drawer. */
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
  /** The same pitches as the source spells them (`gs4`), for editing. */
  pitchSpellings: string[];
  duration: Fraction;
  durationSpelling: string;
  bar: number;
  beat: Fraction;
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

export interface ProjectSnapshot {
  name: string;
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
}

/** The event with this id, or undefined. */
export function eventById(snapshot: ProjectSnapshot, id: string): EventFacts | undefined {
  return snapshot.score?.events.find((event) => event.id === id);
}
