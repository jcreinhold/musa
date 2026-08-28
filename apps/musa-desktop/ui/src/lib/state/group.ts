/**
 * What a group command is, spelled the way the core takes it.
 *
 * The five intents are five different musical questions, and this module's
 * whole job is to keep them five: a number key writes a duration on every
 * selected event, the vertical arrows walk the staff or the accidental
 * ladder, and an interval is typed. Nothing here computes a pitch, a
 * duration, or a consequence — the core answers all three, and the preview it
 * returns is what the interface shows (`03-interaction.md` §7).
 */

import type { GroupIntentDto } from "../session/generated/GroupIntentDto";

/**
 * The number keys, as `03-interaction.md`'s duration table binds them: the
 * key is the denominator where the digit allows it, and the three that do not
 * fit on one key take the nearest free digit.
 */
const DURATION_KEYS: ReadonlyMap<string, number> = new Map([
  ["1", 1],
  ["2", 2],
  ["4", 4],
  ["8", 8],
  ["6", 16],
  ["3", 32],
]);

/** How the language spells a duration a number key names. */
function spellDuration(denominator: number): string {
  return denominator === 1 ? "1" : `1/${denominator}`;
}

/**
 * The command a number key means with the score focused and a selection made.
 *
 * Setting, not scaling: `4` says "these are quarter notes", which is the
 * question a composer asks by pressing a number (OMT ch. 009). Scaling the
 * rhythm that is already there is a different command and is not a number
 * key.
 */
export function durationIntent(key: string): GroupIntentDto | null {
  const denominator = DURATION_KEYS.get(key);
  if (denominator === undefined) return null;
  return { kind: "setEachDuration", duration: spellDuration(denominator) };
}

/**
 * The command the vertical respelling keys mean.
 *
 * `⌥↑`/`⌥↓` move the notehead and leave the sign alone; holding `⇧` moves the
 * sign and leaves the notehead alone. Two commands, because they are two
 * different marks on the page (OMT ch. 004).
 */
export function respellIntent(steps: number, accidental: boolean): GroupIntentDto {
  return accidental ? { kind: "shiftAccidentals", steps } : { kind: "moveDiatonically", steps };
}

/**
 * The command a typed interval means, or null for text that is not one.
 *
 * Only obvious nonsense is refused here — an empty field. Whether `P5` or
 * `up A4` is an interval is the core's question, and it answers it with a
 * refusal the composer can read rather than a field that silently does
 * nothing.
 */
export function transposeIntent(interval: string): GroupIntentDto | null {
  const written = interval.trim();
  return written === "" ? null : { kind: "transposeBy", interval: written };
}
