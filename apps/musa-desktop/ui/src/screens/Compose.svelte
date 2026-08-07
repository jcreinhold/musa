<script lang="ts">
  /**
   * The Compose workspace: leaf and margin, exactly the arrangement of
   * `01-visual-language.md` §7.
   *
   * In this prototype the transport is present and disabled, and playback,
   * the playhead, the keyboard map, and Origin view are deliberately absent —
   * they belong to later prompts. Selection and hover are here, because they
   * are pure frontend and they are what proves the overlay layer and the
   * staff-space unit.
   */
  import { untrack } from "svelte";

  import Score from "../lib/score/Score.svelte";
  import { ZOOM_STEPS } from "../lib/engrave/options";
  import Leaf from "../lib/ui/Leaf.svelte";
  import Margin from "../lib/ui/Margin.svelte";
  import GlyphButton from "../lib/ui/GlyphButton.svelte";
  import TransportReadout from "../lib/ui/TransportReadout.svelte";
  import { REPEAT_RIGHT_LEFT } from "../lib/ui/glyphs";
  import { Workspace } from "../lib/state/selection.svelte";
  import type { ProjectSnapshot } from "../lib/state/snapshot";
  import Drawer from "./Drawer.svelte";
  import Inspector from "./Inspector.svelte";
  import PartsList from "./PartsList.svelte";

  let { snapshot }: { snapshot: ProjectSnapshot } = $props();

  // The snapshot is a fixture here and never changes; prompt 21 replaces this
  // with a session that pushes new ones.
  const workspace = untrack(() => new Workspace(snapshot));
  const score = $derived(snapshot.score);
  const focused = $derived(workspace.focused);

  let zoomStep = $state(ZOOM_STEPS.indexOf(100));
  let drawerOpen = $state(false);
  const zoom = $derived(ZOOM_STEPS[zoomStep] ?? 100);

  function stepZoom(by: number): void {
    zoomStep = Math.min(Math.max(zoomStep + by, 0), ZOOM_STEPS.length - 1);
  }
</script>

{#if score}
  <div class="workspace">
    <Margin side="top">
      <h1 class="title">{score.title}</h1>
      <div class="controls">
        <div class="transport">
          <button type="button" class="text" disabled>Play</button>
          <button type="button" class="text" disabled>Stop</button>
          <GlyphButton glyph={REPEAT_RIGHT_LEFT} label="Loop the selection" disabled />
        </div>
        <TransportReadout
          {score}
          playback={snapshot.playback}
          bar={focused?.bar ?? 1}
          beat={focused?.beat ?? { numerator: 1, denominator: 1 }}
        />
        <div class="zoom">
          <button type="button" class="text" onclick={() => stepZoom(-1)} aria-label="Zoom out"
            >−</button
          >
          <span class="level">{zoom}&thinsp;%</span>
          <button type="button" class="text" onclick={() => stepZoom(1)} aria-label="Zoom in"
            >+</button
          >
        </div>
      </div>
    </Margin>

    <div class="body">
      <Margin side="left" label="Parts">
        <PartsList parts={score.parts} {workspace} />
      </Margin>

      <div class="stage">
        <Leaf>
          <Score
            mei={snapshot.mei ?? ""}
            revision={snapshot.scoreRevision ?? snapshot.revision}
            {zoom}
            {workspace}
          />
        </Leaf>
      </div>

      <Margin side="right" label="Inspector">
        <Inspector event={focused} />
      </Margin>
    </div>

    <Margin side="bottom">
      <Drawer source={snapshot.source} diagnostics={snapshot.diagnostics} bind:open={drawerOpen} />
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

  .controls {
    display: flex;
    align-items: center;
    gap: var(--s-6);
  }

  .transport,
  .zoom {
    display: flex;
    align-items: center;
    gap: var(--s-1);
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
