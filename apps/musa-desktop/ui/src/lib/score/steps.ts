/**
 * What a pointer gesture on the page means, in the score's own units.
 *
 * The one piece of geometry prompt 53 adds, and it is deliberately *relative*:
 * every gesture is measured from the note it started on, whose spelling and
 * duration the core already published. A diatonic step is half a staff space
 * on every clef and in every key, so a drag needs no clef, no staff-line
 * detection, and no idea of what pitch sits where — only how far it moved.
 *
 * Nothing musical is computed here in the sense `03-interaction.md` §7 means
 * it. What these functions do is spell: letter-and-octave arithmetic on the
 * text the source already contains, the same kind `state/compose.ts` does for
 * the keyboard. The core decides what the spelling means.
 */

/** A diatonic step is half a staff space — a line to the space beside it. */
export const STEP_SPACES = 0.5;

/** How far the pointer travels for one rung of the duration ladder. */
export const RUNG_SPACES = 1.5;

/** How far a press has to move before it stops being a click. */
export const THRESHOLD_PX = 4;

/** How wide the duration handle is at the note's right edge, in staff spaces. */
export const HANDLE_SPACES = 1;

/**
 * How many diatonic steps a vertical travel is, snapped.
 *
 * Up the page is up the staff, so the sign flips: SVG counts y downward and
 * music counts pitch upward.
 */
export function stepsFor(dy: number, staffSpace: number): number {
  if (staffSpace <= 0) return 0;
  return -Math.round(dy / (staffSpace * STEP_SPACES));
}

/** How many rungs a horizontal travel is, snapped. Right is longer. */
export function rungsFor(dx: number, staffSpace: number): number {
  if (staffSpace <= 0) return 0;
  return Math.round(dx / (staffSpace * RUNG_SPACES));
}

/** The seven letters, in the order the staff puts them. */
const LETTERS = "cdefgab";

/** The accidental suffixes the language writes, longest first. */
const ACCIDENTALS = ["ss", "ff", "s", "f", "n"] as const;

/** A written pitch, taken apart the way the language spells it: `gs4`. */
interface Written {
  letter: string;
  accidental: string;
  octave: number;
}

/** Take a written pitch apart, or nothing if it is not one. */
function readPitch(spelling: string): Written | null {
  const letter = spelling.slice(0, 1);
  if (letter.length === 0 || !LETTERS.includes(letter)) return null;
  let rest = spelling.slice(1);
  let accidental = "";
  for (const mark of ACCIDENTALS) {
    if (rest.startsWith(mark)) {
      accidental = mark;
      rest = rest.slice(mark.length);
      break;
    }
  }
  if (!/^-?\d+$/.test(rest)) return null;
  return { letter, accidental, octave: Number(rest) };
}

function writePitch(written: Written): string {
  return `${written.letter}${written.accidental}${written.octave}`;
}

/**
 * The same note `steps` diatonic steps away.
 *
 * The accidental is carried through untouched — that is the whole answer to
 * roadmap §14.5's accidental ambiguity. Moving `gs4` up one step writes `as4`,
 * because the gesture said *a step*, and what a step means in this key is the
 * key signature's business, not the pointer's.
 */
export function shiftStep(spelling: string, steps: number): string | null {
  const written = readPitch(spelling);
  if (!written || steps === 0) return written ? spelling : null;
  const at = LETTERS.indexOf(written.letter) + steps;
  // C is the bottom of the octave, so the octave turns over at C, not at A.
  const octaves = Math.floor(at / LETTERS.length);
  const letter = LETTERS[at - octaves * LETTERS.length];
  if (letter === undefined) return null;
  return writePitch({ ...written, letter, octave: written.octave + octaves });
}

/** Flat to sharp, in the order `⇧↑` already walks them. */
const CYCLE: readonly string[] = ["ff", "f", "", "s", "ss"];

/** The same note with its accidental moved along the ladder, step untouched. */
export function shiftAccidental(spelling: string, by: number): string | null {
  const written = readPitch(spelling);
  if (!written) return null;
  // `n` is an explicit natural, which sits where the bare letter sits.
  const from = CYCLE.indexOf(written.accidental === "n" ? "" : written.accidental);
  const at = Math.min(Math.max(from + by, 0), CYCLE.length - 1);
  return writePitch({ ...written, accidental: CYCLE[at] ?? "" });
}

/**
 * The duration ladder, longest first: the six the number keys spell, each
 * with its dotted form, and nothing between them.
 */
export const LADDER: readonly string[] = [
  "3/2",
  "1",
  "3/4",
  "1/2",
  "3/8",
  "1/4",
  "3/16",
  "1/8",
  "3/32",
  "1/16",
  "3/64",
  "1/32",
];

/** Where a written duration sits on the ladder, or -1 if it is not on it. */
export function rungOf(spelling: string): number {
  return LADDER.indexOf(spelling);
}

/**
 * The duration `longerBy` rungs along from this one, where a positive count is
 * longer — the direction the pointer went.
 *
 * Null when the note's own duration is not on the ladder: a tuplet member, or
 * a value only a tie can write. A gesture that cannot say what it would write
 * does not write, which is how §14.5's tie and tuplet ambiguities stay
 * answered.
 */
export function renotate(spelling: string, longerBy: number): string | null {
  const at = rungOf(spelling);
  if (at < 0) return null;
  return LADDER[Math.min(Math.max(at - longerBy, 0), LADDER.length - 1)] ?? null;
}
