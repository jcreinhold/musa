/**
 * Where the music is, between the engine's position events
 * (`03-interaction.md` §4).
 *
 * The engine owns the clock. It reports a position every so often; at 60 fps
 * a readout that only moved when one arrived would visibly step. So this
 * interpolates *between* events — and never past the last one by more than
 * one interval, so a stalled engine stops the playhead instead of inventing a
 * performance that is not happening.
 *
 * The `requestAnimationFrame` loop runs **only while playing** and is
 * cancelled on stop. That is budget B10: an editor that heats a laptop while
 * nothing happens will not be used.
 */

import type { EventFacts, PlaybackState } from "./snapshot";

/** How far past the last event the playhead may run before it gives up. */
const LEAD_FACTOR = 1;

/** The interval assumed until two events have been seen, in milliseconds. */
const ASSUMED_INTERVAL_MS = 50;

export class Playhead {
  /** The current position in frames, interpolated. */
  frame = $state(0);
  /** Whether the transport is running, as the engine last said. */
  playing = $state(false);

  /** The last reported position, and when it arrived. */
  #reported = 0;
  #at = 0;
  #interval = ASSUMED_INTERVAL_MS;
  #rate = 44_100;
  #frame: number | undefined;

  /**
   * Adopt an engine position. Everything about time comes through here: no
   * timer of the frontend's own ever advances the playhead a single frame
   * further than the engine has actually reached, plus one interval.
   */
  receive(playback: PlaybackState, now: number = performance.now()): void {
    this.#rate = playback.sampleRate;
    if (this.#at !== 0) this.#interval = Math.max(now - this.#at, 1);
    this.#reported = playback.positionFrames;
    this.#at = now;
    this.frame = playback.positionFrames;
    if (playback.playing !== this.playing) {
      this.playing = playback.playing;
      if (playback.playing) this.#run();
      else this.#halt();
    }
  }

  /** Stop interpolating and forget where we were. */
  dispose(): void {
    this.#halt();
  }

  #halt(): void {
    if (this.#frame !== undefined) cancelAnimationFrame(this.#frame);
    this.#frame = undefined;
  }

  #run(): void {
    this.#halt();
    const tick = () => {
      const elapsed = performance.now() - this.#at;
      const lead = Math.min(elapsed, this.#interval * (LEAD_FACTOR + 1));
      this.frame = this.#reported + (lead / 1000) * this.#rate;
      this.#frame = requestAnimationFrame(tick);
    };
    this.#frame = requestAnimationFrame(tick);
  }
}

/**
 * The event sounding at `frame`, per voice — what the score tints.
 *
 * Onsets and ends came from the core (`03-interaction.md` §7); this is a
 * lookup, not a computation about time.
 */
export function soundingAt(events: readonly EventFacts[], frame: number): string[] {
  return events
    .filter((event) => event.kind !== "rest" && event.onsetFrames <= frame && frame < event.endFrames)
    .map((event) => event.id);
}
