<script lang="ts">
  /**
   * The Source workspace (roadmap §14.4): the text and the page, side by
   * side, because the whole claim of a notation-first language is that they
   * are two views of one document.
   *
   * Source on the left, leaf on the right, problems beneath the source where
   * the caret they move is. The drawer inside Compose stays what it is — a
   * quick look — and this is where the writing happens.
   *
   * Nothing here is computed: the highlighting comes from the language
   * package, the diagnostics from the compiler, and which notes a caret
   * position belongs to is the workspace's answer, not this screen's.
   */
  import Score from "../lib/score/Score.svelte";
  import type { ViewMode } from "../lib/engrave/options";
  import Leaf from "../lib/ui/Leaf.svelte";
  import Margin from "../lib/ui/Margin.svelte";
  import SourceEditor from "../lib/ui/SourceEditor.svelte";
  import Workspaces from "../lib/ui/Workspaces.svelte";
  import type { Session } from "../lib/session/session.svelte";
  import type { Workspace } from "../lib/state/selection.svelte";
  import type { Reveal } from "../lib/state/reveal";
  import type { Diagnostic } from "../lib/state/snapshot";

  let {
    session,
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
    workspace: Workspace;
    zoom: number;
    mode: ViewMode;
    /** A place to put the caret, once, when it changes. */
    reveal?: Reveal | null;
    origin?: boolean;
    /** The caret moved; the score follows it (§14.4's other direction). */
    oncaret?: (offset: number) => void;
    ondiagnostic?: (diagnostic: Diagnostic) => void;
    onshow: (which: "compose" | "source") => void;
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
    <section class="text" aria-label="Source">
      <SourceEditor
        source={session.text}
        editable={session.live}
        {diagnostics}
        {highlight}
        {reveal}
        onedit={(text) => session.edit(text)}
        {oncaret}
      />

      <!--
        Problems, beneath the text they are about. With none the pane shows
        nothing at all — not "0 problems" (`05-states.md` §2).
      -->
      {#if diagnostics.length > 0}
        <ul class="problems">
          {#each diagnostics as diagnostic, index (index)}
            <li>
              <button type="button" class="problem" onclick={() => ondiagnostic?.(diagnostic)}>
                <span class="glyph {diagnostic.severity}" aria-hidden="true"></span>
                <span class="message">{diagnostic.message}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <main class="stage">
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

  /* Source left, page right: the claim of the workspace is that they are two
     views of one document, so neither is a panel beside the other. */
  .panes {
    display: grid;
    grid-template-columns: minmax(0, 5fr) minmax(0, 7fr);
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
   * The text sits on its own surface, the page on the surround: the same
   * distinction the leaf makes, so the two halves read as two materials
   * rather than two panels (`01-visual-language.md` §7).
   */
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    padding: var(--s-4) var(--s-5);
    background: var(--surround-in);
    border-right: 1px solid var(--rule);
    overflow: hidden;
  }

  .stage {
    min-width: 0;
    min-height: 0;
    overflow: auto;
    padding: var(--s-5);
  }

  .problems {
    flex: 0 0 auto;
    max-height: 30%;
    margin: var(--s-3) 0 0;
    padding: var(--s-3) 0 0;
    border-top: 1px solid var(--rule);
    list-style: none;
    overflow: auto;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
  }

  /* Severity is a shape as well as a colour (`03-interaction.md` §5). */
  .glyph {
    display: inline-block;
    width: 7px;
    height: 7px;
    margin-right: var(--s-1);
  }

  .glyph.error {
    background: var(--chalk);
  }

  .glyph.warning {
    border: 1px solid var(--chalk);
  }

  .problem {
    display: block;
    width: 100%;
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }

  .problem:hover .message,
  .problem:focus-visible .message {
    text-decoration: underline;
  }

  .empty {
    margin: 0;
    color: var(--ink-muted);
  }
</style>
