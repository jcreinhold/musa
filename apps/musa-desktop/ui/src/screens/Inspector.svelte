<script lang="ts">
  /**
   * The right margin: a run of typographic rows, not a form
   * (`01-visual-language.md` §7). Every value here was computed by the core;
   * the frontend spells none of it (`03-interaction.md` §7).
   */
  import { mark } from "../lib/perf";
  import Fraction from "../lib/ui/Fraction.svelte";
  import Position from "../lib/ui/Position.svelte";
  import TypographicRow from "../lib/ui/TypographicRow.svelte";
  import EditableValue from "../lib/ui/EditableValue.svelte";
  import type {
    DecisionFact,
    EditImpact,
    EventFacts,
    HeaderFact,
    OccurrenceFacts,
    Span,
  } from "../lib/state/snapshot";
  import type { HeaderFieldDto } from "../lib/session/generated/HeaderFieldDto";

  let {
    event,
    adrift = null,
    occurrence,
    choice = null,
    naming = null,
    onorigin,
    decision = null,
    onkeep,
    siblings = 0,
    onconfirm,
    onspecialize,
    oncancel,
    onname,
    oncancelname,
    onpitch,
    onduration,
    onreveal,
    header = [],
    onheader,
  }: {
    event: EventFacts | undefined;
    /**
     * Set when a re-render moved the selection because the event it was on is
     * gone. Saying so is the difference between a selection that followed the
     * edit and one that silently describes a different note
     * (`02-engraving.md` §6).
     */
    adrift?: string | null;
    /**
     * Ask for everything this generator produced, given how many segments of
     * the origin path were clicked. Absent where there is no score to select
     * in — the engraving goldens.
     */
    onorigin?: (depth: number) => void;
    /**
     * The decision this event was played under, when one was made (prompt
     * 76). Null in a determinate piece and for a note no open construct
     * covers, and the row is then not there: a piece that asked nothing has
     * nothing to answer for.
     */
    decision?: DecisionFact | null;
    /**
     * Keep this decision as it came out, or let it go back to being drawn.
     * Absent while the session is not live.
     */
    onkeep?: (path: string, keep: boolean) => void;
    /**
     * How many notes the statement that spells this one spelled in all
     * (prompt 52). One is the ordinary case and says nothing; more is the fact
     * an editing choice will later have to state, so the origin says it first.
     */
    siblings?: number;
    /** The expansion that produced this event, when one did. */
    occurrence?: OccurrenceFacts;
    /**
     * An edit against generated music, waiting on the composer
     * (`04-provenance.md` §4). The counts are the core's; the interface only
     * sets them.
     */
    choice?: EditImpact | null;
    onconfirm?: () => void;
    onspecialize?: () => void;
    oncancel?: () => void;
    /**
     * How many notes an extraction is waiting to cover, or null when none is.
     * The name is asked for here, beside them, rather than in a dialog
     * (§14.5).
     */
    naming?: number | null;
    onname?: (name: string) => void;
    oncancelname?: () => void;
    /** Respell this event's pitch. Absent where there is nothing to edit. */
    onpitch?: (pitch: string) => void;
    /** Renotate its duration. */
    onduration?: (duration: string) => void;
    /** Open the source at a span: the line number's whole job. */
    onreveal?: (span: Span) => void;
    /**
     * What the piece says about itself, as the source spells it — every field
     * it can state, including the ones it has not. This is what the inspector
     * shows when nothing is selected. Empty where there is no piece to read.
     */
    header?: HeaderFact[];
    /** Rewrite one of those statements; absent while the session is not live. */
    onheader?: (field: HeaderFieldDto, value: string) => void;
  } = $props();

  /** How many notes the edit would change — the honest count, not the size
      of the expansions it touches. */
  const notes = $derived(choice?.events.length ?? 0);
  const count = $derived(choice?.occurrences ?? 0);

  /**
   * The motif name being typed, and the field it is typed in. It takes focus
   * the moment it appears, because the composer asked for it with a key and
   * would otherwise have to reach for the mouse to answer.
   */
  let motifName = $state("");
  let nameField = $state<HTMLInputElement | null>(null);
  $effect(() => {
    if (naming !== null) {
      motifName = "";
      nameField?.focus();
    }
  });

  function submitName(): void {
    const name = motifName.trim();
    if (name !== "") onname?.(name);
  }

  /** Where "line N" goes: the `use` statement, or failing that the event. */
  const at = $derived(occurrence?.useSite ?? event?.origin.span);

  const sounds = $derived(event && event.kind !== "rest");

  // B4: the inspector is populated from the same snapshot the halo came from,
  // so this marks the frame the composer can actually read the facts in.
  $effect(() => {
    void event?.id;
    mark("inspector");
  });

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
    <!--
      The choice of `04-provenance.md` §4: inline, never a modal, with the
      consequence stated in counts. Origin view is already open and holding
      itself, and the notes below are already haloed, so what this asks is a
      question the composer can see the answer to.
    -->
    {#if choice}
      <div class="choice" role="group" aria-label="Editing generated music">
        <p class="about">
          This note comes from <span class="from">{choice.occurrence ?? "an expansion"}</span>.
        </p>
        <button type="button" class="option" onclick={() => onconfirm?.()}>
          <span class="what">Edit the motif</span>
          <span class="cost"
            >changes {count}
            {count === 1 ? "occurrence" : "occurrences"}, {notes}
            {notes === 1 ? "note" : "notes"}</span
          >
        </button>
        <button
          type="button"
          class="option"
          disabled={!choice.specializable}
          onclick={() => onspecialize?.()}
        >
          <span class="what">Just this occurrence</span>
          <span class="cost"
            >{choice.specializable
              ? "changes this note, and writes it onto the call"
              : "this call runs more than once, so an override would change every run"}</span
          >
        </button>
        <button type="button" class="cancel" onclick={() => oncancel?.()}>Cancel</button>
      </div>
    {/if}

    <!--
      Extraction, named where the notes are. One field and one sentence: the
      count is the core's, and the notes it covers are already haloed.
    -->
    {#if naming !== null}
      <form
        class="naming"
        aria-label="Name this motif"
        onsubmit={(submitted) => {
          submitted.preventDefault();
          submitName();
        }}
      >
        <p class="about">Extract {naming} {naming === 1 ? "note" : "notes"} into a motif.</p>
        <input
          class="name"
          type="text"
          aria-label="Motif name"
          placeholder="name it"
          spellcheck="false"
          autocomplete="off"
          bind:this={nameField}
          bind:value={motifName}
          onkeydown={(pressed) => {
            pressed.stopPropagation();
            if (pressed.key === "Escape") oncancelname?.();
          }}
        />
      </form>
    {/if}

    {#if adrift}
      <p class="adrift" role="status">{adrift} — this is the nearest note in the same voice.</p>
    {/if}

    <!--
      Pitch and duration are the two things a composer changes from here, and
      they are typed in the language's own spelling — `gs4`, `3/8` — because
      that is what the source will say and there is no second notation to
      learn. A single note only: a chord is four values in one row, which is a
      field this row is not.
    -->
    <TypographicRow label={event.kind === "chord" ? "Pitches" : "Pitch"} editable>
      {#if event.kind === "note" && onpitch}
        <EditableValue
          value={event.pitchSpellings[0] ?? ""}
          label="Pitch"
          onchange={(next) => onpitch(next)}
        />
      {:else if sounds}{event.pitches.join(" ")}{:else}<span class="rest">rest</span>{/if}
    </TypographicRow>

    {#snippet written()}written {asWritten}{/snippet}
    <TypographicRow label="Duration" editable trailing={asWritten ? written : undefined}>
      {#if onduration}
        <EditableValue
          value={event.durationSpelling}
          label="Duration"
          onchange={(next) => onduration(next)}
        />
      {:else}<Fraction value={event.duration} />{/if}
    </TypographicRow>

    <TypographicRow label="Position">
      <Position bar={event.bar} beat={event.beat} size="value" />
    </TypographicRow>

    <TypographicRow label="Voice">
      {event.part} <span class="sep">▸</span>
      {event.voice}
    </TypographicRow>

    <!--
      What is in force *here*, not what the header said: a piece modulates and
      a part changes clef, and an inspector that showed the opening key beside
      a note in a later one would be quietly wrong. Absent when the piece has
      said nothing — an unspecified key is not C major.
    -->
    {#if event.key}
      <TypographicRow label="Key">{event.key}</TypographicRow>
    {/if}
    {#if event.clef}
      <TypographicRow label="Clef">{event.clef}</TypographicRow>
    {/if}

    <!--
      The Origin row of 04-provenance.md §3: always present, held or not. It
      reads outside-in, in containment order — the `use` that produced these
      notes sits inside the transform block, and the path says so.

      Every part of it is a control. A transform segment selects what it
      produced in this voice; the innermost segment is the occurrence itself,
      so it selects the whole expansion and reveals the motif's declaration in
      the source column; the line number opens the source at the `use` statement.
    -->
    <TypographicRow label="Origin">
      {#snippet trailing()}
        {#if at && onreveal}
          <button type="button" class="segment line" onclick={() => onreveal(at)}
            >line {event.origin.line}</button
          >
        {:else}line {event.origin.line}{/if}{#if siblings > 1}<span class="sep">·</span><span
            class="kin">{siblings} notes</span
          >{/if}
      {/snippet}
      {#if event.origin.generated}
        <span class="path">
          {#each event.origin.path as segment, index (index)}
            {#if index > 0}<span class="sep">▸</span>{/if}<button
              type="button"
              class="segment"
              disabled={onorigin === undefined}
              onclick={() => {
                mark("origin");
                onorigin?.(index + 1);
              }}>{segment}</button
            >
          {/each}
          {#if event.origin.noteIndex !== null}<span class="sep">▸</span><span class="segment note"
              >note {event.origin.noteIndex}</span
            >{/if}
        </span>
      {:else}
        <span class="authored">authored</span>
      {/if}
    </TypographicRow>

    <!--
      The last step of the origin, and the only one that is not about where
      the note is written: it is about why there are this many of them
      (prompt 76). The source says what the piece allows; this row says what
      this reading decided, and offers the one control that makes a decision
      stop moving.

      Both strings are the core's. "the fill, first choice" and "4 passes"
      are musical sentences, and a frontend that assembled them would be
      spelling a musical fact (`03-interaction.md` §7).
    -->
    {#if decision}
      <TypographicRow label="Decision">
        {#snippet trailing()}
          {#if onkeep}
            <button
              type="button"
              class="segment keep"
              aria-pressed={decision.pinned}
              onclick={() => onkeep(decision.path, !decision.pinned)}
              >{decision.pinned ? "kept" : "keep this one"}</button
            >
          {:else if decision.pinned}<span class="kept">kept</span>{/if}
        {/snippet}
        <span class="path">
          {#if onreveal}<button
              type="button"
              class="segment"
              onclick={() => onreveal(decision.span)}>{decision.asked}</button
            >{:else}<span class="segment">{decision.asked}</span>{/if}<span class="sep">▸</span
          ><span class="segment answered">{decision.answered}</span>
        </span>
      </TypographicRow>
    {/if}
  </div>
{:else if header.length > 0}
  <!--
    Nothing selected is not nothing. With no note in hand the inspector shows
    the piece: every statement its header can carry, including the ones it
    does not carry yet. One move answers what can be changed here, what this
    piece has already said, and where to click to say something it has not —
    which is why there is no help bar (prompt 54).

    The values are the source's own spellings: `quarter = 72`, `a minor`,
    `4/4`. The band may go on printing `♩ = 72` at rest, but what a composer
    types is musa, because a field that accepted a second dialect would be a
    second language to keep working.
  -->
  <div class="inspector piece" role="group" aria-label="This piece">
    {#each header as fact (fact.field)}
      <TypographicRow label={fact.field} editable={onheader !== undefined}>
        {#if onheader}
          <EditableValue
            value={fact.value ?? ""}
            label={fact.field}
            placeholder={fact.field === "title" ? undefined : "—"}
            allowEmpty={fact.field !== "title"}
            onchange={(next) => onheader(fact.field, next)}
          />
        {:else}{fact.value ?? "—"}{/if}
      </TypographicRow>
    {/each}
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

  /*
   * The choice reads as a short piece of prose with two answers under it, not
   * as a dialog: no box, no shadow, no overlay. `--plate` marks it as being
   * about provenance, which is the only thing that colour ever means.
   */
  .choice {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    border-left: 2px solid var(--plate);
    padding-left: var(--s-3);
  }

  /* Same left rule as the choice: both are the source about to change. */
  .naming {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    border-left: 2px solid var(--plate);
    padding-left: var(--s-3);
  }

  .name {
    background: none;
    border: 0;
    border-bottom: 1px solid var(--rule);
    padding: 0 0 2px;
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    color: var(--ink);
  }

  .name:focus {
    outline: none;
    border-bottom-color: var(--plate);
  }

  .about {
    margin: 0;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink);
  }

  .from {
    font-family: var(--f-mono);
    color: var(--plate);
  }

  .option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    background: none;
    border: 0;
    border-radius: var(--radius-control);
    padding: var(--s-1) var(--s-2);
    margin-left: calc(var(--s-2) * -1);
    text-align: left;
    cursor: pointer;
  }

  .option:hover:not(:disabled) {
    background: var(--plate-wash);
  }

  .option:disabled {
    cursor: default;
  }

  .what {
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink);
  }

  .option:disabled .what {
    color: var(--ink-muted);
  }

  .cost {
    font-family: var(--f-ui);
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    color: var(--ink-muted);
  }

  .cancel {
    align-self: flex-start;
    background: none;
    border: 0;
    padding: 0;
    font-family: var(--f-ui);
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    color: var(--ink-muted);
    cursor: pointer;
  }

  .cancel:hover,
  .cancel:focus-visible {
    text-decoration: underline;
  }

  .sep {
    color: var(--ink-faint);
    padding: 0 0.15em;
  }

  /* The sibling count is a fact about the line, not a second control: it sits
     in the same trailing, unemphasised, and nothing about it is clickable. */
  .kin {
    color: var(--ink-faint);
  }

  /* Generated material is the one thing musa uniquely knows; --plate says so. */
  .path .segment {
    color: var(--plate);
  }

  /*
   * A segment is a control that looks like the text it is. Underlining it on
   * hover is the whole affordance: a row of buttons in the margin would read
   * as a form, which this is not.
   */
  button.segment {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    cursor: pointer;
  }

  button.segment:hover:not(:disabled),
  button.segment:focus-visible {
    text-decoration: underline;
  }

  button.segment:disabled {
    cursor: default;
  }

  /* The line number is chrome, not provenance: it stays the row's quiet trailing. */
  button.segment.line {
    color: inherit;
  }

  /* What this reading decided is a fact about the performance, not about the
     source, so it is not --plate: nothing generated it. */
  .path .segment.answered {
    color: var(--ink);
  }

  /* "keep this one" is an offer; "kept" is a state. One control says both,
     and the underline is what tells them apart. */
  .kept,
  button.segment.keep {
    color: var(--ink-muted);
  }

  button.segment.keep[aria-pressed="true"] {
    color: var(--ink);
    text-decoration: underline;
  }

  .path {
    display: inline;
    overflow-wrap: anywhere;
  }

  .empty {
    margin: 0;
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }
</style>
