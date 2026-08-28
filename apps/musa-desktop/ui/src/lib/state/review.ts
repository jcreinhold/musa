/**
 * How a proposal is laid out and read, without deciding anything about it.
 *
 * Review draws what the project composed: every pitch, duration, line, and
 * bar position in here came off `ReviewFactsDto` and none of it is
 * recomputed. What this module owns is arrangement — which line a note is
 * drawn on, where along the bar it sits, and which questions stand over it —
 * so that the component is markup and the arithmetic is testable
 * (`03-interaction.md` §7).
 */

import type { PlacedVoiceDto } from "../session/generated/PlacedVoiceDto";
import type { PlacementPlanDto } from "../session/generated/PlacementPlanDto";
import type { ReviewAmbiguityDto } from "../session/generated/ReviewAmbiguityDto";
import type { ReviewFactsDto } from "../session/generated/ReviewFactsDto";
import type { ReviewNoteDto } from "../session/generated/ReviewNoteDto";

/** Grid ticks in one whole note, the unit the project states positions in. */
export const WHOLE_TICKS = 96;

/** One note as the leaf draws it: the project's facts plus where they go. */
export interface Placed {
  note: ReviewNoteDto;
  /** Its index in the proposal, which every gesture names it by. */
  index: number;
  /** Left edge as a fraction of the take, 0..1. */
  start: number;
  /** Width as a fraction of the take, never zero. */
  width: number;
}

/** One line of the proposal, drawn as one staff row. */
export interface Row {
  voice: number;
  notes: Placed[];
}

/**
 * How long the whole take runs, in ticks — the last written end, never less
 * than one bar so a single note does not fill the page.
 */
export function extent(facts: ReviewFactsDto): number {
  const end = facts.notes.reduce((longest, note) => Math.max(longest, note.onsetTicks + note.endTicks), 0);
  return Math.max(end, WHOLE_TICKS);
}

/**
 * The proposal as staff rows, one per line it uses, lowest-numbered first.
 *
 * A line with no notes is not a row: the proposal's voice count says how many
 * lines it writes, and drawing an empty one would claim a rest nobody played.
 */
export function rows(facts: ReviewFactsDto): Row[] {
  const total = extent(facts);
  const byVoice = new Map<number, Placed[]>();
  facts.notes.forEach((note, index) => {
    const placed: Placed = {
      note,
      index,
      start: note.onsetTicks / total,
      width: Math.max(note.endTicks / total, 1 / total),
    };
    byVoice.set(note.voice, [...(byVoice.get(note.voice) ?? []), placed]);
  });
  return [...byVoice.entries()].sort(([left], [right]) => left - right).map(([voice, notes]) => ({ voice, notes }));
}

/** The bar lines to draw, as fractions of the take. */
export function barlines(facts: ReviewFactsDto, barTicks: number): number[] {
  if (barTicks <= 0) return [];
  const total = extent(facts);
  const lines: number[] = [];
  for (let tick = barTicks; tick < total; tick += barTicks) lines.push(tick / total);
  return lines;
}

/** One bar of the destination meter, in ticks; 0 when the meter is not one. */
export function barTicks(meter: string): number {
  const [count, unit] = meter.split("/").map((part) => Number.parseInt(part, 10));
  if (!Number.isFinite(count) || !Number.isFinite(unit) || unit <= 0 || count <= 0) return 0;
  if (WHOLE_TICKS % unit !== 0) return 0;
  return (WHOLE_TICKS / unit) * count;
}

/** Which marks stand over a note. */
export function marksOver(index: number, facts: ReviewFactsDto): ReviewAmbiguityDto[] {
  return facts.ambiguities.filter((mark) => mark.notes.includes(index));
}

/** Every note any of the marks covers, so the leaf can halo them together. */
export function marked(facts: ReviewFactsDto): Set<number> {
  return new Set(facts.ambiguities.flatMap((mark) => mark.notes));
}

/**
 * The mark a keyboard reader should reach next, given the one it is on.
 *
 * Marks are ordered by the notes they stand over, so "next question" walks
 * the phrase left to right rather than jumping around the id space.
 */
export function nextMark(facts: ReviewFactsDto, current: string | null, step: 1 | -1): string | null {
  const ids = facts.ambiguities.map((mark) => mark.id);
  if (ids.length === 0) return null;
  const at = current === null ? -1 : ids.indexOf(current);
  if (at === -1) return step === 1 ? (ids[0] ?? null) : (ids[ids.length - 1] ?? null);
  const next = (at + step + ids.length) % ids.length;
  return ids[next] ?? null;
}

/**
 * What the top margin says about where this phrase would go and what it is.
 *
 * One sentence, in the project's own vocabulary. A stale reading says so
 * here rather than anywhere else, because the answer to "can I keep this"
 * belongs beside the button that keeps it (`05-states.md` §4).
 */
export function destination(facts: ReviewFactsDto): string {
  const lines = facts.voiceCount === 1 ? "one line" : `${facts.voiceCount} lines`;
  const notes = facts.notes.length === 1 ? "1 note" : `${facts.notes.length} notes`;
  const where = facts.voice === null ? facts.part : `${facts.part}\u2019s ${facts.voice}`;
  return `${notes} in ${lines}, into ${where}, in ${facts.meter}`;
}

/** Whether the reading can be kept: it writes exactly and is not already kept. */
export function keepable(facts: ReviewFactsDto): boolean {
  return facts.source !== null && !facts.sealed && facts.current;
}

/**
 * Whether the accepted phrase can be written now.
 *
 * Two separate questions, and both have to be yes: the project has said what
 * it would write, and the reading is still of the document on screen. A
 * plan computed against a revision that has moved describes byte ranges of a
 * document nobody has — which is exactly what the project refuses to apply.
 */
export function placeable(facts: ReviewFactsDto, plan: PlacementPlanDto | null): boolean {
  return plan !== null && facts.sealed && facts.current;
}

/**
 * What one planned line would do, in the words the margin uses.
 *
 * The distinction the sentence has to carry is whether the name is a line
 * the part already has — in which case the phrase joins music that is
 * already there — or a line acceptance would add. Those are different acts,
 * and a composer choosing a name is choosing between them.
 */
export function fate(line: PlacedVoiceDto, part: string): string {
  const bars = line.bars === 1 ? "1 bar" : `${line.bars} bars`;
  return line.added ? `${bars} into a new line` : `${bars} into ${part}\u2019s ${line.name}`;
}

/**
 * The names to offer for the plan's lines, keeping whatever is already typed.
 *
 * Seeding is by position and only when the count changes: re-planning after
 * every keystroke answers with the names it was given, and reseeding from
 * that answer would be a loop that fights the person typing.
 */
export function naming(plan: PlacementPlanDto | null, typed: string[]): string[] {
  if (!plan) return [];
  if (typed.length === plan.voices.length) return typed;
  return plan.voices.map((line) => line.name);
}

/**
 * How a run of tapped beats reaches the core: microseconds from the take's
 * start, ascending, with the tap that was the downbeat.
 *
 * The clock is the browser's, and it is used for *intervals* only — the core
 * pins each tap to the nearest played note and never to a wall-clock time,
 * so a slow frame moves a tap and not the take.
 */
export function taps(
  times: number[],
  origin: number,
  downbeat: number | null,
): {
  beatsMicros: bigint[];
  downbeat: number | null;
} {
  return {
    beatsMicros: times.map((time) => BigInt(Math.max(0, Math.round((time - origin) * 1000)))),
    downbeat,
  };
}
