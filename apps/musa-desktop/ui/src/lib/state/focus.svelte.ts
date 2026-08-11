/**
 * The shared focus: what the reader is looking at, marked in both views at
 * once.
 *
 * Selection is a committed act and Origin view is a held one; this is neither.
 * It is attention — where the pointer is, or where the arrow keys just landed
 * — and it exists because the two hard questions about a musa score are
 * *which note did this line write* and *which line wrote this note*, and both
 * have plural answers a composer cannot guess.
 *
 * One focus, not a score hover and a source hover that agree. Whatever is
 * focused resolves to the same three things wherever it came from: the notes
 * to mark on the page, the statement that spells them, and the statement that
 * placed them.
 *
 * Everything here is read from the snapshot. Which notes one statement spelled
 * is a fact about expansion and the core computed it; this module indexes what
 * it was given (`03-interaction.md` §7).
 */

import type { EventFacts, ProjectSnapshot, Span } from "./snapshot";

/**
 * What attention is on: a note on the page, or a line in the text.
 *
 * A line and not a character, because a line is what a reader points at. The
 * editor supplies its own extent — that is the editor answering a question
 * about its own document, not the frontend deriving anything musical
 * (`03-interaction.md` §7).
 */
export type FocusTarget =
  | { kind: "event"; id: string }
  | { kind: "line"; from: number; to: number };

/** What one focus marks, in both views at once. */
export interface Marked {
  /** Every note to hairline on the page. */
  events: string[];
  /**
   * The statement that spells the focused music, when it is not the statement
   * that placed it — a note inside a `motif` body. Marked in the source with
   * a hairline under the text.
   */
  definition: Span | null;
  /**
   * The statement that placed it: the `use`, or the note itself when it was
   * authored. Its line number is marked.
   */
  place: Span | null;
}

/** Nothing focused — the shared empty value, allocated once. */
const NOTHING: Marked = Object.freeze({ events: [], definition: null, place: null });

function keyOf(span: Span): string {
  return `${span.start}:${span.end}`;
}

function same(a: Span, b: Span): boolean {
  return a.start === b.start && a.end === b.end;
}

/** One span that names an event, in the flat list the containment scan walks. */
interface Interval {
  start: number;
  end: number;
  id: string;
}

/**
 * The two directions, resolved.
 *
 * Built once per score rather than once per pointer move: hover is a
 * pointer-rate event and has to be frame-local (`06-performance.md`).
 */
class Index {
  /** Events grouped by the statement that spells them: one line's plural answer. */
  readonly #spelled = new Map<string, string[]>();
  /** Every span that names an event — where written, where spelled — by start. */
  readonly #intervals: Interval[] = [];
  /** The widest of them, which bounds how far back a containment scan looks. */
  #widest = 0;

  constructor(events: readonly EventFacts[]) {
    for (const event of events) {
      const spelled = event.origin.definitionSpan;
      const key = keyOf(spelled);
      const kin = this.#spelled.get(key);
      if (kin) kin.push(event.id);
      else this.#spelled.set(key, [event.id]);

      this.#add(event.origin.span, event.id);
      if (!same(spelled, event.origin.span)) this.#add(spelled, event.id);
    }
    this.#intervals.sort((a, b) => a.start - b.start);
  }

  #add(span: Span, id: string): void {
    if (span.end <= span.start) return;
    this.#intervals.push({ start: span.start, end: span.end, id });
    this.#widest = Math.max(this.#widest, span.end - span.start);
  }

  /** Every note this statement spelled, across every occurrence, in score order. */
  spelled(span: Span): string[] {
    return this.#spelled.get(keyOf(span)) ?? [];
  }

  /**
   * Every note whose text overlaps `[from, to)` — the notes written there, and
   * the notes a `motif` body line spells, which are different events.
   */
  within(from: number, to: number): string[] {
    const intervals = this.#intervals;
    // The first interval that starts too late to overlap; everything after it
    // starts later still.
    let high = intervals.length;
    let low = 0;
    while (low < high) {
      const middle = (low + high) >> 1;
      if ((intervals[middle]?.start ?? 0) < to) low = middle + 1;
      else high = middle;
    }
    const found: string[] = [];
    // …and nothing starting more than the widest span back can reach it.
    for (let index = low - 1; index >= 0; index -= 1) {
      const interval = intervals[index];
      if (!interval || from - interval.start > this.#widest) break;
      if (interval.end > from && !found.includes(interval.id)) found.push(interval.id);
    }
    return found.reverse();
  }
}

export class Focus {
  readonly #read: () => ProjectSnapshot | null;

  /**
   * What the pointer is over, and whether a pointer is in a view that reports
   * focus at all. The two are separate because "over blank paper" and "not
   * pointing at anything" are different states: the first marks nothing and
   * the second hands the focus back to the keyboard.
   */
  pointer = $state<FocusTarget | null>(null);
  pointing = $state(false);

  /**
   * Where the keyboard left it. Set from the selection, which is what the
   * arrow keys move and what the source caret already selects — so focus
   * follows the keyboard without a second thing to keep in step, and the
   * feature is on for someone who never hovers.
   */
  keyboard = $state<FocusTarget | null>(null);

  #index: Index | undefined;
  #built: number | undefined;

  constructor(read: () => ProjectSnapshot | null) {
    this.#read = read;
  }

  /** The focus: the pointer's while there is one, the keyboard's otherwise. */
  get target(): FocusTarget | null {
    return this.pointing ? this.pointer : this.keyboard;
  }

  /** The pointer moved onto a note, or onto nothing. */
  point(id: string | null): void {
    this.pointing = true;
    this.pointer = id === null ? null : { kind: "event", id };
  }

  /** The pointer moved over a line of the text, or off the text. */
  pointLine(line: { from: number; to: number } | null): void {
    this.pointing = true;
    this.pointer = line === null ? null : { kind: "line", from: line.from, to: line.to };
  }

  /** The pointer left. The last focus does not linger. */
  leave(): void {
    this.pointing = false;
    this.pointer = null;
  }

  /** The keyboard landed on a note, or on nothing. */
  land(id: string | undefined): void {
    this.keyboard = id === undefined ? null : { kind: "event", id };
  }

  get #spans(): Index {
    const snapshot = this.#read();
    const revision = snapshot?.scoreRevision ?? -1;
    if (!this.#index || this.#built !== revision) {
      this.#index = new Index(snapshot?.score?.events ?? []);
      this.#built = revision;
    }
    return this.#index;
  }

  /** How many notes one statement spelled — the plural answer, as a number. */
  spelled(span: Span | undefined): number {
    return span ? this.#spans.spelled(span).length : 0;
  }

  /** What the focus marks right now, in both views. */
  get marked(): Marked {
    const target = this.target;
    if (!target) return NOTHING;
    if (target.kind === "line") {
      const events = this.#spans.within(target.from, target.to);
      return events.length === 0 ? NOTHING : { events, definition: null, place: null };
    }
    const event = (this.#read()?.score?.events ?? []).find(
      (candidate) => candidate.id === target.id,
    );
    if (!event) return NOTHING;
    const spelled = event.origin.definitionSpan;
    const kin = this.#spans.spelled(spelled);
    return {
      // Every note that same statement spelled, not only the one under the
      // pointer: showing one of six is a lie the editing choice later has to
      // correct with a number (`04-provenance.md` §4).
      events: kin.length > 0 ? kin : [event.id],
      definition: same(spelled, event.origin.span) ? null : spelled,
      place: event.origin.span,
    };
  }
}
