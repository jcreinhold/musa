<script lang="ts">
  /**
   * The score pane: the pages, the overlay layer over them, and the pointer
   * model over both.
   *
   * This component owns no music. It asks the engrave module for a layout and
   * for pages, puts them on the leaf, and translates pointer events into
   * `EventId`s through the `xml:id` contract. Everything it displays about a
   * note comes from the snapshot the core produced.
   *
   * The three things that make it a surface to work in all day rather than a
   * viewer (`02-engraving.md` §§5–7):
   *
   * - a re-render never disrupts — the old pages stay on screen and stay
   *   interactive while the worker lays out the new revision, and the reader
   *   is anchored to an event id rather than to a pixel offset;
   * - zoom is a true re-layout, with a transient transform only for the
   *   duration of a pinch;
   * - pages render on demand, so a 300-page score does not hold 300 SVG trees.
   */
  import { onMount, untrack } from "svelte";

  import { mark } from "../perf";
  import { createEngraver, type Engraver, type Layout, type PageSvg } from "../engrave/engraver";
  import { modeOptions, pageFor, pixelsPerUnit, type ViewMode } from "../engrave/options";
  import type { Workspace } from "../state/selection.svelte";
  import Page from "./Page.svelte";
  import { boxesFor, pad, type Rect } from "./geometry";
  import { eventIdOf } from "./ids";

  let {
    mei,
    revision,
    zoom,
    mode = "page",
    workspace,
    onpinch,
  }: {
    mei: string;
    revision: number;
    zoom: number;
    mode?: ViewMode;
    /** Absent for a fixture shown purely as engraving. */
    workspace?: Workspace;
    /** A settled pinch, as a factor on the current zoom (§5). */
    onpinch?: (factor: number) => void;
  } = $props();

  /** Selection halo padding, in staff spaces (§8). */
  const HALO_SPACES = 0.6;

  /** How many pages either side of the visible ones stay resident (§7). */
  const NEIGHBOURS = 1;

  /** How long a pinch must settle before it becomes a real re-layout (§5). */
  const PINCH_SETTLE_MS = 120;

  let host = $state<HTMLDivElement | undefined>();
  let layout = $state<Layout | undefined>();
  let resident = $state<Map<number, PageSvg>>(new Map());
  let visible = $state<Set<number>>(new Set([1]));
  let selectionRects = $state<Rect[]>([]);
  let hoverRects = $state<Rect[]>([]);
  let width = $state(0);
  let height = $state(0);
  let gesture = $state(1);

  const pages = $derived(layout?.pages ?? 1);
  const box = $derived(layout?.box ?? { width: 0, height: 0 });
  const staffSpace = $derived(layout?.staffSpace ?? 0);
  /**
   * In page view the leaf fixes the width and the page's proportions give the
   * height. Continuous has no such anchor — the page is as wide as the music
   * turned out to be — so the scale is stated instead: the same CSS pixels per
   * page unit that page view would have used at this zoom (§4).
   */
  const scale = $derived(mode === "continuous" ? pixelsPerUnit(zoom) : undefined);
  /** What to keep in the document: what is on screen, plus a page either side. */
  const wanted = $derived(
    new Set(
      [...visible].flatMap((page) =>
        Array.from({ length: NEIGHBOURS * 2 + 1 }, (_, offset) => page - NEIGHBOURS + offset),
      ),
    ),
  );

  let engraver: Engraver | undefined;
  let loaded: string | undefined;
  let inFlight = 0;
  let observer: IntersectionObserver | undefined;
  let pinch: ReturnType<typeof setTimeout> | undefined;

  /**
   * The event at the top of the viewport, and where in the viewport it is.
   * Anchoring to this rather than to a scroll offset is what keeps an
   * inserted note from teleporting the reader (§6).
   */
  function anchor(): { id: string; offset: number } | undefined {
    const container = host;
    if (!container) return undefined;
    const top = container.getBoundingClientRect().top;
    for (const element of container.querySelectorAll<SVGGraphicsElement>('g[id^="event-"]')) {
      const at = element.getBoundingClientRect().top - top;
      if (at >= 0) return { id: element.id, offset: at };
    }
    return undefined;
  }

  /** Put the anchor back where it was, once the new layout has drawn it. */
  async function restore(held: { id: string; offset: number }): Promise<void> {
    const container = host;
    if (!container || !engraver) return;
    const page = await engraver.locate(held.id);
    if (page !== null) await ensure(page);
    await new Promise(requestAnimationFrame);
    const element = container.querySelector<SVGGraphicsElement>(`[id="${held.id}"]`);
    if (!element) return;
    const top = container.getBoundingClientRect().top;
    container.scrollTop += element.getBoundingClientRect().top - top - held.offset;
  }

  /** Render a page if it is not already in hand. */
  async function ensure(page: number): Promise<void> {
    if (!engraver || page < 1 || page > pages || resident.has(page)) return;
    const attempt = inFlight;
    const rendered = await engraver.page(page);
    if (attempt !== inFlight) return;
    resident = new Map(resident).set(page, rendered);
    mark("score");
  }

  async function engrave(): Promise<void> {
    if (!engraver || width === 0 || height === 0) return;
    const options = { ...pageFor(width, height, zoom, mode), zoom, ...modeOptions(mode) };
    const key = `${revision}:${mei.length}`;
    const held = loaded === undefined ? undefined : anchor();
    const attempt = (inFlight += 1);
    const next =
      key === loaded
        ? await engraver.relayout(options)
        : await engraver.load(mei, revision, options);
    // A layout overtaken while it was computing is dropped rather than shown:
    // the caller has already asked for its replacement (§2).
    if (attempt !== inFlight) return;
    loaded = key;

    // The old pages stay on screen and interactive until their replacements
    // are in hand; `resident` is replaced whole, never emptied first (§6).
    const first = visible.size > 0 ? Math.min(...visible) : 1;
    const arriving = new Map<number, PageSvg>();
    const from = Math.max(first - NEIGHBOURS, 1);
    const to = Math.min(first + NEIGHBOURS, next.pages);
    for (let page = from; page <= to; page += 1) {
      const rendered = await engraver.page(page);
      if (attempt !== inFlight) return;
      arriving.set(page, rendered);
    }
    layout = next;
    resident = arriving;
    mark("score");
    if (held) void restore(held);
  }

  onMount(() => {
    engraver = createEngraver();
    observer = new IntersectionObserver(
      (entries) => {
        const seen = new Set(visible);
        for (const entry of entries) {
          const page = Number((entry.target as HTMLElement).dataset.page);
          if (entry.isIntersecting) seen.add(page);
          else seen.delete(page);
        }
        visible = seen.size > 0 ? seen : new Set([1]);
      },
      // A page either side of the viewport is already being rendered by the
      // time it is scrolled to, which is what makes scrolling continuous.
      { root: host, rootMargin: "50%" },
    );
    return () => {
      observer?.disconnect();
      engraver?.dispose();
    };
  });

  $effect(() => {
    // Re-engrave when the score, the zoom, the mode, or the leaf's box changes.
    void mei;
    void revision;
    void zoom;
    void mode;
    void width;
    void height;
    void untrack(() => engrave());
  });

  /** Render what has scrolled into view; drop what has scrolled well out. */
  $effect(() => {
    const keep = wanted;
    void untrack(async () => {
      for (const page of keep) await ensure(page);
      if ([...resident.keys()].some((page) => !keep.has(page))) {
        resident = new Map([...resident].filter(([page]) => keep.has(page)));
      }
    });
  });

  /** Measure after every page swap: the boxes belong to this layout only. */
  $effect(() => {
    void resident;
    const ids = workspace ? workspace.selected : [];
    const hovered = workspace?.hovered ?? null;
    const container = host;
    if (!container || resident.size === 0) {
      selectionRects = [];
      hoverRects = [];
      return;
    }
    const grow = (rect: Rect) => pad(rect, HALO_SPACES, staffSpace);
    selectionRects = ids.flatMap((id) => boxesFor(container, id)).map(grow);
    hoverRects = hovered && !ids.includes(hovered) ? boxesFor(container, hovered).map(grow) : [];
  });

  function observe(element: HTMLElement): { destroy: () => void } {
    observer?.observe(element);
    return { destroy: () => observer?.unobserve(element) };
  }

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

  /**
   * A pinch gets an immediate transform, so the gesture feels attached to the
   * fingers, and a true re-layout 120 ms after it settles (§5). The transform
   * never survives the gesture: a persistent one is why score applications
   * look like PDF viewers.
   */
  function onwheel(event: WheelEvent): void {
    if (!event.ctrlKey || !onpinch) return;
    event.preventDefault();
    gesture = Math.min(Math.max(gesture * (1 - event.deltaY / 100), 0.5), 2);
    clearTimeout(pinch);
    pinch = setTimeout(() => {
      const settled = gesture;
      gesture = 1;
      onpinch?.(settled);
    }, PINCH_SETTLE_MS);
  }
</script>

<!--
  The score pane is a single tab stop; arrows navigate within it, because 4000
  tab stops is not accessibility (`03-interaction.md` §3). That single tab stop
  on a non-interactive element is the point, so the lint is answered rather
  than obeyed: `application` is the correct role for a widget with its own
  internal navigation, and a screen reader must hand the arrows through.
-->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="engraving"
  class:continuous={mode === "continuous"}
  bind:this={host}
  bind:clientWidth={width}
  bind:clientHeight={height}
  role="application"
  aria-label="Engraved score"
  tabindex="0"
  {onpointerdown}
  {onpointermove}
  {onpointerleave}
  {onwheel}
>
  <div class="pages" style:transform={gesture === 1 ? undefined : `scale(${gesture})`}>
    {#each Array.from({ length: pages }, (_, index) => index + 1) as number (number)}
      <div use:observe data-page={number} class="slot">
        <Page
          {number}
          page={resident.get(number)}
          {box}
          {scale}
          {staffSpace}
          selection={selectionRects}
          hover={hoverRects}
        />
      </div>
    {/each}
  </div>
</div>

<style>
  .engraving {
    height: 100%;
    overflow: auto;
    /*
     * Reserved, always. Without it the first page makes the scrollbar appear,
     * which narrows the leaf, which changes the page size, which re-lays the
     * score out — a feedback loop that ends in a degenerate layout.
     */
    scrollbar-gutter: stable;
    color: var(--ink);
  }

  .pages {
    display: flex;
    flex-direction: column;
    gap: var(--s-4);
    transform-origin: top center;
  }

  /* Continuous is one system, scrolled horizontally (§4). */
  .engraving.continuous .pages {
    flex-direction: row;
    width: max-content;
  }

  .slot {
    min-width: 0;
  }


  /*
   * A hollow notehead's centre is not on any path, so a click there would
   * miss the note. Bounding-box hit testing is what makes a half note as
   * clickable as a quarter.
   */
  .engraving :global(g[id^="event-"]) {
    pointer-events: bounding-box;
  }
</style>
