<script lang="ts">
  /**
   * The Source workspace (roadmap §14.4): the text and the page, side by
   * side, because the whole claim of a notation-first language is that they
   * are two views of one document.
   *
   * Source on the left, leaf on the right, problems beneath the source where
   * the caret they move is — the same column, on the same material, behind
   * the same hairline as in Compose. What this workspace changes is the
   * proportion, not the arrangement: the score's margins retire and the page
   * takes everything the text does not (`01-visual-language.md` §8).
   *
   * Nothing here is computed: the highlighting comes from the language
   * package, the diagnostics from the compiler, and which notes a caret
   * position belongs to is the workspace's answer, not this screen's.
   */
  import Score from "../lib/score/Score.svelte";
  import type { ViewMode } from "musa-engrave";
  import Leaf from "../lib/ui/Leaf.svelte";
  import Margin from "../lib/ui/Margin.svelte";
  import SourcePane from "../lib/ui/SourcePane.svelte";
  import Findings from "./Findings.svelte";
  import ANALYSES from "../lib/session/generated/analysis-kinds.json";
  import Workspaces from "../lib/ui/Workspaces.svelte";
  import type { Screen } from "../lib/commands/map";
  import { SOURCE_FLOOR, type Preferences } from "../lib/session/preferences.svelte";
  import type { Session } from "../lib/session/session.svelte";
  import type { Workspace } from "../lib/state/selection.svelte";
  import type { Reveal } from "../lib/state/reveal";
  import type { Focus } from "../lib/state/focus.svelte";
  import type { Candidate } from "../lib/state/gesture.svelte";
  import { volumeOf, type Diagnostic, type Span } from "../lib/state/snapshot";

  let {
    session,
    preferences,
    workspace,
    zoom,
    mode,
    reveal = null,
    origin = false,
    focus,
    sounding = [],
    onvisible,
    candidate = null,
    onedit,
    oncandidate,
    oncaret,
    ondiagnostic,
    onreveal,
    onshow,
  }: {
    session: Session;
    /** Text size and vim mode, for the source column. */
    preferences: Preferences;
    workspace: Workspace;
    zoom: number;
    mode: ViewMode;
    /** A place to put the caret, once, when it changes. */
    reveal?: Reveal | null;
    origin?: boolean;
    /** The shared focus, marked in both panes at once. */
    focus: Focus;
    /** Where the music on the visible pages was written, as source spans. */
    sounding?: Span[];
    /** The score reports what it has engraved on screen. */
    onvisible?: (ids: string[]) => void;
    /** What a pointer gesture in flight would write, drawn in the text. */
    candidate?: { start: number; end: number; text: string } | null;
    /** A gesture came up on the preview: write it. */
    onedit?: (candidate: Candidate) => void;
    /** A gesture moved: ask what it would write. */
    oncandidate?: (candidate: Candidate | null) => void;
    /** The caret moved; the score follows it (§14.4's other direction). */
    oncaret?: (offset: number) => void;
    ondiagnostic?: (diagnostic: Diagnostic) => void;
    /** Open the source at a span: what a finding's evidence reaches. */
    onreveal?: (span: Span) => void;
    onshow: (which: Screen) => void;
  } = $props();

  const snapshot = $derived(session.snapshot);
  /** Whether the switcher offers the contents page (`07-the-volume.md`). */
  const volume = $derived(volumeOf(snapshot) !== null);
  /** What the text marks: where the expansion in view came from. */
  const highlight = $derived(workspace.sourceSpans(origin));
  const diagnostics = $derived(snapshot?.diagnostics ?? []);

  /** The narrowest a leaf is still a page, as in Compose. */
  const STAGE_MIN = 320;

  /** The stage, so the seam can ask how much of it is still spare. */
  let stage: HTMLElement | null = $state(null);

  /**
   * The bundled module being read, when one is (`08-elaboration.md` §3).
   *
   * It replaces the text and nothing else: the page keeps showing the piece,
   * because the module is not a score and the composer has not left theirs.
   */
  const library = $derived(session.library);

  /** Where in the module the term was, restated by the core in its measure. */
  const libraryReveal = $derived(library?.span ? { span: library.span, focus: true } : null);
</script>

<!--
  A reading of the piece, under the problems with it. Two different things
  share this column and the design is keeping them apart: a diagnostic says
  something is wrong, a finding says something was seen (`08-elaboration.md`
  §5).
-->
{#snippet footer()}
  {#if library === null}
    <Findings
      report={session.report}
      reading={session.reading}
      revision={snapshot?.scoreRevision ?? snapshot?.revision ?? null}
      kinds={ANALYSES}
      onask={session.live ? (kind) => void session.analyze(kind) : undefined}
      onselect={(events) => (workspace.selection = { kind: "event", events })}
      {onreveal}
    />
  {/if}
{/snippet}

{#if snapshot}
  <div class="source-workspace">
    <Margin side="top">
      <div class="identity">
        <h1 class="title">{snapshot.score?.title ?? ""}</h1>
        <Workspaces current="source" {volume} {onshow} />
      </div>
      {#if session.notice}
        <p class="notice" class:failure={session.notice.tone === "failure"} role="status">
          {session.notice.message}
        </p>
      {/if}
    </Margin>

    <div class="panes">
      <SourcePane
        source={library?.text ?? session.text}
        editable={session.live && library === null}
        library={library && { name: library.name }}
        onclose={library ? () => session.closeLibrary() : undefined}
        onlibrary={(uri, start, end) => void session.openLibrary(uri, start, end)}
        terms={snapshot.terms}
        names={snapshot.names}
        diagnostics={library ? [] : diagnostics}
        {highlight}
        reveal={library ? libraryReveal : reveal}
        {footer}
        focus={focus.marked}
        {sounding}
        {candidate}
        onpoint={(line) => focus.pointLine(line)}
        modal={preferences.vim}
        onedit={(text) => session.edit(text)}
        {oncaret}
        onundo={() => void session.undo()}
        onredo={() => void session.redo()}
        onsave={() => void session.save()}
        {ondiagnostic}
        width={preferences.sourceWidth}
        floor={SOURCE_FLOOR}
        spare={() => (stage?.clientWidth ?? STAGE_MIN) - STAGE_MIN}
        onwiden={(width) => preferences.widenSource(width)}
        onreset={() => preferences.resetSource()}
      />

      <main class="stage" class:continuous={mode === "continuous"} bind:this={stage}>
        <Leaf stale={session.stale}>
          <Score
            mei={snapshot.mei ?? ""}
            revision={snapshot.scoreRevision ?? snapshot.revision}
            {zoom}
            {mode}
            {workspace}
            {origin}
            {focus}
            {onvisible}
            onedit={session.live ? onedit : undefined}
            oncandidate={session.live ? oncandidate : undefined}
          />
        </Leaf>
      </main>
    </div>
  </div>
{:else}
  <Margin side="bottom"><p class="empty">Nothing open.</p></Margin>
{/if}

<style>
  .source-workspace {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
    min-height: 0;
    max-width: 100%;
    overflow-x: hidden;
    background: var(--surround);
  }

  /*
   * Source left, page right: the claim of the workspace is that they are two
   * views of one document, so neither is a panel beside the other.
   *
   * The text takes its measure and the page takes the rest (§8). A fraction
   * would give the text a width that has nothing to do with how long its
   * lines are — which it did, and the right third of every file was empty.
   */
  .panes {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    min-height: 0;
    /* What a deliberate ask may take here: everything but a page.
       There are no margins in this workspace to protect. */
    --source-room: max(300px, calc(100vw - 320px));
  }

  .identity {
    display: flex;
    align-items: baseline;
    gap: var(--s-5);
    min-width: 0;
  }

  .title {
    margin: 0;
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    font-weight: 400;
    color: var(--ink);
  }

  .notice {
    margin: 0;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  .notice.failure {
    color: var(--chalk);
  }

  /*
   * The page is a page here too. Compose already shapes the leaf to the
   * paper, and the claim of this workspace is that it is the same screen at a
   * different proportion — a leaf that changed shape when the margins retired
   * would make that a lie.
   */
  .stage {
    display: flex;
    justify-content: center;
    min-width: 0;
    min-height: 0;
    overflow: auto;
    padding: var(--s-5);
  }

  .stage :global(> *) {
    height: 100%;
    aspect-ratio: 210 / 297;
    max-width: 100%;
    min-width: 0;
  }

  /* Continuous is not a page, so the leaf stops pretending to be one (§4). */
  .stage.continuous :global(> *) {
    aspect-ratio: auto;
    width: 100%;
  }

  .empty {
    margin: 0;
    color: var(--ink-muted);
  }
</style>
