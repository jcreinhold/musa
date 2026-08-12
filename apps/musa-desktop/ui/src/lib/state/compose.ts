/**
 * What a keystroke means while note entry is on.
 *
 * One pure function, so the entry map can be read in one place and tested
 * without a browser. It decides between *changing* the selected event and
 * *writing* a new one — `03-interaction.md` §1's distinction between an event
 * selection and a caret — and spells the result as an `EditScore` command.
 * Nothing musical is computed here: pitches and durations are spelled the way
 * the language spells them and the core decides what they mean.
 */

import type { EditDto } from "../session/generated/EditDto";
import type { InsertAtDto } from "../session/generated/InsertAtDto";
import { NoteEntry, PITCH_KEYS, octaveOf, spellDuration } from "./entry.svelte";
import type { Workspace } from "./selection.svelte";

/** What a keystroke did. */
export type Stroke =
  /** Not an entry key; the command map may still want it. */
  | { kind: "pass" }
  /** It changed what the *next* note would be, and nothing else. */
  | { kind: "settings" }
  /** It asks for this edit. */
  | { kind: "edit"; edit: EditDto; at: InsertAtDto | null };

const PASS: Stroke = { kind: "pass" };
const SETTINGS: Stroke = { kind: "settings" };

/**
 * Where a new statement goes: the caret if there is one, else after the
 * selection, else the end of the voice the composer is working in.
 */
export function anchorFor(workspace: Workspace): InsertAtDto | null {
  const selection = workspace.selection;
  if (selection.kind === "caret") {
    return selection.before === "end"
      ? { kind: "endOfVoice", part: selection.part, voice: selection.voice }
      : { kind: "before", event: selection.before };
  }
  const ids = workspace.selected;
  const last = ids[ids.length - 1];
  if (last !== undefined) return { kind: "after", event: last };
  const active = workspace.active;
  return active
    ? { kind: "endOfVoice", part: active.part, voice: active.voice }
    : null;
}

/** The event the shortcuts change, as opposed to write after. */
function selectedEvent(workspace: Workspace) {
  return workspace.selection.kind === "event" ? workspace.focused : undefined;
}

/** A write, or nothing at all when there is no voice to write into. */
function insert(
  at: InsertAtDto | null,
  note: (EditDto & { kind: "insertNote" })["note"],
): Stroke {
  return at === null
    ? PASS
    : { kind: "edit", at, edit: { kind: "insertNote", at, note } };
}

/**
 * What notes played in on a MIDI keyboard write.
 *
 * The keyboard supplies the pitches — already spelled by the core, which is
 * the only side that knows the key signature — and entry supplies the
 * duration, because a keyboard cannot say how long a note is *notated* for
 * (roadmap §14.5: pitch from the keyboard, duration from the number keys).
 * One key is a note and several held together are a chord, which is the
 * grouping the core made before this ever saw them.
 */
export function played(
  pitches: string[],
  entry: NoteEntry,
  workspace: Workspace,
): Stroke {
  const at = anchorFor(workspace);
  const [first, ...rest] = pitches;
  if (first === undefined) return PASS;
  return insert(
    at,
    rest.length === 0
      ? { kind: "note", pitch: first, duration: entry.duration }
      : { kind: "chord", pitches, duration: entry.duration },
  );
}

/**
 * Read a keystroke as an entry command.
 *
 * Mutates `entry` for the keys that only change what comes next — the
 * duration, the dot, the octave, the accidental — because those are settings,
 * not edits, and a composer expects them to persist across notes.
 */
export function stroke(
  event: KeyboardEvent,
  entry: NoteEntry,
  workspace: Workspace,
): Stroke {
  const key = event.key;
  const selected = selectedEvent(workspace);

  // Modified arrows adjust what the letters will spell. They are the only
  // entry keys that take a modifier, because the unmodified arrows are
  // navigation and stay navigation.
  if (event.metaKey || event.ctrlKey) {
    if (key !== "ArrowUp" && key !== "ArrowDown") return PASS;
    entry.shiftOctave(key === "ArrowUp" ? 1 : -1);
    return SETTINGS;
  }
  if (event.shiftKey) {
    if (key !== "ArrowUp" && key !== "ArrowDown") return PASS;
    entry.shiftAccidental(key === "ArrowUp" ? 1 : -1);
    return SETTINGS;
  }
  if (event.altKey) return PASS;

  // A number picks the duration; with an event selected it renotates that
  // event as well, which is `03-interaction.md` §1's rule that the same
  // shortcut changes a selection rather than writing after it.
  if (entry.chooseDuration(key) || key === ".") {
    if (key === ".") entry.toggleDot();
    if (!selected) return SETTINGS;
    return {
      kind: "edit",
      at: null,
      edit: {
        kind: "changeDuration",
        event: selected.id,
        duration: spellDuration(entry.denominator, entry.dotted),
        mode: "editDefinition",
      },
    };
  }

  if (PITCH_KEYS.includes(key)) {
    // Respelling a selected note keeps it in its own octave: a composer
    // fixing a wrong letter does not also want it moved an octave.
    if (selected && selected.kind !== "rest") {
      return {
        kind: "edit",
        at: null,
        edit: {
          kind: "changePitch",
          event: selected.id,
          pitch: entry.pitch(key, octaveOf(selected.pitch) ?? entry.octave),
          mode: "editDefinition",
        },
      };
    }
    return insert(anchorFor(workspace), {
      kind: "note",
      pitch: entry.pitch(key),
      duration: entry.duration,
    });
  }

  // `r` rather than `Space`, which plays: a transport key that stopped
  // playing inside a mode would be worse than a second letter to learn.
  if (key === "r") {
    return insert(anchorFor(workspace), {
      kind: "rest",
      duration: entry.duration,
    });
  }

  return PASS;
}
