<script lang="ts">
  /**
   * The Compose workspace: leaf and margin, exactly the arrangement of
   * `01-visual-language.md` §7.
   *
   * The score, the playhead, and the keyboard map belong to later prompts.
   * What is here is everything the session can already answer: the piece, its
   * problems, its transport, and the source you edit it through.
   */
  import { untrack } from "svelte";

  import Score from "../lib/score/Score.svelte";
  import type { ViewMode } from "../lib/engrave/options";
  import Leaf from "../lib/ui/Leaf.svelte";
  import Margin from "../lib/ui/Margin.svelte";
  import GlyphButton from "../lib/ui/GlyphButton.svelte";
  import TransportReadout from "../lib/ui/TransportReadout.svelte";
  import { REPEAT_RIGHT_LEFT } from "../lib/ui/glyphs";
  import type { Session } from "../lib/session/session.svelte";
  import { Workspace } from "../lib/state/selection.svelte";
  import type { ProjectSnapshot } from "../lib/state/snapshot";
  import Drawer from "./Drawer.svelte";
  import Inspector from "./Inspector.svelte";
  import PartsList from "./PartsList.svelte";

  let {
    session,
    zoom,
    mode,
    onzoom,
    onpinch,
    onmode,
  }: {
    session: Session;
    zoom: number;
    mode: ViewMode;
    onzoom: (by: number) => void;
    onpinch: (factor: number) => void;
    onmode: (mode: ViewMode) => void;
  } = $props();

  // Read through, never copied: the session replaces the snapshot on every
  // revision, and a selection is only meaningful against the current one.
  const workspace = new Workspace(() => session.snapshot as ProjectSnapshot);

  const snapshot = $derived(session.snapshot);
  const score = $derived(snapshot?.score ?? null);
  const focused = $derived(workspace.focused);
  const problems = $derived(snapshot?.diagnostics.filter((d) => d.severity === "error") ?? []);

  // A new score is a new set of events. The selection is by id and usually
  // survives it untouched; when the note it was on is gone, this is what moves
  // it to a neighbour and tells the inspector to say so (`02-engraving.md` §6).
  $effect(() => {
    void snapshot?.scoreRevision;
    untrack(() => workspace.reconcile());
  });
</script>

{#if snapshot && score}
  <div class="workspace">
    <Margin side="top">
      <h1 class="title">{score.title}</h1>

      <!--
        The top margin carries whatever the interface currently has to say:
        a completed operation for three seconds, a failure until it is
        superseded, and — the one that matters — how far behind the page is
        while the source has problems (`05-states.md` §4).
      -->
      {#if session.notice}
        <p class="notice" class:failure={session.notice.tone === "failure"} role="status">
          {session.notice.message}
        </p>
      {:else if session.stale && session.shownRevision !== null}
        <p class="notice stale" role="status">
          Showing revision {session.shownRevision} — the current source has {problems.length}
          {problems.length === 1 ? "problem" : "problems"}
        </p>
      {/if}

      <div class="controls">
        <div class="transport">
          <button
            type="button"
            class="text"
            disabled={!session.live}
            onclick={() => void session.play()}>Play</button
          >
          <button
            type="button"
            class="text"
            disabled={!session.live || !snapshot.playback.playing}
            onclick={() => void session.stop()}>Stop</button
          >
          <GlyphButton glyph={REPEAT_RIGHT_LEFT} label="Loop the selection" disabled />
        </div>
        <TransportReadout
          {score}
          playback={snapshot.playback}
          stale={session.stale}
          bar={focused?.bar ?? 1}
          beat={focused?.beat ?? { numerator: 1, denominator: 1 }}
        />
        <!--
          Two words, one of them current: page or continuous (§4). A toggle
          named for what it shows, rather than a pair of icons a reader has to
          learn.
        -->
        <div class="view" role="group" aria-label="View">
          <button
            type="button"
            class="text"
            aria-pressed={mode === "page"}
            onclick={() => onmode("page")}>Pages</button
          >
          <button
            type="button"
            class="text"
            aria-pressed={mode === "continuous"}
            onclick={() => onmode("continuous")}>Continuous</button
          >
        </div>

        <div class="zoom">
          <button type="button" class="text" onclick={() => onzoom(-1)} aria-label="Zoom out"
            >−</button
          >
          <span class="level">{zoom}&thinsp;%</span>
          <button type="button" class="text" onclick={() => onzoom(1)} aria-label="Zoom in"
            >+</button
          >
        </div>
      </div>
    </Margin>

    <div class="body">
      <Margin side="left" label="Parts">
        <PartsList parts={score.parts} {workspace} />
      </Margin>

      <div class="stage" class:continuous={mode === "continuous"}>
        <Leaf stale={session.stale}>
          <Score
            mei={snapshot.mei ?? ""}
            revision={snapshot.scoreRevision ?? snapshot.revision}
            {zoom}
            {mode}
            {workspace}
            {onpinch}
          />
        </Leaf>
      </div>

      <Margin side="right" label="Inspector">
        <Inspector event={focused} adrift={workspace.adrift} />
      </Margin>
    </div>

    <Margin side="bottom">
      <Drawer
        source={session.text}
        editable={session.live}
        diagnostics={snapshot.diagnostics}
        onedit={(text) => session.edit(text)}
        bind:open={session.drawerOpen}
      />
    </Margin>
  </div>
{/if}

<style>
  .workspace {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    height: 100%;
  }

  .title {
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    font-weight: 400;
    margin: 0;
    color: var(--ink);
  }

  .notice {
    margin: 0;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  .notice.stale,
  .notice.failure {
    color: var(--chalk);
  }

  .controls {
    display: flex;
    align-items: center;
    gap: var(--s-6);
  }

  .transport,
  .view,
  .zoom {
    display: flex;
    align-items: center;
    gap: var(--s-1);
  }

  /* The current view is the one set in ink; the other is an offer. */
  .text[aria-pressed="true"] {
    color: var(--ink);
  }

  /*
   * Transport and zoom are words, not pictograms. Musa ships no icon font of
   * its own, and the thesis rejects an icon toolbar: a control whose symbol
   * an engraver does not already draw is set as a word.
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
  }

  .text:hover:not(:disabled) {
    color: var(--ink);
  }

  .text:disabled {
    color: var(--ink-faint);
    cursor: default;
  }

  .level {
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    font-variant-numeric: tabular-nums;
    color: var(--ink-faint);
    min-width: 5ch;
    text-align: center;
  }

  .body {
    display: grid;
    grid-template-columns: minmax(0, 200px) minmax(0, 1fr) minmax(0, 240px);
    min-height: 0;
  }

  /*
   * Page view is the default, so the leaf is a page: an upright sheet of
   * portrait proportions, centred, with the surround visible around it. A
   * leaf stretched to the window is a text editor with staves in it — the
   * music would be laid out across a metre-wide sheet and read as a ribbon
   * along the top.
   */
  .stage {
    display: flex;
    justify-content: center;
    min-width: 0;
    padding: 0 var(--s-4) var(--s-6);
  }

  .stage :global(> *) {
    height: 100%;
    aspect-ratio: 210 / 297;
    max-width: 100%;
    min-width: 0;
  }

  /*
   * Continuous is not a page, so the leaf stops pretending to be one: it
   * takes the full width and the music scrolls sideways through it (§4).
   */
  .stage.continuous :global(> *) {
    aspect-ratio: auto;
    width: 100%;
  }

  /*
   * Below 1100px the inspector collapses to the right margin only, and the
   * parts list narrows; the leaf never shrinks below a legible staff size.
   */
  @media (max-width: 1100px) {
    .body {
      grid-template-columns: minmax(0, 150px) minmax(0, 1fr) minmax(0, 200px);
    }

    .controls {
      gap: var(--s-4);
    }
  }
</style>
