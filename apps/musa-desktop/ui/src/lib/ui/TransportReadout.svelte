<script lang="ts">
  /**
   * Where you are, how fast, and in what key — the right end of the top
   * margin (`01-visual-language.md` §7).
   *
   * With the transport stopped the position shown is the selection's: "the
   * current position" is where the composer is working, and a readout frozen
   * at `1:1` for the whole session would be furniture. Elapsed time is the
   * only value here the frontend derives, and it derives it from frames the
   * engine reported (`03-interaction.md` §7).
   */
  import type { Fraction as Rational, PlaybackState, ScoreFacts } from "../state/snapshot";
  import Position from "./Position.svelte";
  import { elapsed, tempoNote } from "./glyphs";

  let {
    score,
    playback,
    bar,
    beat,
    stale = false,
  }: {
    score: ScoreFacts;
    playback: PlaybackState;
    bar: number;
    beat: Rational;
    /** What you hear is the last valid plan, not the text (`05-states.md` §4). */
    stale?: boolean;
  } = $props();

  const note = $derived(tempoNote(score.tempoBeat.numerator, score.tempoBeat.denominator));
</script>

<div class="readout" class:stale>
  <div class="where">
    <Position {bar} {beat} />
    <span class="time">{elapsed(playback.positionFrames, playback.sampleRate)}</span>
  </div>
  <dl class="facts">
    <div class="fact">
      <dt>Tempo</dt>
      <dd>
        {#if note}<span class="note" aria-hidden="true">{note}</span>{/if}
        <span class="bpm">= {score.tempoBpm}</span>
      </dd>
    </div>
    <div class="fact">
      <dt>Key</dt>
      <dd class="key">{score.key ?? "—"}</dd>
    </div>
    <div class="fact">
      <dt>Meter</dt>
      <dd>{score.meterCount}/{score.meterUnit}</dd>
    </div>
  </dl>
</div>

<style>
  /*
   * Centred, not baseline-aligned: the bar number is 34px beside 10px labels,
   * and a shared baseline would push its top out of the 48px margin.
   */
  .readout {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--s-5);
  }

  .where {
    display: flex;
    align-items: baseline;
    gap: var(--s-2);
  }

  .stale .time,
  .stale :global(.bar) {
    color: var(--chalk);
  }

  .time {
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    font-variant-numeric: tabular-nums;
    color: var(--ink-muted);
  }

  /*
   * Tempo, key and meter are one group and are set tighter than the gap that
   * separates them from the position: proximity is what says "these three are
   * the score's facts and that one is where you are", and it does it without
   * a rule between them.
   */
  .facts {
    display: flex;
    align-items: baseline;
    gap: var(--s-4);
    margin: 0;
  }

  /*
   * And because they are one group, they go as one. Below the width where the
   * inspector already gives way, the readout keeps the position and stops
   * printing the score's facts: tempo, key and meter belong to the piece and
   * are engraved at the head of the page in the score's own hand, while the
   * position belongs to where you are working and is printed nowhere else. A
   * margin short of room should stop repeating the page before it stops
   * saying anything the page does not (`01-visual-language.md` §7).
   */
  @media (max-width: 1100px) {
    .facts {
      display: none;
    }
  }

  .fact {
    display: flex;
    flex-direction: column;
    gap: var(--s-1);
  }

  dt {
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    font-weight: 500;
    letter-spacing: var(--tracking-micro);
    text-transform: uppercase;
    color: var(--ink-muted);
  }

  dd {
    margin: 0;
    font-family: var(--f-mono);
    font-size: var(--t-value-size);
    line-height: var(--t-value-line);
    font-variant-numeric: tabular-nums;
    color: var(--ink-muted);
    white-space: nowrap;
  }

  .key {
    font-family: var(--f-score-text);
    font-size: var(--t-name-size);
    line-height: var(--t-value-line);
  }

  /* The tempo's note is the score's own glyph, set on the text baseline. */
  .note {
    font-family: var(--f-notation);
    font-size: var(--t-name-size);
    line-height: 0;
  }
</style>
