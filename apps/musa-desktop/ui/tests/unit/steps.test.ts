/**
 * What a pointer gesture means, in the score's own units.
 *
 * The rules under test are the ones a composer would state: a step is half a
 * staff space wherever the note is, an accidental is carried rather than
 * recomputed, and a gesture that cannot say what it would write writes
 * nothing.
 */

import { describe, expect, it } from "vitest";

import {
  LADDER,
  RUNG_SPACES,
  STEP_SPACES,
  renotate,
  rungsFor,
  shiftAccidental,
  shiftStep,
  stepsFor,
} from "../../src/lib/score/steps";
import { Gesture, type Press } from "../../src/lib/state/gesture.svelte";

/** The staff space at 50%, 100%, and 200% — the zoom is the only thing that changes. */
const ZOOMS = [5, 10, 20];

describe("a vertical drag", () => {
  it("is the same number of steps at every zoom", () => {
    for (const staffSpace of ZOOMS) {
      const step = staffSpace * STEP_SPACES;
      expect(stepsFor(-step * 2, staffSpace)).toBe(2);
      expect(stepsFor(step * 3, staffSpace)).toBe(-3);
    }
  });

  it("counts up the page as up the staff", () => {
    expect(stepsFor(-10, 10)).toBeGreaterThan(0);
    expect(stepsFor(10, 10)).toBeLessThan(0);
  });

  it("snaps to the nearest step, so a wavering hand writes one note", () => {
    expect(stepsFor(-4, 10)).toBe(1);
    expect(stepsFor(-6, 10)).toBe(1);
    expect(stepsFor(-2, 10)).toBe(0);
  });

  /*
   * The gesture is relative to the note it started on, so the clef never
   * enters the arithmetic. A treble note and a bass note dragged the same
   * distance move the same number of steps — which is what makes the geometry
   * safe to do in the frontend at all.
   */
  it("means the same thing on a treble note and on a bass one", () => {
    const steps = stepsFor(-10, 10);
    expect(shiftStep("g4", steps)).toBe("b4");
    expect(shiftStep("g2", steps)).toBe("b2");
  });
});

describe("spelling a step away", () => {
  it("turns the octave over at C, not at A", () => {
    expect(shiftStep("b4", 1)).toBe("c5");
    expect(shiftStep("c5", -1)).toBe("b4");
  });

  it("carries the accidental rather than deciding one", () => {
    expect(shiftStep("g#4", 1)).toBe("a#4");
    expect(shiftStep("eb5", -2)).toBe("cb5");
  });

  it("leaves a note that did not move exactly as it was", () => {
    expect(shiftStep("g#4", 0)).toBe("g#4");
  });

  it("writes nothing for something that is not a written pitch", () => {
    expect(shiftStep("r", 1)).toBeNull();
    expect(shiftStep("g", 1)).toBeNull();
  });
});

describe("the accidental ladder", () => {
  it("walks flat to sharp, and stops at both ends", () => {
    expect(shiftAccidental("g4", 1)).toBe("g#4");
    expect(shiftAccidental("g#4", 1)).toBe("g##4");
    expect(shiftAccidental("g##4", 1)).toBe("g##4");
    expect(shiftAccidental("gbb4", -1)).toBe("gbb4");
  });

  it("treats an explicit natural as the bare letter", () => {
    expect(shiftAccidental("gn4", 1)).toBe("g#4");
  });
});

describe("the duration ladder", () => {
  it("is one rung per drag distance, at every zoom", () => {
    for (const staffSpace of ZOOMS) {
      expect(rungsFor(staffSpace * RUNG_SPACES, staffSpace)).toBe(1);
      expect(rungsFor(-staffSpace * RUNG_SPACES * 2, staffSpace)).toBe(-2);
    }
  });

  it("has the dotted values between the plain ones", () => {
    expect(renotate("1/4", 1)).toBe("3/8");
    expect(renotate("3/8", 1)).toBe("1/2");
    expect(renotate("1/2", -2)).toBe("1/4");
  });

  it("stops at the ends rather than inventing a duration", () => {
    expect(renotate(LADDER[0] ?? "", 1)).toBe(LADDER[0]);
    expect(renotate(LADDER[LADDER.length - 1] ?? "", -1)).toBe(
      LADDER[LADDER.length - 1],
    );
  });

  it("writes nothing for a duration that is not on the ladder", () => {
    expect(renotate("1/3", 1)).toBeNull();
    expect(renotate("5/8", -1)).toBeNull();
  });
});

/** A press on a quarter-note G, at the origin, with a four-unit threshold. */
function press(over: Partial<Press> = {}): Press {
  return {
    event: "event-0",
    pitch: "g4",
    duration: "1/4",
    atEdge: false,
    alt: false,
    x: 0,
    y: 0,
    threshold: 4,
    ...over,
  };
}

describe("the gesture", () => {
  it("is still a click until the pointer travels", () => {
    const gesture = new Gesture();
    gesture.begin(press());
    gesture.move(1, -2, 10);
    expect(gesture.axis).toBe("none");
    expect(gesture.candidate).toBeNull();
    expect(gesture.release()).toBeNull();
  });

  it("is a respelling when it goes up", () => {
    const gesture = new Gesture();
    gesture.begin(press());
    gesture.move(0, -10, 10);
    expect(gesture.axis).toBe("pitch");
    expect(gesture.candidate).toMatchObject({ kind: "pitch", value: "b4" });
  });

  it("is an accidental when ⌥ was held", () => {
    const gesture = new Gesture();
    gesture.begin(press({ alt: true }));
    gesture.move(0, -5, 10);
    expect(gesture.axis).toBe("accidental");
    expect(gesture.candidate).toMatchObject({ value: "g#4" });
  });

  it("is a range selection when it goes sideways", () => {
    const gesture = new Gesture();
    gesture.begin(press());
    gesture.move(40, -2, 10);
    expect(gesture.axis).toBe("range");
    expect(gesture.candidate).toBeNull();
  });

  it("is a renotation when it started on the handle, whichever way it goes", () => {
    const gesture = new Gesture();
    gesture.begin(press({ atEdge: true }));
    gesture.move(15, 4, 10);
    expect(gesture.axis).toBe("duration");
    expect(gesture.candidate).toMatchObject({ kind: "duration", value: "3/8" });
  });

  it("keeps the axis it resolved to, however the pointer wanders after", () => {
    const gesture = new Gesture();
    gesture.begin(press());
    gesture.move(0, -10, 10);
    gesture.move(60, -10, 10);
    expect(gesture.axis).toBe("pitch");
    expect(gesture.candidate).toMatchObject({ value: "b4" });
  });

  it("has nothing to write when it comes back where it started", () => {
    const gesture = new Gesture();
    gesture.begin(press());
    gesture.move(0, -10, 10);
    gesture.move(0, 0, 10);
    expect(gesture.release()).toBeNull();
  });

  it("cannot respell a rest, which has no step to drag", () => {
    const gesture = new Gesture();
    gesture.begin(press({ pitch: null }));
    gesture.move(0, -10, 10);
    expect(gesture.axis).toBe("range");
  });

  it("forgets everything on Esc", () => {
    const gesture = new Gesture();
    gesture.begin(press());
    gesture.move(0, -10, 10);
    gesture.cancel();
    expect(gesture.axis).toBe("none");
    expect(gesture.candidate).toBeNull();
    expect(gesture.event).toBeNull();
    gesture.move(0, -20, 10);
    expect(gesture.candidate).toBeNull();
  });
});
