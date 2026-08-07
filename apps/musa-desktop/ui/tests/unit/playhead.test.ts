/**
 * The playhead's two laws (`03-interaction.md` §4, budgets B5 and B10).
 *
 * B5 — jitter under one frame — is a statement about interpolation, so it is
 * measured against a clock the test owns rather than sampled through a
 * browser: a real animation frame is scheduled by the compositor, and a
 * number taken from it measures the machine, not the code. What the code
 * must guarantee is that between two engine positions the playhead advances
 * linearly, and that it never invents a performance the engine is not having.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { Playhead, soundingAt } from "../../src/lib/state/playhead.svelte";
import type { EventFacts, PlaybackState } from "../../src/lib/state/snapshot";

const RATE = 48_000;

/** How often the engine reports, in milliseconds (prompt 18's cadence). */
const INTERVAL_MS = 50;

/** One display frame at 60 Hz, which is the unit B5 is stated in. */
const FRAME_MS = 1000 / 60;

function playback(positionFrames: number, playing = true): PlaybackState {
  return { playing, positionFrames, totalFrames: RATE * 10, sampleRate: RATE, loopRegion: null };
}

/** A hand-driven `requestAnimationFrame`, so the test owns the clock. */
class Frames {
  #pending: FrameRequestCallback[] = [];
  now = 0;

  install(): void {
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      this.#pending.push(callback);
      return this.#pending.length;
    });
    vi.stubGlobal("cancelAnimationFrame", () => {
      this.#pending = [];
    });
    vi.stubGlobal("performance", { now: () => this.now });
  }

  /** Advance the clock by one display frame and run whatever was scheduled. */
  tick(by = FRAME_MS): void {
    this.now += by;
    const due = this.#pending;
    this.#pending = [];
    for (const callback of due) callback(this.now);
  }

  get scheduled(): number {
    return this.#pending.length;
  }
}

let frames: Frames;

beforeEach(() => {
  frames = new Frames();
  frames.install();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the playhead between engine positions", () => {
  it("B5: never deviates from linear by as much as a frame", () => {
    const playhead = new Playhead();
    playhead.receive(playback(0), 0);
    // One interval, so it knows the engine's cadence.
    frames.now = INTERVAL_MS;
    playhead.receive(playback((RATE * INTERVAL_MS) / 1000), frames.now);

    const start = playhead.frame;
    const worst = { at: 0, off: 0 };
    for (let step = 1; step <= 3; step += 1) {
      frames.tick();
      const elapsed = frames.now - INTERVAL_MS;
      const expected = start + (elapsed / 1000) * RATE;
      const off = Math.abs(playhead.frame - expected);
      if (off > worst.off) worst.off = off;
      worst.at = elapsed;
    }
    // Expressed in the unit the budget uses: frames of *display*, not audio.
    const drift = (worst.off / RATE) * 1000;
    expect(drift, `worst deviation ${drift.toFixed(3)} ms`).toBeLessThan(FRAME_MS);
  });

  it("stops rather than inventing a performance the engine is not having", () => {
    const playhead = new Playhead();
    playhead.receive(playback(0), 0);
    frames.now = INTERVAL_MS;
    playhead.receive(playback((RATE * INTERVAL_MS) / 1000), frames.now);

    // The engine goes quiet. The playhead may lead it by one interval and no
    // further: a silent engine is a stalled transport, not a fast one.
    for (let step = 0; step < 40; step += 1) frames.tick();
    const led = ((playhead.frame - (RATE * INTERVAL_MS) / 1000) / RATE) * 1000;
    expect(led, `led the engine by ${led.toFixed(1)} ms`).toBeLessThanOrEqual(INTERVAL_MS * 2);
  });

  it("B10: schedules nothing once the transport stops", () => {
    const playhead = new Playhead();
    playhead.receive(playback(0), 0);
    frames.tick();
    expect(frames.scheduled, "a frame is pending while playing").toBeGreaterThan(0);

    playhead.receive(playback(1000, false), frames.now);
    expect(frames.scheduled, "nothing is pending once stopped").toBe(0);
    frames.tick();
    expect(frames.scheduled).toBe(0);
  });

  it("schedules nothing at all before the transport ever runs", () => {
    const playhead = new Playhead();
    playhead.receive(playback(0, false), 0);
    expect(frames.scheduled).toBe(0);
  });
});

describe("what is sounding", () => {
  const event = (id: string, onsetFrames: number, endFrames: number, kind = "note"): EventFacts =>
    ({ id, kind, onsetFrames, endFrames }) as EventFacts;

  const events = [
    event("a", 0, 100),
    event("b", 100, 200),
    event("c", 100, 200),
    event("r", 200, 300, "rest"),
  ];

  it("reads the frames the core supplied, half-open at the end", () => {
    expect(soundingAt(events, 0)).toEqual(["a"]);
    expect(soundingAt(events, 99)).toEqual(["a"]);
    // A note ends exactly where the next begins; both must not sound at once.
    expect(soundingAt(events, 100)).toEqual(["b", "c"]);
  });

  it("never tints a rest", () => {
    expect(soundingAt(events, 250)).toEqual([]);
  });
});
