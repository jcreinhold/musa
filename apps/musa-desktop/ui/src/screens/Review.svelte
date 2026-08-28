<script lang="ts" module>
  /** The onset group a `group-N` mark stands over. */
  export function markGroup(id: string): number {
    const digits = id.slice(id.lastIndexOf("-") + 1);
    const group = Number.parseInt(digits, 10);
    return Number.isFinite(group) ? group : 0;
  }
</script>

<script lang="ts">
  /**
   * Review: one take, read as notation, with the few decisions that matter.
   *
   * The leaf draws the proposal in the score's own visual language; the top
   * margin says where it would go and offers Accept and Discard; the right
   * margin asks whatever is still open. Everything musical here — pitches,
   * written values, lines, bar positions, the words a mark uses, the sentence
   * each action reports — came off the project's reading. This screen chooses
   * arrangement and keystrokes, and computes no notation
   * (`03-interaction.md` §7).
   *
   * Progressive disclosure is the whole design: a take the search is sure of
   * shows as ordinary notation and asks nothing, so keeping it is one press.
   * A mark appears only where the readings that survived actually disagree
   * (prompt 203), and selecting it shows the two or three readings in place.
   */
  import Leaf from "../lib/ui/Leaf.svelte";
  import { mark as moment } from "../lib/perf";
  import Margin from "../lib/ui/Margin.svelte";
  import type { ReviewActionDto } from "../lib/session/generated/ReviewActionDto";
  import type { ReviewAuditionDto } from "../lib/session/generated/ReviewAuditionDto";
  import type { ReviewFactsDto } from "../lib/session/generated/ReviewFactsDto";
  import type { PlacementPlanDto } from "../lib/session/generated/PlacementPlanDto";
  import { durationIntent, respellIntent, transposeIntent } from "../lib/state/group";
  import {
    barTicks,
    barlines,
    destination,
    fate,
    keepable,
    marked,
    marksOver,
    naming,
    nextMark,
    placeable,
    rows,
    taps,
  } from "../lib/state/review";

  let {
    facts,
    plan = null,
    refusal = null,
    onact,
    onundo,
    onaudition,
    onaccept,
    onplan,
    onplace,
    ondiscard,
  }: {
    /** The reading, as the project composed it. */
    facts: ReviewFactsDto;
    /** What keeping it would write, once it has been accepted. */
    plan?: PlacementPlanDto | null;
    /** Why the project would not write it, in the project's own words. */
    refusal?: string | null;
    /** Make one gesture. Absent while nothing is answering. */
    onact?: (action: ReviewActionDto) => void;
    onundo?: () => void;
    onaudition?: (mode: ReviewAuditionDto) => void;
    onaccept?: () => void;
    /** Ask again with these line names. Writes nothing. */
    onplan?: (voices: string[]) => void;
    /** Write the phrase into these lines. */
    onplace?: (voices: string[]) => void;
    ondiscard?: () => void;
  } = $props();

  /** Which proposal notes the reader has selected, by index. */
  let selection = $state<number[]>([]);
  /** Which mark is open, so its readings show in place. */
  let open = $state<string | null>(null);
  /** Whether the transpose field is showing, and what is typed in it. */
  let transposing = $state(false);
  let interval = $state("");
  /** Tap times, in page milliseconds, while a pulse is being tapped. */
  let tapping = $state<number[] | null>(null);
  /** The interval field, so asking for one puts the caret in it. */
  let field = $state<HTMLInputElement | null>(null);

  /*
   * Focus follows the selection, and survives the proposal being replaced.
   *
   * A decision composes a new proposal, so the button the composer was on is
   * a new element — but it is the *same note*, because selection is an index
   * into a take that no decision changes. Restoring focus by that index is
   * what makes a keyboard reading of a phrase continuous rather than a series
   * of returns to the top (`03-interaction.md` §5).
   */
  $effect(() => {
    // Read what a new proposal changes, so this runs again when it does.
    void facts;
    moment("reviewDrawn");
    const only = selection.length === 1 ? selection[0] : undefined;
    if (only === undefined) return;
    document.getElementById(`proposal-${only}`)?.focus();
    moment("reviewFocus");
  });

  // The composer asked for an interval, so the caret goes where they type it.
  // `autofocus` is refused by the browser when something already has focus,
  // which on a keyboard path is always.
  $effect(() => {
    if (transposing) field?.focus();
  });

  const bar = $derived(barTicks(facts.meter));
  const staves = $derived(rows(facts));
  const lines = $derived(barlines(facts, bar));
  const questioned = $derived(marked(facts));
  const ready = $derived(keepable(facts));
  const settled = $derived(placeable(facts, plan));

  /**
   * The names the phrase would be written into, one per planned line.
   *
   * Typed here and answered by the project: every change re-asks for the
   * plan, so "does this part already have a line called that" is the
   * project's question and not this screen's guess.
   */
  let typed = $state<string[]>([]);
  // The guard is load-bearing, exactly as it is for the selection above: an
  // effect that writes the state it read runs forever otherwise.
  $effect(() => {
    const offered = naming(plan, typed);
    if (offered.length !== typed.length || offered.some((name, at) => name !== typed[at])) typed = offered;
  });

  function rename(at: number, name: string): void {
    typed = typed.map((held, index) => (index === at ? name : held));
    onplan?.(typed);
  }

  /** Write it. The names go with it, so what was read is what is written. */
  function keep(): void {
    onplace?.(typed);
  }

  // A mark that has been settled is no longer open: the panel must not go on
  // offering readings for a question that has been answered.
  $effect(() => {
    if (open !== null && !facts.ambiguities.some((mark) => mark.id === open)) open = null;
  });

  // Focus survives a new proposal because selection is by *index* into a
  // proposal whose length decisions never change — the take is immutable, so
  // note 4 is the same key press before and after every decision. The guard
  // is load-bearing: an effect that writes the state it read runs forever.
  $effect(() => {
    const kept = selection.filter((index) => index < facts.notes.length);
    if (kept.length !== selection.length) selection = kept;
  });

  function toggle(index: number, extend: boolean): void {
    if (!extend) {
      selection = selection.includes(index) && selection.length === 1 ? [] : [index];
    } else {
      selection = selection.includes(index)
        ? selection.filter((other) => other !== index)
        : [...selection, index].sort((left, right) => left - right);
    }
    const marks = marksOver(index, facts);
    open = marks[0]?.id ?? null;
  }

  /** Every gesture leaves through here, so one place marks what it owes. */
  function act(action: ReviewActionDto): void {
    moment("reviewAct");
    onact?.(action);
  }

  function transform(intent: ReturnType<typeof durationIntent>): void {
    if (!intent || selection.length === 0) return;
    act({ kind: "transform", notes: selection, intent });
  }

  /** The notes a mark stands over, spelled — what tells two marks apart. */
  function pitches(mark: { notes: number[] }): string {
    return mark.notes.map((index) => facts.notes[index]?.pitch ?? "").join(" ");
  }

  function choose(ambiguity: string, choice: string): void {
    act({ kind: "choose", ambiguity, choice });
  }

  /** Begin or end a run of taps; ending one sends the pulse to the project. */
  function tap(): void {
    if (tapping === null) {
      tapping = [performance.now()];
      return;
    }
    tapping = [...tapping, performance.now()];
  }

  function readPulse(): void {
    const times = tapping;
    tapping = null;
    if (!times || times.length < 2) return;
    const { beatsMicros, downbeat } = taps(times, times[0] ?? 0, 0);
    act({ kind: "tap", beatsMicros, downbeat });
  }

  function typedInterval(): void {
    const intent = transposeIntent(interval);
    transposing = false;
    interval = "";
    if (intent) transform(intent);
  }

  /**
   * The keyboard path to every gesture the pointer has.
   *
   * Arrow keys walk the phrase, `Tab`-free question stepping is on `n`/`p`,
   * numbers write values, the bracket keys walk the staff and the accidental
   * ladder, and the three verbs — undo, accept, discard — are one press each.
   */
  function onkeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.metaKey || event.ctrlKey) return;
    const target = event.target as HTMLElement | null;
    if (target?.tagName === "INPUT") return;
    const only = selection.length === 1 ? selection[0] : null;

    switch (event.key) {
      case "ArrowRight":
      case "ArrowLeft": {
        const step = event.key === "ArrowRight" ? 1 : -1;
        const at = only ?? (step === 1 ? -1 : facts.notes.length);
        const next = Math.min(Math.max(at + step, 0), facts.notes.length - 1);
        toggle(next, false);
        event.preventDefault();
        return;
      }
      case "ArrowUp":
      case "ArrowDown": {
        if (selection.length === 0) return;
        const steps = event.key === "ArrowUp" ? 1 : -1;
        transform(respellIntent(steps, event.shiftKey));
        event.preventDefault();
        return;
      }
      case "n":
      case "p": {
        open = nextMark(facts, open, event.key === "n" ? 1 : -1);
        event.preventDefault();
        return;
      }
      case "t": {
        if (selection.length > 0) transposing = true;
        event.preventDefault();
        return;
      }
      case "u": {
        onundo?.();
        event.preventDefault();
        return;
      }
      case "k": {
        if (settled) keep();
        else if (ready) onaccept?.();
        event.preventDefault();
        return;
      }
      case "Escape": {
        if (transposing) transposing = false;
        else if (tapping !== null) readPulse();
        else if (open !== null) open = null;
        else if (selection.length > 0) selection = [];
        else ondiscard?.();
        event.preventDefault();
        return;
      }
      default:
        break;
    }

    const value = durationIntent(event.key);
    if (value && selection.length > 0) {
      transform(value);
      event.preventDefault();
    }
  }
</script>

<svelte:window on:keydown={onkeydown} />

<div class="workspace">
  <Margin side="top">
    <div class="identity">
      <h1 class="title">Review</h1>
      <!--
        One line, and which one depends on what is still open: what the take
        *is* while that is the question, and where it *goes* once the notation
        is settled. Saying both at once would be two sentences competing to be
        the one thing the margin is for (`05-states.md` §1).
      -->
      {#if facts.sealed}
        <p class="kept" role="status">{facts.takeName} — {plan ? plan.summary : "Kept — ready to place."}</p>
      {:else}
        <p class="where">{facts.takeName} — {destination(facts)}</p>
      {/if}
      {#if !facts.current}
        <p class="stale" role="status">The piece changed while you were reading this.</p>
      {/if}
      {#if refusal}
        <p class="refused" role="alert">{refusal}</p>
      {/if}
    </div>

    <div class="verbs" role="group" aria-label="What to do with this phrase">
      <div class="hearing" role="group" aria-label="What to hear">
        <button
          type="button"
          class="text"
          aria-pressed={facts.audition === "played"}
          onclick={() => {
            moment("reviewAudition");
            onaudition?.("played");
          }}>Played</button
        >
        <button
          type="button"
          class="text"
          aria-pressed={facts.audition === "written"}
          onclick={() => {
            moment("reviewAudition");
            onaudition?.("written");
          }}>Written</button
        >
      </div>
      <button type="button" class="text" onclick={tapping === null ? tap : readPulse}>
        {tapping === null ? "Tap the pulse" : `Read ${tapping.length} taps`}
      </button>
      {#if tapping !== null}
        <button type="button" class="text" onclick={tap}>Tap</button>
      {/if}
      <button type="button" class="text" disabled={facts.history.length === 0} onclick={() => onundo?.()}>
        Take back
      </button>
      {#if facts.sealed}
        <button type="button" class="text keep" disabled={!settled} onclick={keep}>Keep</button>
      {:else}
        <button type="button" class="text keep" disabled={!ready} onclick={() => onaccept?.()}>Accept</button>
      {/if}
      <button type="button" class="text" onclick={() => ondiscard?.()}>Discard</button>
    </div>
  </Margin>

  <div class="body">
    <main class="stage">
      <Leaf stale={!facts.current}>
        <div class="paper" role="group" aria-label="The proposed notation">
          {#each lines as at (at)}
            <div class="barline" style:left={`${at * 100}%`}></div>
          {/each}
          {#each staves as row (row.voice)}
            <div class="stave" role="group" aria-label={`Line ${row.voice + 1}`}>
              <div class="rule"></div>
              {#each row.notes as placed (placed.index)}
                <button
                  type="button"
                  class="note"
                  class:selected={selection.includes(placed.index)}
                  class:asked={questioned.has(placed.index)}
                  class:pedal={placed.note.pedalExtended}
                  class:grace={placed.note.grace}
                  id={`proposal-${placed.index}`}
                  style:left={`${placed.start * 100}%`}
                  style:width={`${placed.width * 100}%`}
                  aria-pressed={selection.includes(placed.index)}
                  aria-label={questioned.has(placed.index) ? `${placed.note.name} — needs a choice` : placed.note.name}
                  onclick={(event) => toggle(placed.index, event.shiftKey)}
                >
                  <span class="head" aria-hidden="true">{placed.note.pitch}</span>
                </button>
              {/each}
            </div>
          {/each}
        </div>
      </Leaf>
    </main>

    <Margin side="right" label="Choices">
      <div class="asks">
        {#if facts.ambiguities.length === 0 && !plan}
          <p class="settled">
            {facts.sealed ? "This phrase is kept." : "Nothing to decide — this reads one way."}
          </p>
        {/if}

        <!--
          The last question the phrase asks, and the only one whose answer is
          typed: which line of the part each voice of the reading joins. It
          appears only once the notation is settled, because until then there
          is no phrase to put anywhere (`03-interaction.md` §7).
        -->
        {#if plan}
          <div class="mark open lines" role="group" aria-label="Where this phrase goes">
            <ul class="choices">
              {#each plan.voices as line, at (line.proposalVoice)}
                <li class="line">
                  <label for={`line-${line.proposalVoice}`}>Line {line.proposalVoice + 1}</label>
                  <input
                    id={`line-${line.proposalVoice}`}
                    type="text"
                    spellcheck="false"
                    autocomplete="off"
                    value={typed[at] ?? line.name}
                    oninput={(event) => rename(at, event.currentTarget.value)}
                  />
                  <span class="fate">{fate(line, plan.part)}</span>
                </li>
              {/each}
            </ul>
          </div>
        {/if}

        {#each facts.ambiguities as mark (mark.id)}
          <!--
            A group rather than a landmark, and named by the notes it stands
            over: four clusters asking the same question are four different
            questions, and a reader moving between them has to hear which.
          -->
          <div
            class="mark"
            class:open={mark.id === open}
            role="group"
            aria-label={`${mark.explanation} — ${pitches(mark)}`}
            id={`mark-${mark.id}`}
          >
            <button type="button" class="text head" aria-expanded={mark.id === open} onclick={() => (open = mark.id)}>
              {mark.explanation}
            </button>
            {#if mark.id === open}
              <p class="affected">
                {mark.notes.length === 1 ? "1 note" : `${mark.notes.length} notes`}:
                {pitches(mark)}
              </p>
              {#if mark.choices.length === 0}
                <p class="affected">Tap a few beats while you listen, then read them.</p>
              {/if}
              <ul class="choices">
                {#each mark.choices as choice (choice.id)}
                  <li>
                    <button
                      type="button"
                      class="text"
                      aria-pressed={choice.current}
                      onclick={() => choose(mark.id, choice.id)}
                    >
                      {choice.label}
                    </button>
                  </li>
                {/each}
              </ul>
              {#if mark.kind === "onsetGroup"}
                <div class="contextual" role="group" aria-label="What this cluster is">
                  <button
                    type="button"
                    class="text"
                    onclick={() => act({ kind: "makeChord", group: markGroup(mark.id) })}
                  >
                    Make chord
                  </button>
                  <button type="button" class="text" onclick={() => act({ kind: "split", group: markGroup(mark.id) })}>
                    Keep rolled
                  </button>
                </div>
              {/if}
              {#if mark.kind === "writtenEnd" && mark.notes.length === 1}
                <div class="contextual" role="group" aria-label="Where this note ends">
                  <button
                    type="button"
                    class="text"
                    onclick={() => act({ kind: "tie", note: mark.notes[0] ?? 0, tied: true })}
                  >
                    Tie
                  </button>
                  <button
                    type="button"
                    class="text"
                    onclick={() => act({ kind: "tie", note: mark.notes[0] ?? 0, tied: false })}
                  >
                    Do not tie
                  </button>
                </div>
              {/if}
              {#if mark.kind === "voice"}
                <div class="contextual" role="group" aria-label="Assign a line">
                  {#each [0, 1, 2, 3] as voice (voice)}
                    <button
                      type="button"
                      class="text"
                      onclick={() => act({ kind: "assignVoice", notes: mark.notes, voice })}
                    >
                      Line {voice + 1}
                    </button>
                  {/each}
                </div>
              {/if}
            {/if}
          </div>
        {/each}

        {#if selection.length > 0}
          <div class="mark open" role="group" aria-label="What to do with the selection">
            <p class="affected">
              {selection.length === 1 ? "1 note" : `${selection.length} notes`} selected.
            </p>
            {#if transposing}
              <form
                class="interval"
                onsubmit={(event) => {
                  event.preventDefault();
                  typedInterval();
                }}
              >
                <label>
                  Interval
                  <input type="text" bind:this={field} bind:value={interval} placeholder="up P5" />
                </label>
                <button type="submit" class="text">Transpose</button>
              </form>
            {:else}
              <button type="button" class="text" onclick={() => (transposing = true)}>Transpose…</button>
            {/if}
          </div>
        {/if}

        {#if facts.losses.length > 0}
          <div class="mark open" role="group" aria-label="What could not be written">
            <ul class="choices">
              {#each facts.losses as loss (loss)}
                <li class="loss">{loss}</li>
              {/each}
            </ul>
          </div>
        {/if}

        {#if facts.history.length > 0}
          <div class="mark open" role="group" aria-label="What you decided">
            <ol class="choices">
              {#each facts.history as step, at (`${at}-${step}`)}
                <li class="loss">{step}</li>
              {/each}
            </ol>
          </div>
        {/if}

        {#if facts.source !== null}
          <div class="mark open" role="group" aria-label="The source this would write">
            <pre class="source">{facts.source}</pre>
          </div>
        {/if}
      </div>
    </Margin>
  </div>
</div>

<style>
  .workspace {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
    min-height: 0;
  }

  .body {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(14rem, 20rem);
    gap: var(--s-3);
    min-height: 0;
    padding: var(--s-3);
  }

  .identity {
    display: flex;
    align-items: baseline;
    gap: var(--s-3);
    flex-wrap: wrap;
  }

  .title {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
  }

  .where,
  .stale,
  .kept,
  .settled,
  .affected,
  .fate,
  .loss {
    margin: 0;
    font-size: 0.85rem;
    color: var(--ink-muted);
  }

  /*
   * A refusal is louder than the rest of the margin because it is the one
   * line that changes what to do next: everything else here describes what
   * is, and this says why nothing happened (`05-states.md` §3).
   */
  .refused {
    margin: 0;
    font-size: 0.85rem;
    color: var(--chalk);
  }

  .line {
    display: grid;
    grid-template-columns: auto minmax(4rem, 1fr);
    align-items: baseline;
    column-gap: var(--s-2);
  }

  .line input {
    font: inherit;
    font-size: 0.85rem;
    color: var(--ink);
    background: var(--leaf);
    border: 1px solid var(--rule);
    border-radius: var(--radius-control);
    padding: 0 var(--s-1);
    min-width: 0;
  }

  .line .fate {
    grid-column: 1 / -1;
  }

  .verbs {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    flex-wrap: wrap;
    padding-top: var(--s-2);
  }

  .hearing {
    display: flex;
    gap: var(--s-2);
  }

  /*
   * A control whose symbol an engraver does not already draw is set as a
   * word (`01-visual-language.md` §4). Every screen states this for itself
   * because Svelte scopes styles to the component that renders them.
   */
  .text {
    background: none;
    border: 0;
    border-radius: var(--radius-control);
    padding: var(--s-1) var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
    cursor: pointer;
    text-align: left;
  }

  .text:hover:not(:disabled) {
    color: var(--ink);
  }

  .text:disabled {
    color: var(--ink-faint);
    cursor: default;
  }

  /* The reading that is in force is stated, not merely available. */
  .text[aria-pressed="true"] {
    color: var(--ink);
    box-shadow: inset 0 -1px 0 var(--plate);
  }

  .stage {
    display: grid;
    min-width: 0;
    min-height: 0;
  }

  /* The leaf is the page: it takes the stage, and the phrase sits at its top. */
  .paper {
    position: relative;
    padding: var(--s-4) var(--s-3);
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    gap: var(--s-4);
    height: 100%;
    overflow-y: auto;
  }

  .barline {
    position: absolute;
    top: var(--s-4);
    bottom: auto;
    height: calc(100% - 2 * var(--s-4));
    width: 1px;
    background: var(--rule);
  }

  .stave {
    position: relative;
    height: 2.5rem;
  }

  /* A staff line is a rule, not an accent: sanguine is for what is refused. */
  .rule {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
    background: var(--rule);
  }

  .note {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    height: 1.6rem;
    min-width: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    /*
     * A notehead is ink on paper. The accent is a fill for chrome, and a note
     * drawn in it is a note whose pitch name cannot be read against it
     * (`01-visual-language.md` §2).
     */
    background: var(--leaf);
    border: 0;
    border-bottom: 1px solid var(--ink);
    border-radius: 0;
    color: var(--ink);
    font-family: var(--f-score-text);
    font-size: var(--t-small-size);
    cursor: pointer;
  }

  .note.selected {
    background: var(--plate-wash);
    border-color: var(--plate);
  }

  /* A question, not a problem: the mark is an underline, never a red box. */
  .note.asked {
    border-bottom: 3px double var(--chalk);
  }

  .note.pedal .head::after {
    content: "‸";
  }

  .note.grace {
    font-size: 0.65rem;
  }

  .asks {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    overflow-y: auto;
  }

  .mark {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
  }

  .mark .head {
    text-align: left;
  }

  .choices {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
  }

  .contextual {
    display: flex;
    gap: var(--s-2);
    flex-wrap: wrap;
  }

  .interval {
    display: flex;
    align-items: baseline;
    gap: var(--s-2);
  }

  .source {
    margin: 0;
    font-size: 0.75rem;
    white-space: pre-wrap;
    overflow-x: auto;
    color: var(--ink-muted);
  }
</style>
