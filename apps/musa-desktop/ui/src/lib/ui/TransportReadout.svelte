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
   *
   * Tempo, key, and meter are the piece's own statements, and here they are
   * fields. A composer looking at the band is looking at the
   * piece's tempo; making them find the inspector to change it is the app
   * knowing something it will not act on. They are the same three fields the
   * inspector shows, in a second place, with the same commit rule.
   */
  import type {
    Fraction as Rational,
    HeaderFact,
    PlaybackState,
    ScoreFacts,
  } from "../state/snapshot";
  import type { HeaderFieldDto } from "../session/generated/HeaderFieldDto";
  import EditableValue from "./EditableValue.svelte";
  import Position from "./Position.svelte";
  import { elapsed, tempoNote } from "./glyphs";

  let {
    score,
    playback,
    bar,
    beat,
    stale = false,
    onheader,
  }: {
    score: ScoreFacts;
    playback: PlaybackState;
    bar: number;
    beat: Rational;
    /** What you hear is the last valid plan, not the text (`05-states.md` §4). */
    stale?: boolean;
    /** Rewrite one of the piece's own statements; absent when not live. */
    onheader?: (field: HeaderFieldDto, value: string) => void;
  } = $props();

  const note = $derived(
    tempoNote(score.tempoBeat.numerator, score.tempoBeat.denominator),
  );

  /** What the source says for one of the three, for the field to start from. */
  const said = $derived(
    (field: HeaderFieldDto) =>
      score.header.find((fact: HeaderFact) => fact.field === field)?.value ??
      "",
  );
</script>

<div class="readout" class:stale>
  <div class="where">
    <Position {bar} {beat} />
    <span class="time"
      >{elapsed(playback.positionFrames, playback.sampleRate)}</span
    >
  </div>
  <dl class="facts">
    <!--
      At rest each of these reads the way an engraver would set it — `♩ = 72`,
      `A minor`. Under focus it is the source's own spelling, because that is
      what the composer types and there is no second notation to learn.
    -->
    <div class="fact">
      <dt>Tempo</dt>
      <dd class:editable={onheader}>
        {#if onheader}
          <EditableValue
            value={said("tempo")}
            label="Tempo"
            onchange={(next) => onheader("tempo", next)}
          />
        {:else}
          {#if note}<span class="note" aria-hidden="true">{note}</span>{/if}
          <span class="bpm">= {score.tempoBpm}</span>
        {/if}
      </dd>
    </div>
    <div class="fact">
      <dt>Key</dt>
      <dd class="key" class:editable={onheader}>
        {#if onheader}
          <EditableValue
            value={said("key")}
            label="Key"
            placeholder="—"
            allowEmpty
            onchange={(next) => onheader("key", next)}
          />
        {:else}{score.key ?? "—"}{/if}
      </dd>
    </div>
    <div class="fact">
      <dt>Meter</dt>
      <dd class:editable={onheader}>
        {#if onheader}
          <EditableValue
            value={said("meter")}
            label="Meter"
            onchange={(next) => onheader("meter", next)}
          />
        {:else}{score.meterCount}/{score.meterUnit}{/if}
      </dd>
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

  /*
   * The same three-weight hairline the inspector's rows carry, so a field
   * reads as a field in whichever margin it is printed.
   *
   * The pixel it costs is reserved on every row, editable or not: a band that
   * grew by a pixel the moment the session went live would shift every page
   * under it, and a readout and a field should occupy the same space anyway.
   */
  dd {
    border-bottom: 1px solid transparent;
  }

  .editable {
    border-bottom-color: var(--rule);
    color: var(--ink);
  }

  /*
   * Block-level, not inline: an inline field aligns on its baseline and adds
   * a pixel of descender to the line box, which is the same shift by another
   * route.
   *
   * `grid` rather than `block`, because the field *is* a grid — it stacks the
   * entry on a hidden copy of its own text, and that copy is what gives it a
   * width. Flattening it to `block` put the two side by side and left the
   * entry one character wide.
   */
  .editable :global(.field) {
    display: grid;
    /* Content-width, as `inline-grid` gives everywhere else: a block-level
       grid would fill the band and put the hairline under nothing. */
    width: fit-content;
  }

  .editable:hover {
    border-bottom-color: var(--ink-muted);
  }

  .editable:focus-within {
    border-bottom-color: var(--plate);
  }

  /* The tempo's note is the score's own glyph, set on the text baseline. */
  .note {
    font-family: var(--f-notation);
    font-size: var(--t-name-size);
    line-height: 0;
  }
</style>
