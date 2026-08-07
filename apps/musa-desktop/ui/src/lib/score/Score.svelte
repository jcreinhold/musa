<script lang="ts">
  /**
   * The engraved page, its overlay layer, and the pointer model over both.
   *
   * This component owns no music. It asks the engrave module for a page, puts
   * it on the leaf, and translates pointer events into `EventId`s through the
   * `xml:id` contract. Everything it displays about a note comes from the
   * snapshot the core produced.
   */
  import { onMount, untrack } from "svelte";

  import { createEngraver, type Engraver, type Layout, type PageSvg } from "../engrave/engraver";
  import { pageFor } from "../engrave/options";
  import type { Workspace } from "../state/selection.svelte";
  import Overlay from "./Overlay.svelte";
  import { boxesFor, pad, type Rect } from "./geometry";
  import { eventIdOf } from "./ids";

  let {
    mei,
    revision,
    zoom,
    workspace,
  }: {
    mei: string;
    revision: number;
    zoom: number;
    /** Absent for a fixture shown purely as engraving. */
    workspace?: Workspace;
  } = $props();

  /** Selection halo padding, in staff spaces (`02-engraving.md` §8). */
  const HALO_SPACES = 0.6;

  let host = $state<HTMLDivElement | undefined>();
  let engraver: Engraver | undefined;
  let layout = $state<Layout | undefined>();
  let page = $state<PageSvg | undefined>();
  let selectionRects = $state<Rect[]>([]);
  let hoverRects = $state<Rect[]>([]);
  let width = $state(0);
  let height = $state(0);

  const box = $derived(page?.box ?? { width: 0, height: 0 });
  const staffSpace = $derived(layout?.staffSpace ?? 0);

  let loaded: string | undefined;
  let inFlight = 0;

  async function engrave(): Promise<void> {
    if (!engraver || width === 0 || height === 0) return;
    const size = pageFor(width, height, zoom);
    const key = `${revision}:${mei.length}`;
    const attempt = (inFlight += 1);
    const next =
      key === loaded
        ? await engraver.relayout({ ...size, zoom })
        : await engraver.load(mei, revision, { ...size, zoom });
    loaded = key;
    // A layout overtaken while it was computing is dropped rather than shown:
    // the caller has already asked for its replacement (`02-engraving.md` §2).
    if (attempt !== inFlight) return;
    const rendered = await engraver.page(1);
    if (attempt !== inFlight) return;
    // The old page stays on screen until the new one is in hand
    // (`02-engraving.md` §6); assigning here is the swap.
    layout = next;
    page = rendered;
  }

  onMount(() => {
    engraver = createEngraver();
    return () => engraver?.dispose();
  });

  $effect(() => {
    // Re-engrave when the score, the zoom, or the leaf's width changes.
    void mei;
    void revision;
    void zoom;
    void width;
    void height;
    void untrack(() => engrave());
  });

  /** Measure after every page swap: the boxes belong to this layout only. */
  $effect(() => {
    void page;
    const ids = workspace ? workspace.selected : [];
    const hovered = workspace?.hovered ?? null;
    const container = host;
    if (!container || !page) {
      selectionRects = [];
      hoverRects = [];
      return;
    }
    const unit = staffSpace;
    const grow = (rect: Rect) => pad(rect, HALO_SPACES, unit);
    selectionRects = ids.flatMap((id) => boxesFor(container, id)).map(grow);
    hoverRects = hovered && !ids.includes(hovered) ? boxesFor(container, hovered).map(grow) : [];
  });

  function onpointerdown(event: PointerEvent): void {
    if (!workspace) return;
    const id = eventIdOf(event.target as Element);
    if (id) workspace.select(id, event.shiftKey);
    else workspace.clear();
  }

  function onpointermove(event: PointerEvent): void {
    if (!workspace) return;
    workspace.hovered = eventIdOf(event.target as Element);
  }

  function onpointerleave(): void {
    if (workspace) workspace.hovered = null;
  }
</script>

<!--
  The score pane is a single tab stop; arrows navigate within it, because 4000
  tab stops is not accessibility (`03-interaction.md` §3). That single tab stop
  is exactly what the rule below flags; `application` is the correct role for a
  widget with its own internal navigation, and the stop is required, not
  incidental.
-->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="engraving"
  bind:this={host}
  bind:clientWidth={width}
  bind:clientHeight={height}
  role="application"
  aria-label="Engraved score"
  tabindex="0"
  {onpointerdown}
  {onpointermove}
  {onpointerleave}
>
  <div class="page">
    {#if page}
      <!-- The engrave module is the only producer of this markup, and it
           sanitized it on the way out of the worker (§3). -->
      {@html page.svg}
      <Overlay {box} {staffSpace} selection={selectionRects} hover={hoverRects} />
    {/if}
  </div>
</div>

<style>
  .engraving {
    height: 100%;
    overflow: auto;
    color: var(--ink);
  }

  .page {
    position: relative;
  }

  .engraving :global(svg) {
    display: block;
    width: 100%;
    height: auto;
  }

  .engraving :global(g) {
    fill: currentColor;
    stroke: currentColor;
  }

  /*
   * An event is clickable across its whole box, not only where it is inked.
   * The centre of a half note is a hole, and a composer who clicks the middle
   * of a notehead and gets nothing has been told the score is not really
   * there. `bounding-box` is what SVG2 provides for exactly this.
   */
  .engraving :global(g[id^="event-"]) {
    pointer-events: bounding-box;
  }
</style>
