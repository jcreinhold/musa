/**
 * What a group command is, before the core sees it.
 *
 * The rule these hold to is the one the whole feature rests on: the interface
 * decides *which command* the composer asked for and nothing else. It never
 * computes a pitch, a duration, or a consequence — so what there is to test
 * here is that five different musical questions stay five.
 */

import { describe, expect, it } from "vitest";

import { durationIntent, respellIntent, transposeIntent } from "../../src/lib/state/group";

describe("the duration keys", () => {
  it("write the value the key names on every selected event", () => {
    expect(durationIntent("4")).toEqual({ kind: "setEachDuration", duration: "1/4" });
    expect(durationIntent("1")).toEqual({ kind: "setEachDuration", duration: "1" });
    expect(durationIntent("6")).toEqual({ kind: "setEachDuration", duration: "1/16" });
  });

  it("set rather than scale, which is a different question", () => {
    // `4` says "these are quarter notes". Making the passage half as long is
    // a command the number keys deliberately do not have (OMT ch. 009).
    const intent = durationIntent("4");
    expect(intent?.kind).toBe("setEachDuration");
  });

  it("are only the six duration keys", () => {
    expect(durationIntent("5")).toBeNull();
    expect(durationIntent("a")).toBeNull();
    expect(durationIntent("")).toBeNull();
  });
});

describe("the vertical keys", () => {
  it("move the notehead and the sign with two different commands", () => {
    expect(respellIntent(1, false)).toEqual({ kind: "moveDiatonically", steps: 1 });
    expect(respellIntent(-1, true)).toEqual({ kind: "shiftAccidentals", steps: -1 });
  });
});

describe("the interval field", () => {
  it("passes what was written through, trimmed", () => {
    expect(transposeIntent("  up P5 ")).toEqual({ kind: "transposeBy", interval: "up P5" });
  });

  it("asks nothing of the core for an empty field", () => {
    expect(transposeIntent("   ")).toBeNull();
  });

  it("leaves judging an interval to the core", () => {
    // `Q9` is not an interval, and this is not the side that knows that: a
    // refusal the composer can read beats a field that silently does nothing.
    expect(transposeIntent("Q9")).toEqual({ kind: "transposeBy", interval: "Q9" });
  });
});
