/**
 * The printed page's front matter, read the way the pointer reads it
 *.
 *
 * The MEI backend writes the page head itself so that every printed line
 * carries an `xml:id`, which is what makes clicking the title the same
 * machinery as clicking a notehead. These tests hold the two halves of that
 * contract together: the ids `musa-render` writes and the fields this module
 * maps them to are one list, and the fixture is the real MEI the core
 * produced.
 */

import { describe, expect, it } from "vitest";

import { fixture } from "../../src/lib/state/fixtures";
import { fieldForId } from "../../src/lib/score/front-matter";
import type { ProjectSnapshot } from "../../src/lib/state/snapshot";

const snapshot = fixture("glass-mountain").snapshot as ProjectSnapshot;

describe("the id a printed line carries", () => {
  it("names the field a click on it would edit", () => {
    expect(fieldForId("front-title")).toBe("title");
    expect(fieldForId("front-subtitle")).toBe("subtitle");
    expect(fieldForId("front-composer")).toBe("composer");
    expect(fieldForId("front-arranger")).toBe("arranger");
    expect(fieldForId("front-copyright")).toBe("copyright");
  });

  it("is nothing for anything else on the page", () => {
    expect(fieldForId("event-0")).toBeNull();
    expect(fieldForId("layer-1-1")).toBeNull();
    expect(fieldForId("front-unheard-of")).toBeNull();
    expect(fieldForId(null)).toBeNull();
    expect(fieldForId(undefined)).toBeNull();
  });
});

describe("the engraved page", () => {
  it("names every line the piece actually states", () => {
    const mei = snapshot.mei ?? "";
    // Glass Mountain names four of the five; it has no arranger, and an
    // unnamed role prints nothing rather than an empty line.
    for (const id of ["front-title", "front-subtitle", "front-composer", "front-copyright"]) {
      expect(mei).toContain(`xml:id="${id}"`);
    }
    expect(mei).not.toContain("front-arranger");
  });
});

describe("what the piece says about itself", () => {
  it("lists every field, filled in or not, in the order they are written", () => {
    expect(snapshot.score?.header.map((fact) => fact.field)).toEqual([
      "title",
      "subtitle",
      "composer",
      "arranger",
      "copyright",
      "tempo",
      "meter",
      "key",
    ]);
  });

  it("spells the values the way the source spells them", () => {
    const said = (field: string) =>
      snapshot.score?.header.find((fact) => fact.field === field)?.value;
    // Not `♩ = 72` and not `A minor`: a field that read one dialect and wrote
    // another would be a second language to keep working.
    expect(said("tempo")).toBe("quarter = 72");
    expect(said("key")).toBe("a minor");
    expect(said("meter")).toBe("4/4");
    expect(said("title")).toBe("Glass Mountain");
    // The one it does not state is present and empty, which is what puts an
    // editable row on screen for it.
    expect(said("arranger")).toBeNull();
  });
});
