/**
 * Note entry: the state behind the keyboard's writing half
 * (`03-interaction.md` §3).
 *
 * Entry is a mode, and deliberately so. The navigation map already owns the
 * unmodified letters — `F` follows, `L` loops, `O` is the lens — so a bare
 * `f` cannot also be the note F without one of the two silently losing. Every
 * notation editor a musician has used answers this the same way: you go into
 * entry, and while you are in it the letters are pitches. The mode is never
 * invisible; the duration glyph sits in the top margin the whole time it is
 * on.
 *
 * This class holds only what the *next* note would be. It issues nothing and
 * knows nothing about the score: the pitch and duration it spells go into an
 * `EditScore` command, and the core decides what that means.
 */

import { durationGlyph } from "../ui/glyphs";

/**
 * The number keys, as `03-interaction.md`'s entry table binds them: the key
 * is the denominator where the digit allows it, and the three that do not fit
 * on one key take the nearest free digit.
 */
export const DURATION_KEYS: ReadonlyMap<string, number> = new Map([
  ["1", 1],
  ["2", 2],
  ["4", 4],
  ["8", 8],
  ["6", 16],
  ["3", 32],
]);

/** The letters that are pitches while entry is on. */
export const PITCH_KEYS = "cdefgab";

/** The accidentals entry can spell, in the language's own suffixes. */
export type Accidental = "" | "#" | "b";

/** How far the octave key can go before it stops being a piano. */
const OCTAVE_RANGE = { low: 0, high: 8 } as const;

/**
 * How the language spells a duration.
 *
 * Dotting is arithmetic on the notation, not on a name: a dotted 1/8 is 3/16,
 * which is exactly what the source says and what the inspector shows beside
 * it.
 */
export function spellDuration(denominator: number, dotted: boolean): string {
  if (dotted) return `3/${denominator * 2}`;
  return denominator === 1 ? "1" : `1/${denominator}`;
}

/** How the language spells a pitch: letter, accidental, octave. */
export function spellPitch(letter: string, accidental: Accidental, octave: number): string {
  return `${letter}${accidental}${octave}`;
}

/** The octave a written pitch is in, read back out of the core's spelling. */
export function octaveOf(pitch: string | null | undefined): number | null {
  const digits = /(-?\d+)$/.exec(pitch ?? "");
  return digits ? Number(digits[1]) : null;
}

export class NoteEntry {
  /** Whether the letters are pitches right now. */
  on = $state(false);

  /** The duration the next note gets, as a denominator of a whole note. */
  denominator = $state(4);

  /** Whether that duration is dotted. */
  dotted = $state(false);

  /** The octave letters land in, until an octave key moves it. */
  octave = $state(4);

  /** The accidental letters take, until an accidental key clears it. */
  accidental = $state<Accidental>("");

  /** The duration as the source spells it. */
  get duration(): string {
    return spellDuration(this.denominator, this.dotted);
  }

  /** The duration as the top margin draws it. */
  get glyph(): string {
    return durationGlyph(this.denominator, this.dotted);
  }

  /** What the next note would be spelled, in whatever octave is asked for. */
  pitch(letter: string, octave: number = this.octave): string {
    return spellPitch(letter, this.accidental, octave);
  }

  /** Turn entry on or off. Leaving it forgets the accidental, not the duration. */
  set(on: boolean): void {
    this.on = on;
    if (!on) this.accidental = "";
  }

  toggle(): void {
    this.set(!this.on);
  }

  /** A number key. Reports whether it was one. */
  chooseDuration(key: string): boolean {
    const denominator = DURATION_KEYS.get(key);
    if (denominator === undefined) return false;
    this.denominator = denominator;
    return true;
  }

  /** `.` — dotted or not, which is a property of the duration, not a mode. */
  toggleDot(): void {
    this.dotted = !this.dotted;
  }

  /** `⌘↑` `⌘↓` — the octave the letters land in. */
  shiftOctave(by: number): void {
    this.octave = Math.min(Math.max(this.octave + by, OCTAVE_RANGE.low), OCTAVE_RANGE.high);
  }

  /**
   * `⇧↑` `⇧↓` — sharp, natural, flat, in that order in each direction.
   *
   * Three states on one axis rather than two toggles: a composer raising a
   * note presses the same key again to raise it further, and pressing the
   * other one is how it comes back.
   */
  shiftAccidental(by: number): void {
    const ladder: Accidental[] = ["b", "", "#"];
    const at = ladder.indexOf(this.accidental);
    this.accidental = ladder[Math.min(Math.max(at + by, 0), ladder.length - 1)] ?? "";
  }
}
