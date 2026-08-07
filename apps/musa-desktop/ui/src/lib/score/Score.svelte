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
  import {
    boxesFor,
    bracketNear,
    bracketOver,
    pad,
    runsFor,
    traceTo,
    NOTHING,
    type Bracket,
    type Marks,
    type Rect,
  } from "./geometry";
  import { eventIdOf } from "./ids";

  let {
    mei,
    revision,
    zoom,
    mode = "page",
    workspace,
    onpinch,
    playing = [],
    loop = null,
    follow = "off",
    origin = false,
    flash = [],
    bring = null,
  }: {
    mei: string;
    revision: number;
    zoom: number;
    mode?: ViewMode;
    /** Absent for a fixture shown purely as engraving. */
    workspace?: Workspace;
    /** A settled pinch, as a factor on the current zoom (§5). */
    onpinch?: (factor: number) => void;
    /** The event ids sounding right now (`03-interaction.md` §4). */
    playing?: string[];
    /** The looped range, as its first and last event ids. */
    loop?: [string, string] | null;
    /** How the page follows the playhead: off, a page turn, or a scroll. */
    follow?: "off" | "page" | "continuous";
    /** Origin view held or pinned (`04-provenance.md` §2). */
    origin?: boolean;
    /** Event ids a diagnostic points at; their systems flash once. */
    flash?: string[];
    /**
     * A note to bring into view, once, when it changes — how the outline
     * takes you to a section (`docs/prompts/35`). Unlike following the
     * playhead, this is asked for, so it scrolls even when the note is
     * already on screen but off to one side.
     */
    bring?: { id: string } | null;
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
  /** Every mark on every resident page, measured in that page's own units. */
  let marks = $state<Map<number, Marks>>(new Map());
  /**
   * Origin view's margin brackets, kept apart from the rest of the marks
   * because they cost a measurement per generated event and change only when
   * the lens opens or the pages swap — never on a pointer move.
   */
  let brackets = $state<Map<number, Bracket[]>>(new Map());
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

  /**
   * The pages whose ink is the current layout's, with their numbers.
   *
   * The arriving ink only: a page mid-cross-fade still holds the outgoing
   * engraving, and measuring that would both double every box and take the
   * coordinates from the wrong root (`02-engraving.md` §6).
   */
  function arriving(container: HTMLElement): [number, HTMLElement][] {
    const found: [number, HTMLElement][] = [];
    for (const page of container.querySelectorAll<HTMLElement>(".page")) {
      const ink = page.querySelector<HTMLElement>(".arriving");
      if (ink) found.push([Number(page.dataset.page), ink]);
    }
    return found;
  }

  /**
   * Origin view's brackets: one `⟨` per system a given expansion reached.
   *
   * Measured only while the lens is open, because it costs a box per generated
   * event and a score can be mostly generated.
   */
  $effect(() => {
    void resident;
    const occurrences = origin ? (workspace?.occurrences ?? []) : [];
    const container = host;
    void staffSpace;
    if (!container || occurrences.length === 0) {
      brackets = new Map();
      return;
    }
    const measured = new Map<number, Bracket[]>();
    for (const [number, element] of arriving(container)) {
      const found = occurrences.flatMap((occurrence) =>
        runsFor(element, occurrence.events).map((where) =>
          bracketOver(occurrence.id, occurrence.label, where.run, staffSpace),
        ),
      );
      if (found.length > 0) measured.set(number, found);
    }
    brackets = measured;
  });

  /** Measure after every page swap: the boxes belong to this layout only. */
  $effect(() => {
    void resident;
    const ids = workspace ? workspace.selected : [];
    const hovered = workspace?.hovered ?? null;
    const sounding = playing;
    const looped = loop;
    const flashing = flash;
    const bracketed = brackets;
    const traced = origin ? hovered : null;
    const caret = workspace?.caretAt ?? null;
    const container = host;
    if (!container || resident.size === 0) {
      marks = new Map();
      return;
    }
    const occurrenceOf = (id: string) =>
      workspace?.snapshot?.score?.events.find((event) => event.id === id)?.origin.occurrence ?? null;
    const grow = (rect: Rect) => pad(rect, HALO_SPACES, staffSpace);
    const measured = new Map<number, Marks>();
    for (const [number, element] of arriving(container)) {
      const boxes = (id: string) => boxesFor(element, id);
      const first = (id: string) => boxesFor(element, id)[0] ?? null;
      const from = looped ? first(looped[0]) : null;
      const to = looped ? first(looped[1]) : null;
      const here = bracketed.get(number) ?? [];
      const at = traced ? first(traced) : null;
      const occurrence = traced ? occurrenceOf(traced) : null;
      const bracket = at && occurrence ? bracketNear(here, occurrence, at) : undefined;
      measured.set(number, {
        selection: ids.flatMap(boxes).map(grow),
        hover: hovered && !ids.includes(hovered) ? boxes(hovered).map(grow) : [],
        playing: sounding.flatMap(boxes).map(grow),
        caret: caret ? caretRect(first(caret.id), caret.side) : null,
        loop: from && to ? { from, to } : null,
        flash: flashing.length > 0 ? runsFor(element, flashing).map((where) => where.system) : [],
        brackets: here,
        trace: bracket && at ? traceTo(bracket, at) : null,
      });
    }
    marks = measured;
    mark("halo");
  });

  /**
   * The caret is a hairline at an edge of the event it is pinned to: its left
   * edge before it, its right edge after the voice's last (§1). Half a staff
   * space of air keeps it off the notehead it belongs in front of.
   */
  function caretRect(rect: Rect | null, side: "before" | "after"): Rect | null {
    if (rect === null) return null;
    const x = side === "before" ? rect.x - staffSpace / 2 : rect.x + rect.width + staffSpace / 2;
    return { ...rect, x, width: 0 };
  }

  /**
   * Follow the playhead (§4).
   *
   * A page turn, not a scroll: in page view the sounding note's page is
   * brought into view whole, exactly as a page turner would. Continuous has
   * no pages to turn, so there it is a scroll, and only when the note has
   * actually left the viewport — a viewport that re-centres every note is a
   * viewport nobody can read from.
   */
  $effect(() => {
    const [first] = playing;
    const container = host;
    if (follow === "off" || first === undefined || !container) return;
    void marks;
    untrack(() => {
      const element = container.querySelector<SVGGraphicsElement>(`[id="${first}"]`);
      if (!element) return;
      const seen = container.getBoundingClientRect();
      const at = element.getBoundingClientRect();
      const inside =
        at.top >= seen.top &&
        at.bottom <= seen.bottom &&
        at.left >= seen.left &&
        at.right <= seen.right;
      if (inside) return;
      if (follow === "page") element.closest(".page")?.scrollIntoView({ block: "start" });
      else element.scrollIntoView({ block: "nearest", inline: "center" });
    });
  });

  /**
   * Go to a note the composer asked for, once per request.
   *
   * Page view brings its whole page, as a page turner would; continuous
   * centres it. Both wait on `marks`, because the note is only findable once
   * the page it is on has been engraved.
   */
  $effect(() => {
    const target = bring;
    const container = host;
    if (!target || !container) return;
    void marks;
    untrack(() => {
      const element = container.querySelector<SVGGraphicsElement>(`[id="${target.id}"]`);
      if (!element) return;
      if (mode === "page") element.closest(".page")?.scrollIntoView({ block: "start" });
      else element.scrollIntoView({ block: "nearest", inline: "center" });
    });
  });

  /**
   * Name every engraved event for a screen reader (`03-interaction.md` §5).
   *
   * Verovio writes the geometry; the names come from the snapshot, so what is
   * read out is a musical sentence rather than a path element. Done once per
   * page swap rather than per frame, because the names change only when the
   * music does.
   */
  $effect(() => {
    const arrived = resident;
    const container = host;
    if (!container || !workspace) return;
    const describe = workspace.describe.bind(workspace);
    untrack(() => {
      void arrived;
      const facts = new Map(
        (workspace.snapshot?.score?.events ?? []).map((event) => [event.id, event]),
      );
      for (const element of container.querySelectorAll<SVGGraphicsElement>('g[id^="event-"]')) {
        const event = facts.get(eventIdOf(element) ?? "");
        if (event === undefined) continue;
        // `img` is the honest role for an engraved note: a graphic with a
        // name. `option`/`listitem` would demand a parent Verovio does not
        // write, and a `g` with no role may not carry a name at all.
        element.setAttribute("role", "img");
        element.setAttribute("aria-label", describe(event));
        element.setAttribute("tabindex", "-1");
        // Origin view is then a stylesheet, not a traversal: which ink is
        // generated is stamped on the ink once, when the page arrives.
        if (event.origin.generated) element.dataset["generated"] = "true";
        else delete element.dataset["generated"];
      }
    });
  });

  /** What the pane reports as its current item: the focused event (§5). */
  const activeDescendant = $derived(workspace?.selected[0] ?? undefined);

  function observe(element: HTMLElement): { destroy: () => void } {
    observer?.observe(element);
    return { destroy: () => observer?.unobserve(element) };
  }

  function onpointerdown(event: PointerEvent): void {
    if (!workspace) return;
    mark("select");
    const id = eventIdOf(event.target as Element);
    if (!id) {
      workspace.clear();
      return;
    }
    // Held lens: a click asks about provenance, so the unit is the whole
    // expansion rather than the notehead under the pointer (§2).
    const generated = workspace.snapshot?.score?.events.find((candidate) => candidate.id === id);
    if (origin && generated?.origin.occurrence) {
      workspace.selectOccurrence(generated.origin.occurrence);
      return;
    }
    workspace.select(id, event.shiftKey);
  }

  /** A click on a margin bracket selects what that expansion produced. */
  function selectOccurrence(id: string): void {
    mark("select");
    workspace?.selectOccurrence(id);
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
  class:origin
  bind:this={host}
  bind:clientWidth={width}
  bind:clientHeight={height}
  role="application"
  aria-label="Engraved score"
  aria-activedescendant={activeDescendant}
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
          marks={marks.get(number) ?? NOTHING}
          onoccurrence={selectOccurrence}
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

  /*
   * Origin view (`04-provenance.md` §2). Verovio's output inherits
   * `currentColor`, so re-inking the generated groups is the whole lens: ink
   * and opacity only, no reflow, nothing inserted into the engraving. The
   * transition lives on the ink rather than on the held class so that
   * releasing the key fades back rather than snapping.
   */
  .engraving :global(g[data-generated="true"]) {
    transition:
      color var(--m-base) var(--e-out),
      opacity var(--m-base) var(--e-out);
  }

  .engraving.origin :global(g[data-generated="true"]) {
    color: var(--plate);
    opacity: 0.65;
  }

  @media (prefers-reduced-motion: reduce) {
    .engraving :global(g[data-generated="true"]) {
      transition: none;
    }
  }
</style>
