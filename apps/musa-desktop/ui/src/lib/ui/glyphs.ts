/**
 * The SMuFL codepoints the chrome uses. Every one is a symbol an engraver
 * already draws; musa ships no icon font of its own.
 */

/** Repeat signs — `03-interaction.md` §4 draws loop with these, not a box. */
export const REPEAT_RIGHT_LEFT = "\uE042";

/** Individual notes (SMuFL "Individual notes", U+E1D0–), for tempo marks. */
const NOTE_BY_DENOMINATOR = new Map<number, string>([
  [1, "\uE1D2"], // noteWhole
  [2, "\uE1D3"], // noteHalfUp
  [4, "\uE1D5"], // noteQuarterUp
  [8, "\uE1D7"], // note8thUp
  [16, "\uE1D9"], // note16thUp
  [32, "\uE1DB"], // note32ndUp
]);

/** The augmentation dot that follows a note (SMuFL `augmentationDot`). */
const AUGMENTATION_DOT = "\uE1E7";

/**
 * A duration as a musician draws it: the note, plus a dot when it is dotted.
 *
 * This is what the top margin shows while note entry is on — a word like
 * "dotted eighth" would be the interface explaining notation to someone who
 * writes it (`03-interaction.md` §3).
 */
export function durationGlyph(denominator: number, dotted: boolean): string {
  const note = NOTE_BY_DENOMINATOR.get(denominator) ?? "";
  return dotted ? `${note}${AUGMENTATION_DOT}` : note;
}

/**
 * The note glyph for a tempo's beat unit, or `undefined` for a unit no single
 * glyph spells — in which case the caller sets the fraction as text rather
 * than approximating it with the wrong note.
 */
export function tempoNote(
  numerator: number,
  denominator: number,
): string | undefined {
  return numerator === 1 ? NOTE_BY_DENOMINATOR.get(denominator) : undefined;
}

/** `4:37` from a frame count — display arithmetic, not musical time. */
export function elapsed(frames: number, sampleRate: number): string {
  const seconds = sampleRate > 0 ? Math.floor(frames / sampleRate) : 0;
  const minutes = Math.floor(seconds / 60);
  return `${minutes}:${String(seconds % 60).padStart(2, "0")}`;
}
