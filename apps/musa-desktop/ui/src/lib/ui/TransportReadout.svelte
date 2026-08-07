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
  }: { score: ScoreFacts; playback: PlaybackState; bar: number; beat: Rational } = $props();

  const note = $derived(tempoNote(score.tempoBeat.numerator, score.tempoBeat.denominator));
</script>

<div class="readout">
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
    align-items: center;
    gap: var(--s-6);
  }

  .where {
    display: flex;
    align-items: baseline;
    gap: var(--s-2);
  }

  .time {
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    font-variant-numeric: tabular-nums;
    color: var(--ink-faint);
  }

  .facts {
    display: flex;
    align-items: baseline;
    gap: var(--s-6);
    margin: 0;
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
    color: var(--ink-faint);
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
