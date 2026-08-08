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
  import type { ViewMode } from "../lib/engrave/options";
  import Leaf from "../lib/ui/Leaf.svelte";
  import Margin from "../lib/ui/Margin.svelte";
  import SourcePane from "../lib/ui/SourcePane.svelte";
  import Workspaces from "../lib/ui/Workspaces.svelte";
  import type { Screen } from "../lib/commands/map";
  import type { Preferences } from "../lib/session/preferences.svelte";
  import type { Session } from "../lib/session/session.svelte";
  import type { Workspace } from "../lib/state/selection.svelte";
  import type { Reveal } from "../lib/state/reveal";
  import type { Diagnostic } from "../lib/state/snapshot";

  let {
    session,
    preferences,
    workspace,
    zoom,
    mode,
    reveal = null,
    origin = false,
    oncaret,
    ondiagnostic,
    onshow,
  }: {
    session: Session;
    /** Text size and vim mode, for the source column (prompt 55). */
    preferences: Preferences;
    workspace: Workspace;
    zoom: number;
    mode: ViewMode;
    /** A place to put the caret, once, when it changes. */
    reveal?: Reveal | null;
    origin?: boolean;
    /** The caret moved; the score follows it (§14.4's other direction). */
    oncaret?: (offset: number) => void;
    ondiagnostic?: (diagnostic: Diagnostic) => void;
    onshow: (which: Screen) => void;
  } = $props();

  const snapshot = $derived(session.snapshot);
  /** What the text marks: where the expansion in view came from. */
  const highlight = $derived(workspace.sourceSpans(origin));
  const diagnostics = $derived(snapshot?.diagnostics ?? []);
</script>

{#if snapshot}
  <div class="source-workspace">
    <Margin side="top">
      <div class="identity">
        <h1 class="title">{snapshot.score?.title ?? ""}</h1>
        <Workspaces current="source" onshow={onshow} />
      </div>
      {#if session.notice}
        <p class="notice" class:failure={session.notice.tone === "failure"} role="status">
          {session.notice.message}
        </p>
      {/if}
    </Margin>

    <div class="panes">
      <SourcePane
        source={session.text}
        editable={session.live}
        {diagnostics}
        {highlight}
        {reveal}
        modal={preferences.vim}
        onedit={(text) => session.edit(text)}
        {oncaret}
        onundo={() => void session.undo()}
        onredo={() => void session.redo()}
        onsave={() => void session.save()}
        {ondiagnostic}
      />

      <main class="stage" class:continuous={mode === "continuous"}>
        <Leaf stale={session.stale}>
          <Score
            mei={snapshot.mei ?? ""}
            revision={snapshot.scoreRevision ?? snapshot.revision}
            {zoom}
            {mode}
            {workspace}
            {origin}
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
