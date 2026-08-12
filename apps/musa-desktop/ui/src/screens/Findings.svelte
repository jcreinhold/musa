<script lang="ts">
  /**
   * What an analysis saw (`08-elaboration.md` §5).
   *
   * A reading, not a verdict. Nothing here is a diagnostic, nothing is red,
   * and nothing is styled as an error — a cadence with two of its three kinds
   * of evidence is a finding with an absence recorded, and an interface that
   * painted it like a syntax error would be answering a question nobody asked
   * with a confidence nobody has.
   *
   * Every sentence is the core's: the method, the assumptions, the summary,
   * the standing word, and what each ground cites. This screen chooses the
   * order and nothing else (`03-interaction.md` §7).
   */
  import Position from "../lib/ui/Position.svelte";
  import Ticked from "../lib/ui/Ticked.svelte";
  import type { AnalysisFacts, FindingFacts, NoteFacts, Span } from "../lib/state/snapshot";

  let {
    report = null,
    reading = null,
    revision = null,
    kinds,
    onask,
    onselect,
    onreveal,
  }: {
    /** The last reading, or null while none has been asked for. */
    report?: AnalysisFacts | null;
    /** Which reading is in flight, so the panel can say it is reading. */
    reading?: string | null;
    /** The score revision on screen, so an older reading can say it is one. */
    revision?: number | null;
    /** The analyses this compiler runs, by their own names. */
    kinds: readonly { kind: string; method: string }[];
    /** Ask for one. Absent while the session is not live. */
    onask?: (kind: string) => void;
    /** Select the notes a finding points at. */
    onselect?: (events: string[]) => void;
    /** Open the source at a span. */
    onreveal?: (span: Span) => void;
  } = $props();

  /** The notes one finding's evidence names, if it names any. */
  function notesOf(finding: FindingFacts): NoteFacts[] {
    const evidence = finding.evidence;
    if (evidence.kind === "event") return [evidence];
    if (evidence.kind === "passage") return evidence.notes;
    return [];
  }

  /** Where the evidence is written, when it is written anywhere. */
  function spanOf(finding: FindingFacts): Span | null {
    const evidence = finding.evidence;
    if (evidence.kind === "event") return evidence.span;
    if (evidence.kind === "annotation") return evidence.span;
    if (evidence.kind === "passage") return evidence.notes[0]?.span ?? null;
    return null;
  }

  /**
   * What the evidence is, in a few words.
   *
   * A value in force is the one kind that is not a place, and it says so
   * rather than being silently unclickable.
   */
  function evidenceOf(finding: FindingFacts): string {
    const evidence = finding.evidence;
    switch (evidence.kind) {
      case "event":
        return `${evidence.part}, ${evidence.voice}`;
      case "passage":
        return `${evidence.notes.length} ${evidence.notes.length === 1 ? "note" : "notes"}`;
      case "annotation":
        return "written above the staff";
      case "inForce":
        return "in force here";
    }
  }

  /**
   * Whether the reading is of a score that has since been recompiled.
   *
   * It stays on screen — a panel that emptied itself on every keystroke would
   * be answering a question nobody asked with silence — and says what it is
   * (`05-states.md` §4, `08-elaboration.md` §8). Unknown revisions are not
   * stale: an absent number is no evidence of one.
   */
  const stale = $derived(report !== null && revision !== null && report.revision < revision);

  function show(finding: FindingFacts): void {
    const notes = notesOf(finding);
    if (notes.length > 0) onselect?.(notes.map((note) => note.event));
    const span = spanOf(finding);
    if (span) onreveal?.(span);
  }
</script>

<section class="findings" aria-label="Analysis">
  <!--
    Asking is explicit. Analysis costs real work and states assumptions the
    reader has to agree to, so it never runs on a keystroke and never runs on
    a timer (`08-elaboration.md` §5).
  -->
  <div class="ask" role="group" aria-label="Read the score">
    {#each kinds as offered (offered.kind)}
      <button
        type="button"
        class="kind"
        class:current={report !== null && reading === null && report.kind === offered.kind}
        aria-busy={reading === offered.kind}
        title={offered.method}
        disabled={onask === undefined}
        onclick={() => onask?.(offered.kind)}>{offered.kind}</button
      >
    {/each}
  </div>

  {#if report}
    <!--
      The method and the assumptions come first, because a reader who does not
      accept them can stop there. That is the whole reason they are printed.
    -->
    {#if stale}
      <p class="stale" role="status">Read before the last change. Ask again to read the score as it stands.</p>
    {/if}
    <p class="method">{report.method}</p>
    {#if report.profile}
      <p class="profile">against {report.profile}</p>
    {/if}
    {#if report.assumptions.length > 0}
      <ul class="assumptions">
        {#each report.assumptions as assumption, index (index)}
          <li><Ticked text={assumption} /></li>
        {/each}
      </ul>
    {/if}

    {#if report.findings.length === 0}
      <!-- An answer, not an empty pane: somebody asked this question. -->
      <p class="nothing">Nothing found.</p>
    {:else}
      <ul class="list">
        {#each report.findings as finding, index (index)}
          <li>
            <button type="button" class="finding" onclick={() => show(finding)}>
              <span class="summary"><Ticked text={finding.summary} /></span>
              <span class="where"><Position bar={finding.bar} beat={finding.beat} size="value" /></span>
            </button>
            <p class="standing">
              <span class="word">{finding.standing}</span>
              <span class="evidence">{evidenceOf(finding)}</span>
            </p>
            {#if finding.rule}
              <p class="rule">
                <Ticked text={finding.rule.states} />
                <span class="cites">{finding.rule.cites}</span>
              </p>
            {/if}
            {#if finding.grounds.length > 0}
              <ul class="grounds">
                {#each finding.grounds as ground, at (at)}
                  <!--
                    An unsatisfied ground is shown, not hidden: the absence is
                    the information, and it is marked by a word and a shape
                    rather than by colour alone (`03-interaction.md` §5).
                  -->
                  <li class:missing={!ground.satisfied}>
                    <span class="glyph" aria-hidden="true">{ground.satisfied ? "▪" : "▫"}</span>
                    <span class="criterion">{ground.criterion}</span>
                    <span class="held">{ground.satisfied ? "" : "not shown"}</span>
                    <span class="cites">{ground.cites}</span>
                  </li>
                {/each}
              </ul>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .findings {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
    padding: var(--s-4) var(--s-5);
  }

  .ask {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-3);
  }

  /*
   * A reading is asked for by name, and the names are the compiler's. They set
   * in mono because they are words the composer also types on a command line.
   */
  .kind {
    font-family: var(--f-mono);
    font-size: var(--t-micro-size);
    color: var(--ink-muted);
    background: none;
    border: 0;
    border-bottom: 1px solid var(--rule);
    padding: 0 0 1px;
    cursor: pointer;
  }

  .kind:hover:not(:disabled) {
    color: var(--ink);
  }

  .kind.current {
    color: var(--plate);
    border-bottom-color: var(--plate);
  }

  .kind:disabled {
    cursor: default;
    opacity: 0.5;
  }

  .method,
  .profile,
  .nothing,
  .rule,
  .stale {
    margin: 0;
  }

  /*
   * A reading of a score that has changed. The words carry it — a reading is
   * not an error and has no severity to borrow a colour from — and the rule
   * above it is the same seam a stale leaf shows (`05-states.md` §4).
   */
  .stale {
    border-top: 1px solid var(--chalk);
    padding-top: var(--s-2);
    color: var(--ink);
  }

  .assumptions {
    margin: 0;
    padding-left: var(--s-4);
    color: var(--ink-faint);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: var(--s-4);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .finding {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-3);
    width: 100%;
    font-family: inherit;
    font-size: inherit;
    color: var(--ink);
    background: none;
    border: 0;
    padding: 0;
    cursor: pointer;
    text-align: left;
  }

  .finding:hover .summary {
    color: var(--plate);
  }

  .where {
    color: var(--ink-faint);
    white-space: nowrap;
  }

  .standing {
    display: flex;
    gap: var(--s-3);
    margin: 0;
  }

  /*
   * How a finding stands is a word. Not a colour, not a bar, not a percentage
   * — the reading does not have that precision and must not look as if it does.
   */
  .word {
    font-style: italic;
    color: var(--ink);
  }

  .evidence,
  .cites {
    color: var(--ink-faint);
  }

  .grounds {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .grounds li {
    display: flex;
    gap: var(--s-2);
  }

  .grounds .glyph {
    color: var(--ink-faint);
  }

  .grounds .missing .criterion {
    font-style: italic;
  }

  .held {
    color: var(--ink-faint);
  }
</style>
