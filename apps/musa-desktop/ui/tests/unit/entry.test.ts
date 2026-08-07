/**
 * The keyboard's writing half: what the entry state spells, and what a
 * keystroke means against a selection.
 *
 * Both are pure — `NoteEntry` holds what the *next* note would be, `stroke`
 * turns a keystroke into an `EditScore` command — so both are testable
 * without a browser, and neither is allowed to compute anything musical
 * (`03-interaction.md` §7). What is asserted here is the spelling and the
 * choice between changing a selected event and writing a new one; that those
 * commands mean what they say is asserted by `musa-project`'s editing laws.
 */

import { describe, expect, it } from "vitest";

import { stroke, anchorFor, played } from "../../src/lib/state/compose";
import { NoteEntry, octaveOf, spellDuration, spellPitch } from "../../src/lib/state/entry.svelte";
import { fixture } from "../../src/lib/state/fixtures";
import { Workspace } from "../../src/lib/state/selection.svelte";
import type { ProjectSnapshot } from "../../src/lib/state/snapshot";

const snapshot = fixture("glass-mountain").snapshot as ProjectSnapshot;

function workspace(): Workspace {
  return new Workspace(() => snapshot);
}

function key(k: string, modifiers: Partial<KeyboardEvent> = {}): KeyboardEvent {
  return {
    key: k,
    metaKey: false,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    ...modifiers,
  } as KeyboardEvent;
}

describe("spelling", () => {
  it("is the language's, so the source says what the composer typed", () => {
    expect(spellDuration(1, false)).toBe("1");
    expect(spellDuration(4, false)).toBe("1/4");
    expect(spellDuration(16, false)).toBe("1/16");
    // A dot is arithmetic on the notation: a dotted eighth is three sixteenths.
    expect(spellDuration(8, true)).toBe("3/16");
    expect(spellDuration(4, true)).toBe("3/8");
    expect(spellPitch("g", "s", 4)).toBe("gs4");
    expect(spellPitch("b", "f", 3)).toBe("bf3");
    expect(spellPitch("c", "", 5)).toBe("c5");
  });

  it("reads an octave back out of the core's own spelling", () => {
    expect(octaveOf("E5")).toBe(5);
    expect(octaveOf("Gs4")).toBe(4);
    expect(octaveOf(null)).toBeNull();
    expect(octaveOf("rest")).toBeNull();
  });
});

describe("entry state", () => {
  it("keeps the duration across notes and forgets the accidental on leaving", () => {
    const entry = new NoteEntry();
    entry.set(true);
    expect(entry.chooseDuration("8")).toBe(true);
    entry.shiftAccidental(1);
    expect(entry.pitch("f")).toBe("fs4");
    entry.set(false);
    entry.set(true);
    // The duration is a working setting; the accidental was about one note.
    expect(entry.duration).toBe("1/8");
    expect(entry.pitch("f")).toBe("f4");
  });

  it("binds every duration key and no others", () => {
    const entry = new NoteEntry();
    const bound: Record<string, string> = {};
    for (const digit of "0123456789") {
      if (entry.chooseDuration(digit)) bound[digit] = entry.duration;
    }
    expect(bound).toEqual({
      "1": "1",
      "2": "1/2",
      "3": "1/32",
      "4": "1/4",
      "6": "1/16",
      "8": "1/8",
    });
  });

  it("climbs the accidental ladder in one direction and back in the other", () => {
    const entry = new NoteEntry();
    entry.shiftAccidental(1);
    expect(entry.accidental).toBe("s");
    // Sharp is the top: pressing again does not invent a double sharp.
    entry.shiftAccidental(1);
    expect(entry.accidental).toBe("s");
    entry.shiftAccidental(-1);
    expect(entry.accidental).toBe("");
    entry.shiftAccidental(-1);
    expect(entry.accidental).toBe("f");
    entry.shiftAccidental(-1);
    expect(entry.accidental).toBe("f");
  });

  it("stops the octave at the ends of a keyboard", () => {
    const entry = new NoteEntry();
    for (let step = 0; step < 12; step += 1) entry.shiftOctave(1);
    expect(entry.octave).toBe(8);
    for (let step = 0; step < 20; step += 1) entry.shiftOctave(-1);
    expect(entry.octave).toBe(0);
  });
});

describe("a keystroke against a selection", () => {
  it("respells the selected note, in the octave it is already in", () => {
    const space = workspace();
    const entry = new NoteEntry();
    space.select("event-0"); // E5.
    const result = stroke(key("f"), entry, space);
    expect(result).toEqual({
      kind: "edit",
      at: null,
      edit: {
        kind: "changePitch",
        event: "event-0",
        pitch: "f5",
        mode: "editDefinition",
      },
    });
    // The entry octave is untouched: respelling is not moving.
    expect(entry.octave).toBe(4);
  });

  it("renotates the selected note when a duration key is pressed", () => {
    const space = workspace();
    const entry = new NoteEntry();
    space.select("event-2");
    expect(stroke(key("8"), entry, space)).toEqual({
      kind: "edit",
      at: null,
      edit: {
        kind: "changeDuration",
        event: "event-2",
        duration: "1/8",
        mode: "editDefinition",
      },
    });
    // And the dot renotates it again, rather than waiting for the next note.
    expect(stroke(key("."), entry, space)).toMatchObject({
      edit: { duration: "3/16" },
    });
  });

  it("writes after the selection when the key is a pitch and nothing is selected", () => {
    const space = workspace();
    const entry = new NoteEntry();
    space.selection = {
      kind: "caret",
      part: "violin",
      voice: "lead",
      before: "end",
    };
    expect(stroke(key("a"), entry, space)).toEqual({
      kind: "edit",
      at: { kind: "endOfVoice", part: "violin", voice: "lead" },
      edit: {
        kind: "insertNote",
        at: { kind: "endOfVoice", part: "violin", voice: "lead" },
        note: { kind: "note", pitch: "a4", duration: "1/4" },
      },
    });
  });

  it("writes a rest on `r`, because Space plays", () => {
    const space = workspace();
    const entry = new NoteEntry();
    space.selection = {
      kind: "caret",
      part: "violin",
      voice: "lead",
      before: "end",
    };
    expect(stroke(key("r"), entry, space)).toMatchObject({
      edit: { note: { kind: "rest", duration: "1/4" } },
    });
  });

  it("treats modified arrows as settings and every other key as not its own", () => {
    const space = workspace();
    const entry = new NoteEntry();
    expect(stroke(key("ArrowUp", { metaKey: true }), entry, space)).toEqual({
      kind: "settings",
    });
    expect(entry.octave).toBe(5);
    expect(stroke(key("ArrowDown", { shiftKey: true }), entry, space)).toEqual({
      kind: "settings",
    });
    expect(entry.accidental).toBe("f");
    // Unmodified arrows stay navigation, and so does everything unbound.
    expect(stroke(key("ArrowLeft"), entry, space)).toEqual({ kind: "pass" });
    expect(stroke(key("z"), entry, space)).toEqual({ kind: "pass" });
    expect(stroke(key("ArrowRight", { altKey: true }), entry, space)).toEqual({
      kind: "pass",
    });
  });
});

describe("where a new note goes", () => {
  it("is after the selection, and the end of the voice with nothing to go after", () => {
    const space = workspace();
    space.select("event-3");
    expect(anchorFor(space)).toEqual({ kind: "after", event: "event-3" });

    space.selection = {
      kind: "caret",
      part: "strings",
      voice: "bass",
      before: "event-f",
    };
    expect(anchorFor(space)).toEqual({ kind: "before", event: "event-f" });

    space.selection = { kind: "none" };
    // With no selection at all it falls back to the voice the inspector is
    // describing, which is the first — never to nowhere.
    expect(anchorFor(space)).toEqual({
      kind: "endOfVoice",
      part: space.focused?.part,
      voice: space.focused?.voice,
    });
  });
});

describe("a note played in on a MIDI keyboard", () => {
  it("is written where the caret is, with the duration entry is set to", () => {
    const space = workspace();
    space.select("event-3");
    const entry = new NoteEntry();
    entry.chooseDuration("8");

    expect(played(["ef4"], entry, space)).toEqual({
      kind: "edit",
      at: { kind: "after", event: "event-3" },
      edit: {
        kind: "insertNote",
        at: { kind: "after", event: "event-3" },
        note: { kind: "note", pitch: "ef4", duration: "1/8" },
      },
    });
  });

  it("is a chord when several keys were held together", () => {
    const space = workspace();
    space.select("event-3");
    const asked = played(["c4", "e4", "g4"], new NoteEntry(), space);

    expect(asked.kind === "edit" && asked.edit).toEqual({
      kind: "insertNote",
      at: { kind: "after", event: "event-3" },
      note: { kind: "chord", pitches: ["c4", "e4", "g4"], duration: "1/4" },
    });
  });

  it("spells nothing itself: the pitches are the core's, accidental and all", () => {
    const space = workspace();
    space.select("event-3");
    const entry = new NoteEntry();
    // Entry's own accidental and octave belong to the letter keys. A played
    // note already knows what it is, and must come through untouched.
    entry.shiftAccidental(1);
    entry.shiftOctave(-2);
    const asked = played(["bf2"], entry, space);

    expect(asked.kind === "edit" && asked.edit.kind === "insertNote" && asked.edit.note).toEqual({
      kind: "note",
      pitch: "bf2",
      duration: "1/4",
    });
  });

  it("writes nothing when the keyboard sent nothing", () => {
    expect(played([], new NoteEntry(), workspace()).kind).toBe("pass");
  });
});
