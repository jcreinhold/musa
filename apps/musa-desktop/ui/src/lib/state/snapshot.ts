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
}

export interface EventFacts {
  /** The engraved `xml:id` — the same identity the SVG carries. */
  id: string;
  part: string;
  voice: string;
  kind: "note" | "rest" | "chord";
  pitch: string | null;
  pitches: string[];
  duration: Fraction;
  durationSpelling: string;
  bar: number;
  beat: Fraction;
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

export interface ScoreFacts {
  title: string;
  tempoBpm: number;
  tempoBeat: Fraction;
  key: string | null;
  meterCount: number;
  meterUnit: number;
  parts: PartFacts[];
  events: EventFacts[];
}

export interface PlaybackState {
  playing: boolean;
  positionFrames: number;
  totalFrames: number;
  sampleRate: number;
  loopRegion: [number, number] | null;
}

export interface ProjectSnapshot {
  name: string;
  source: string;
  revision: number;
  compiles: boolean;
  unsaved: boolean;
  diagnostics: Diagnostic[];
  mei: string | null;
  score: ScoreFacts | null;
  scoreRevision: number | null;
  playback: PlaybackState;
}

/** The event with this id, or undefined. */
export function eventById(snapshot: ProjectSnapshot, id: string): EventFacts | undefined {
  return snapshot.score?.events.find((event) => event.id === id);
}
