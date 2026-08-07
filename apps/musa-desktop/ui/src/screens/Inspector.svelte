<script lang="ts">
  /**
   * The right margin: a run of typographic rows, not a form
   * (`01-visual-language.md` §7). Every value here was computed by the core;
   * the frontend spells none of it (`03-interaction.md` §7).
   */
  import Fraction from "../lib/ui/Fraction.svelte";
  import Position from "../lib/ui/Position.svelte";
  import TypographicRow from "../lib/ui/TypographicRow.svelte";
  import type { EventFacts } from "../lib/state/snapshot";

  let {
    event,
    adrift = null,
  }: {
    event: EventFacts | undefined;
    /**
     * Set when a re-render moved the selection because the event it was on is
     * gone. Saying so is the difference between a selection that followed the
     * edit and one that silently describes a different note
     * (`02-engraving.md` §6).
     */
    adrift?: string | null;
  } = $props();

  const sounds = $derived(event && event.kind !== "rest");

  // `durationSpelling` is how the composer wrote it. Showing it beside the
  // exact value earns its space only when the two differ — a dotted quarter
  // written `1/4.` and sounding `3/8` is worth saying; `1/2` twice is not.
  const asWritten = $derived(
    event && event.durationSpelling !== `${event.duration.numerator}/${event.duration.denominator}`
      ? event.durationSpelling
      : undefined,
  );
</script>

{#if event}
  <div class="inspector">
    {#if adrift}
      <p class="adrift" role="status">{adrift} — this is the nearest note in the same voice.</p>
    {/if}

    <TypographicRow label={event.kind === "chord" ? "Pitches" : "Pitch"} editable>
      {#if sounds}{event.pitches.join(" ")}{:else}<span class="rest">rest</span>{/if}
    </TypographicRow>

    {#snippet written()}written {asWritten}{/snippet}
    <TypographicRow label="Duration" editable trailing={asWritten ? written : undefined}>
      <Fraction value={event.duration} />
    </TypographicRow>

    <TypographicRow label="Position">
      <Position bar={event.bar} beat={event.beat} size="value" />
    </TypographicRow>

    <TypographicRow label="Voice">
      {event.part} <span class="sep">▸</span>
      {event.voice}
    </TypographicRow>

    <!--
      The Origin row of 04-provenance.md §3: always present, held or not. It
      reads outside-in, in containment order — the `use` that produced these
      notes sits inside the transform block, and the path says so. Its
      segments become clickable in prompt 24.
    -->
    <TypographicRow label="Origin">
      {#snippet trailing()}line {event.origin.line}{/snippet}
      {#if event.origin.generated}
        <span class="path">
          {#each event.origin.path as segment, index (index)}
            {#if index > 0}<span class="sep">▸</span>{/if}<span class="segment">{segment}</span>
          {/each}
          {#if event.origin.noteIndex !== null}<span class="sep">▸</span><span class="segment"
              >note {event.origin.noteIndex}</span
            >{/if}
        </span>
      {:else}
        <span class="authored">authored</span>
      {/if}
    </TypographicRow>
  </div>
{:else}
  <p class="empty">Nothing selected.</p>
{/if}

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    gap: var(--s-4);
  }

  .adrift {
    margin: 0;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--chalk);
  }

  .rest,
  .authored {
    color: var(--ink-muted);
  }

  .sep {
    color: var(--ink-faint);
    padding: 0 0.15em;
  }

  /* Generated material is the one thing musa uniquely knows; --plate says so. */
  .path .segment {
    color: var(--plate);
  }

  .path {
    display: inline;
    overflow-wrap: anywhere;
  }

  .empty {
    margin: 0;
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-faint);
  }
</style>
