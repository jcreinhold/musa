/**
 * How a proposal is laid out, before anything is decided about it.
 *
 * The rule these hold to is the one Review rests on: the interface arranges
 * what the project composed and computes no notation. So what there is to
 * test is arrangement — which line a note is drawn on, where along the take
 * it sits, which questions stand over it, and what the margin says about
 * keeping it.
 */

import { describe, expect, it } from "vitest";

import {
  WHOLE_TICKS,
  barTicks,
  barlines,
  destination,
  extent,
  keepable,
  marked,
  marksOver,
  nextMark,
  rows,
  taps,
} from "../../src/lib/state/review";
import type { ReviewAmbiguityDto } from "../../src/lib/session/generated/ReviewAmbiguityDto";
import type { ReviewFactsDto } from "../../src/lib/session/generated/ReviewFactsDto";
import type { ReviewNoteDto } from "../../src/lib/session/generated/ReviewNoteDto";

function note(partial: Partial<ReviewNoteDto> = {}): ReviewNoteDto {
  return {
    pitch: "c4",
    voice: 1,
    onsetTicks: 0,
    endTicks: 24,
    pedalExtended: false,
    grace: false,
    name: "c4, a quarter, line 1, bar 1, beat 1",
    ...partial,
  };
}

function mark(partial: Partial<ReviewAmbiguityDto> = {}): ReviewAmbiguityDto {
  return {
    id: "phase-1",
    kind: "phase",
    explanation: "Two readings of the beat survived.",
    notes: [0],
    choices: [],
    ...partial,
  };
}

function facts(partial: Partial<ReviewFactsDto> = {}): ReviewFactsDto {
  return {
    takeName: "piano@3",
    revision: 3n,
    current: true,
    meter: "4/4",
    policy: "straight",
    notes: [note()],
    voiceCount: 1,
    ambiguities: [],
    losses: [],
    history: [],
    audition: "played",
    source: "bar { c4 }",
    sealed: false,
    changed: [],
    ...partial,
  };
}

describe("how long the take is", () => {
  it("runs to the last written end", () => {
    const take = facts({
      notes: [note({ onsetTicks: 0, endTicks: 96 }), note({ onsetTicks: 96, endTicks: 48 })],
    });
    expect(extent(take)).toBe(144);
  });

  it("is never shorter than a whole note, so one note does not fill the page", () => {
    expect(extent(facts({ notes: [note({ onsetTicks: 0, endTicks: 12 })] }))).toBe(WHOLE_TICKS);
  });
});

describe("the staff rows", () => {
  it("are one per line the proposal actually writes, lowest first", () => {
    const take = facts({
      voiceCount: 2,
      notes: [note({ voice: 2 }), note({ voice: 1 }), note({ voice: 2, onsetTicks: 24 })],
    });
    expect(rows(take).map((row) => row.voice)).toEqual([1, 2]);
    expect(rows(take)[1]?.notes).toHaveLength(2);
  });

  it("do not draw a line nobody played, even when the count claims one", () => {
    // `voiceCount` says how many lines the proposal *may* use. An empty row
    // would claim a rest the take does not contain.
    const take = facts({ voiceCount: 4, notes: [note({ voice: 1 })] });
    expect(rows(take)).toHaveLength(1);
  });

  it("place each note by the ticks the project stated, never by a duration of their own", () => {
    const take = facts({ notes: [note({ onsetTicks: 48, endTicks: 48 })] });
    const placed = rows(take)[0]?.notes[0];
    expect(placed?.start).toBeCloseTo(0.5);
    expect(placed?.width).toBeCloseTo(0.5);
    expect(placed?.index).toBe(0);
  });

  it("give a grace note a width, so it is reachable rather than invisible", () => {
    const take = facts({ notes: [note({ onsetTicks: 0, endTicks: 0, grace: true })] });
    expect(rows(take)[0]?.notes[0]?.width).toBeGreaterThan(0);
  });
});

describe("the bar lines", () => {
  it("are one bar of the destination meter apart", () => {
    const take = facts({ notes: [note({ onsetTicks: 0, endTicks: 288 })] });
    expect(barlines(take, barTicks("4/4"))).toEqual([96 / 288, 192 / 288]);
  });

  it("are none when the meter is not one", () => {
    expect(barTicks("free")).toBe(0);
    expect(barlines(facts(), 0)).toEqual([]);
  });

  it("measure an asymmetric meter as written", () => {
    expect(barTicks("7/8")).toBe(84);
    expect(barTicks("3/4")).toBe(72);
    expect(barTicks("6/8")).toBe(72);
  });

  it("refuse a unit the grid cannot divide, rather than rounding one", () => {
    expect(barTicks("4/5")).toBe(0);
    expect(barTicks("0/4")).toBe(0);
  });
});

describe("the marks", () => {
  it("stand over the notes the project named and no others", () => {
    const take = facts({
      notes: [note(), note({ onsetTicks: 24 })],
      ambiguities: [mark({ id: "a", notes: [1] })],
    });
    expect(marksOver(0, take)).toEqual([]);
    expect(marksOver(1, take).map((each) => each.id)).toEqual(["a"]);
    expect([...marked(take)]).toEqual([1]);
  });

  it("walk left to right, which is the order the project lists them in", () => {
    const take = facts({
      ambiguities: [mark({ id: "a", notes: [0] }), mark({ id: "b", notes: [1] })],
    });
    expect(nextMark(take, null, 1)).toBe("a");
    expect(nextMark(take, "a", 1)).toBe("b");
    expect(nextMark(take, "b", 1)).toBe("a");
    expect(nextMark(take, null, -1)).toBe("b");
    expect(nextMark(take, "a", -1)).toBe("b");
  });

  it("have nothing to walk to when the reading asks nothing", () => {
    expect(nextMark(facts(), null, 1)).toBeNull();
  });
});

describe("what the top margin says", () => {
  it("names the phrase in the project's own words", () => {
    expect(destination(facts())).toBe("1 note in one line, in 4/4");
    expect(destination(facts({ notes: [note(), note()], voiceCount: 2 }))).toBe("2 notes in 2 lines, in 4/4");
  });

  it("keeps a reading that writes exactly and is still the session's", () => {
    expect(keepable(facts())).toBe(true);
  });

  it("cannot keep a reading with no exact written form", () => {
    expect(keepable(facts({ source: null }))).toBe(false);
  });

  it("cannot keep a reading twice, or one the source has moved past", () => {
    expect(keepable(facts({ sealed: true }))).toBe(false);
    expect(keepable(facts({ current: false }))).toBe(false);
  });
});

describe("tapping a pulse", () => {
  it("sends intervals from the take's start, in microseconds", () => {
    expect(taps([1000, 1500, 2000], 1000, 0)).toEqual({
      beatsMicros: [0n, 500_000n, 1_000_000n],
      downbeat: 0,
    });
  });

  it("never sends a tap before the take began", () => {
    expect(taps([900], 1000, null).beatsMicros).toEqual([0n]);
  });
});
