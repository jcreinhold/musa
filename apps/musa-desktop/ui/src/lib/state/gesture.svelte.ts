/**
 * The pointer-gesture state machine: idle → pressed → one axis.
 *
 * A press on a note is not yet a gesture. What it becomes is decided once,
 * when the pointer has moved far enough to mean something, and by two facts
 * only: where the press landed on the note, and which way it went. After that
 * the axis is fixed for the life of the gesture, so a drag never turns into a
 * different edit halfway through.
 *
 * Nothing is written here and nothing is written per pixel. The machine
 * produces a *candidate* — one token's worth of text — and the release turns
 * that into the same `EditScore` command the keyboard issues
 * (`03-interaction.md` §2).
 */

import { renotate, rungsFor, shiftAccidental, shiftStep, stepsFor } from "../score/steps";

/** Which question the gesture turned out to be asking. */
export type Axis = "none" | "pitch" | "accidental" | "duration" | "range";

/** What the pointer went down on, in the page's own coordinates. */
export interface Press {
  /** The note under the pointer. */
  event: string;
  /** Its written pitch, or null for a rest — which has no step to drag. */
  pitch: string | null;
  /** Its written duration, as the source spells it. */
  duration: string;
  /** Whether the press landed in the note's duration handle, at its right edge. */
  atEdge: boolean;
  /** Whether `⌥` was held: the accidental instead of the step. */
  alt: boolean;
  x: number;
  y: number;
  /** How far the pointer must travel to stop being a click, in page units. */
  threshold: number;
}

/** What the gesture would write, if it were released now. */
export interface Candidate {
  event: string;
  kind: "pitch" | "duration";
  /** The text that would go into the token. */
  value: string;
  /**
   * How far the gesture went, in the units of its own axis: diatonic steps
   * for a pitch, rungs of the ladder for a duration. The overlay draws with
   * this; the edit is made from `value`.
   */
  distance: number;
}

export class Gesture {
  #press: Press | null = null;

  /** What the gesture resolved to, once it passed the threshold. */
  axis = $state<Axis>("none");

  /** What it would write. Null while it is still a click, or on the range axis. */
  candidate = $state<Candidate | null>(null);

  /** Whether an edit is in flight — the state the overlay and the source draw. */
  get editing(): boolean {
    return this.axis === "pitch" || this.axis === "accidental" || this.axis === "duration";
  }

  /** The note the gesture started on, while it is running. */
  get event(): string | null {
    return this.#press?.event ?? null;
  }

  /** The pointer went down. Nothing is decided yet, and nothing is written. */
  begin(press: Press): void {
    this.#press = press;
    this.axis = "none";
    this.candidate = null;
  }

  /**
   * The pointer moved. Resolves the axis the first time past the threshold,
   * then re-snaps the candidate.
   */
  move(x: number, y: number, staffSpace: number): void {
    const press = this.#press;
    if (!press) return;
    const dx = x - press.x;
    const dy = y - press.y;
    if (this.axis === "none") {
      if (Math.hypot(dx, dy) < press.threshold) return;
      this.axis = this.#resolve(press, dx, dy);
    }
    this.candidate = this.#snap(press, dx, dy, staffSpace);
  }

  /**
   * The pointer came up. Returns what to write, and forgets the gesture.
   *
   * A release back at the origin has no candidate and writes nothing, which is
   * the "changed my mind" that costs no keystroke.
   */
  release(): Candidate | null {
    const written = this.candidate;
    this.cancel();
    return written;
  }

  /** `Esc`, or the pointer leaving: the document is untouched either way. */
  cancel(): void {
    this.#press = null;
    this.axis = "none";
    this.candidate = null;
  }

  /**
   * Which axis this is. Decided by where the press landed first and by
   * direction second, so the right edge always means duration and never
   * means a very shallow respelling.
   */
  #resolve(press: Press, dx: number, dy: number): Axis {
    if (press.atEdge) return "duration";
    // Horizontal is range selection and stays that way: musical position is
    // statement order, not a coordinate, so sideways has no answer to give.
    if (Math.abs(dy) <= Math.abs(dx)) return "range";
    if (press.pitch === null) return "range";
    return press.alt ? "accidental" : "pitch";
  }

  #snap(press: Press, dx: number, dy: number, staffSpace: number): Candidate | null {
    const pitch = press.pitch;
    switch (this.axis) {
      case "pitch": {
        const steps = stepsFor(dy, staffSpace);
        const value = pitch === null || steps === 0 ? null : shiftStep(pitch, steps);
        if (value === null) return null;
        return { event: press.event, kind: "pitch", value, distance: steps };
      }
      case "accidental": {
        const by = stepsFor(dy, staffSpace);
        const value = pitch === null || by === 0 ? null : shiftAccidental(pitch, by);
        // The ladder has ends, so a long drag stops writing rather than
        // pretending there is a triple sharp.
        return value === null || value === pitch
          ? null
          : { event: press.event, kind: "pitch", value, distance: by };
      }
      case "duration": {
        const rungs = rungsFor(dx, staffSpace);
        const value = rungs === 0 ? null : renotate(press.duration, rungs);
        return value === null || value === press.duration
          ? null
          : { event: press.event, kind: "duration", value, distance: rungs };
      }
      case "none":
      case "range":
        return null;
    }
  }
}
