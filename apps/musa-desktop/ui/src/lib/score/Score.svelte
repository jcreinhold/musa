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
  import { createEngraver, type Engraver, type Layout, type PageSvg } from "musa-engrave";
  import { modeOptions, pageFor, pixelsPerUnit, type ViewMode } from "musa-engrave";
  import type { Workspace } from "../state/selection.svelte";
  import Page from "./Page.svelte";
  import {
    boxesFor,
    bracketNear,
    bracketOver,
    headsFor,
    pad,
    runsFor,
    traceTo,
    nearestNote,
    pointIn,
    NOTHING,
    type Bracket,
    type Ghost,
    type Marks,
    type Rect,
  } from "./geometry";
  import { HANDLE_SPACES, RUNG_SPACES, STEP_SPACES, THRESHOLD_PX, shiftStep, stepsFor } from "./steps";
  import type { Focus } from "../state/focus.svelte";
  import { Gesture } from "../state/gesture.svelte";
  import type { Candidate } from "../state/gesture.svelte";
  import type { NoteEntry } from "../state/entry.svelte";
  import type { EventFacts } from "../state/snapshot";
  import { eventIdOf } from "./ids";
  import { frontFieldOf, measureFront, type FrontMatterAt } from "./front-matter";
  import type { HeaderFieldDto } from "../session/generated/HeaderFieldDto";

  let {
    mei,
    revision,
    zoom,
    mode = "page",
    workspace,
    focus,
    entry,
    spell = false,
    onvisible,
    onedit,
    oncandidate,
    oninsert,
    onpinch,
    playing = [],
    loop = null,
    follow = "off",
    origin = false,
    flash = [],
    bring = null,
    header = [],
    onheader,
  }: {
    mei: string;
    revision: number;
    zoom: number;
    mode?: ViewMode;
    /** Absent for a fixture shown purely as engraving. */
    workspace?: Workspace;
    /**
     * The shared focus. The pane reports what the pointer is over
     * and marks what the focus resolves to; it decides neither.
     */
    focus?: Focus;
    /**
     * Note entry, for the one gesture that writes rather than rewrites: a
     * click on an empty staff step, which only means anything while entry is
     * armed.
     */
    entry?: NoteEntry;
    /**
     * Whether a live gesture prints what it would write beside the pointer.
     * It does when the source column is not showing — with the column open
     * the text says it, in the file it will be written into.
     */
    spell?: boolean;
    /**
     * The events engraved on the pages currently in view, reported when they
     * change. The source column ticks the lines that made them, which is a
     * question about *this page* and so can only be answered here.
     */
    onvisible?: (ids: string[]) => void;
    /**
     * A pointer gesture came up on a note: respell it, or renotate it. One
     * command, the same one the keyboard issues (`03-interaction.md` §2).
     */
    onedit?: (candidate: Candidate) => void;
    /**
     * The gesture moved: what it would write, or null when it would write
     * nothing. The source column stands the token in, so it has to be asked
     * while the pointer is still down.
     */
    oncandidate?: (candidate: Candidate | null) => void;
    /** A click on an empty staff step, with entry armed: write a note there. */
    oninsert?: (pitch: string) => void;
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
     * takes you to a section. Unlike following the
     * playhead, this is asked for, so it scrolls even when the note is
     * already on screen but off to one side.
     */
    bring?: { id: string } | null;
    /**
     * What the piece says about itself, as the source spells it — the value a
     * front-matter field starts from when it is opened. Empty for
     * a fixture shown purely as engraving, which is also what makes the page
     * read-only there.
     */
    header?: { field: HeaderFieldDto; value: string | null }[];
    /** Rewrite one of the piece's own statements. */
    onheader?: (field: HeaderFieldDto, value: string) => void;
  } = $props();

  /** Selection halo padding, in staff spaces (§8). */
  const HALO_SPACES = 0.6;

  /**
   * The live pointer gesture. It owns what the drag means; this
   * pane owns the geometry that feeds it and the ink that shows it.
   */
  const drag = new Gesture();

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
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- reactivity is by whole replacement, not in-place mutation
    resident = new Map(resident).set(page, rendered);
    mark("score");
  }

  async function engrave(): Promise<void> {
    if (!engraver || width === 0 || height === 0) return;
    const options = {
      ...pageFor(width, height, zoom, mode),
      zoom,
      ...modeOptions(mode),
    };
    const key = `${revision}:${mei.length}`;
    const held = loaded === undefined ? undefined : anchor();
    const attempt = (inFlight += 1);
    const next = key === loaded ? await engraver.relayout(options) : await engraver.load(mei, revision, options);
    // A layout overtaken while it was computing is dropped rather than shown:
    // the caller has already asked for its replacement (§2).
    if (attempt !== inFlight) return;
    loaded = key;

    // The old pages stay on screen and interactive until their replacements
    // are in hand; `resident` is replaced whole, never emptied first (§6).
    const first = visible.size > 0 ? Math.min(...visible) : 1;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local accumulator, assigned whole below
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
        // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local copy, assigned whole below
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
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local accumulator, assigned whole below
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
    const focused = focus?.marked.events ?? [];
    const pending = drag.candidate;
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
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local accumulator, assigned whole below
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
        focus: focused.flatMap((id) => headsFor(element, id)),
        candidate: pending ? ghostFor(element, pending) : null,
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
   * What is engraved on the pages in view.
   *
   * Reported on a page swap and on a scroll, never on a pointer move: the
   * source column's gutter ticks the lines that made the music currently on
   * screen, and that changes when the page does.
   */
  $effect(() => {
    const arrived = resident;
    const pages = visible;
    const container = host;
    const report = onvisible;
    if (!container || !report) return;
    untrack(() => {
      void arrived;
      // eslint-disable-next-line svelte/prefer-svelte-reactivity -- local set, never state
      const ids = new Set<string>();
      for (const [number, element] of arriving(container)) {
        if (!pages.has(number)) continue;
        for (const drawn of element.querySelectorAll<SVGGraphicsElement>('g[id^="event-"]')) {
          const id = eventIdOf(drawn);
          if (id) ids.add(id);
        }
      }
      report([...ids]);
    });
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
      const inside = at.top >= seen.top && at.bottom <= seen.bottom && at.left >= seen.left && at.right <= seen.right;
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
      const facts = new Map((workspace.snapshot?.score?.events ?? []).map((event) => [event.id, event]));
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

  /**
   * The line of front matter being typed over, if one is.
   *
   * It is a real input laid over the printed text, in the page's own face and
   * at its own size, rather than a field in a panel somewhere else — the whole
   * point is that a composer changes the title where they read it. It commits
   * the way every other value in the interface commits: `Return` sends it,
   * `Esc` puts it back, and blurring sends it, because a value someone typed
   * and clicked away from was meant.
   */
  let renaming = $state<FrontMatterAt | null>(null);
  let draft = $state("");
  let field = $state<HTMLInputElement | undefined>();

  $effect(() => {
    if (renaming) field?.focus();
  });

  /** The page's front matter is only editable where there is a core to tell. */
  const editable = $derived(onheader !== undefined && header.length > 0);

  /** Where a printed line is, given anything drawn inside it. */
  function lineAt(target: Element): FrontMatterAt | null {
    const which = frontFieldOf(target);
    const line = which && target.closest(`[id="front-${which}"]`);
    if (!which || !line || !host || !editable) return null;
    return measureFront(host, line, which);
  }

  function openFront(target: Element): boolean {
    const at = lineAt(target);
    if (!at) return false;
    renaming = at;
    draft = header.find((fact) => fact.field === at.field)?.value ?? "";
    return true;
  }

  function commitFront(): void {
    const open = renaming;
    renaming = null;
    if (!open) return;
    const next = draft.trim();
    const before = header.find((fact) => fact.field === open.field)?.value ?? "";
    if (next !== before) onheader?.(open.field, next);
  }

  /**
   * The field is at least wide enough to type into, which for a role the piece
   * has not filled in yet is wider than the nothing it is printing. Growing it
   * has to keep the edge the line was set against, or a centred title would
   * drift left the moment it was clicked.
   */
  const frontBox = $derived.by(() => {
    if (!renaming) return null;
    // A little wider than the line it covers, always: the field is set in the
    // interface's text face and the page in the engraver's, so the same words
    // do not measure the same, and a title clipped at its first letter the
    // moment it is clicked reads as damage.
    const width = Math.max(renaming.width * 1.15, renaming.fontSize * 6);
    const grew = width - renaming.width;
    const left =
      renaming.align === "center"
        ? renaming.left - grew / 2
        : renaming.align === "right"
          ? renaming.left - grew
          : renaming.left;
    return { left, width };
  });

  /**
   * The line under the pointer, if it is one of the five.
   *
   * Drawn rather than declared: `text-decoration` on SVG text is painted with
   * the glyph's own fill in Blink, so a transparent rest state is not
   * available and the whole page would read as underlined. A hairline of the
   * interface's own is both the honest way to say it and the exact one the
   * inspector's rows draw.
   */
  let hovered = $state<FrontMatterAt | null>(null);

  /** Whether the pointer is over a note's duration handle, which sets the cursor. */
  let handled = $state(false);

  function hoverFront(target: Element): void {
    const which = frontFieldOf(target);
    if (!which) hovered = null;
    else if (which !== hovered?.field) hovered = lineAt(target);
  }

  function onfrontkeydown(pressed: KeyboardEvent): void {
    pressed.stopPropagation();
    if (pressed.key === "Enter") {
      pressed.preventDefault();
      (pressed.currentTarget as HTMLInputElement).blur();
    }
    if (pressed.key === "Escape") {
      pressed.preventDefault();
      renaming = null;
    }
  }

  function onpointerdown(event: PointerEvent): void {
    // The page's own front matter first: it is the one thing on the page that
    // is not music, and clicking it is a different question from clicking a
    // note (`03-interaction.md` §2).
    if (openFront(event.target as Element)) {
      // The pane is a tab stop, so the press that opened the field would
      // otherwise hand focus straight back to it and the blur would close the
      // field before a key reached it. Refusing the default keeps the caret
      // where the composer just pointed.
      event.preventDefault();
      return;
    }
    renaming = null;
    if (!workspace) return;
    mark("select");
    const id = eventIdOf(event.target as Element);
    if (!id) {
      if (!writeAt(event)) workspace.clear();
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
    // The press is also where a gesture would start. It is still a click
    // until the pointer travels, so nothing about selection changes here
    // (`03-interaction.md` §2).
    if (onedit) hold(event, generated ?? null);
  }

  /**
   * Start a gesture on the note that was just pressed.
   *
   * Everything it needs is measured once, here: the note's own box in page
   * units, whether the press landed in the duration handle at its right edge,
   * and what the core says the note is spelled and notated as.
   */
  function hold(event: PointerEvent, note: EventFacts | null): void {
    const element = (event.target as Element).closest<HTMLElement>(".arriving");
    const at = element ? pointIn(element, event.clientX, event.clientY) : null;
    if (!element || !at || !note) return;
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
    drag.begin({
      event: note.id,
      pitch: note.pitchSpellings[0] ?? null,
      duration: note.durationSpelling,
      atEdge: onHandle(event, note.id),
      alt: event.altKey,
      x: at.x,
      y: at.y,
      threshold: THRESHOLD_PX * at.perPixel,
    });
  }

  /**
   * Whether the pointer is in a note's duration handle: the strip at its right
   * edge, one staff space wide, where a drag renotates rather than respells.
   *
   * Measured only when there is a note under the pointer, so blank paper costs
   * nothing.
   */
  function onHandle(event: PointerEvent, id: string): boolean {
    const element = (event.target as Element).closest<HTMLElement>(".arriving");
    const at = element ? pointIn(element, event.clientX, event.clientY) : null;
    const [box] = element ? boxesFor(element, id) : [];
    if (!at || !box) return false;
    // Never more than a third of the note: a whole note is barely wider than
    // the handle, and a note whose middle asks about its duration has no
    // middle left to ask about its pitch.
    const handle = Math.min(staffSpace * HANDLE_SPACES, box.width / 3);
    return at.x >= box.x + box.width - handle;
  }

  /**
   * A click on empty staff, with entry armed: write a note at the step
   * clicked (`03-interaction.md` §2).
   *
   * The step is measured from the nearest note on the same staff, whose
   * spelling the core published — so no clef is read and no pitch is guessed.
   * With no note on that staff to measure from there is nothing to write
   * against, and the click stays a click.
   */
  function writeAt(event: PointerEvent): boolean {
    if (!oninsert || entry?.on !== true) return false;
    const staff = (event.target as Element).closest("g.staff");
    const element = (event.target as Element).closest<HTMLElement>(".arriving");
    const at = element ? pointIn(element, event.clientX, event.clientY) : null;
    if (!staff || !element || !at) return false;
    const anchor = nearestNote(element, staff, at.x);
    const note = workspace?.snapshot?.score?.events.find((each) => each.id === anchor);
    const spelling = note?.pitchSpellings[0];
    const [head] = anchor === null ? [] : headsFor(element, anchor);
    if (!head || spelling === undefined) return false;
    const pitch = shiftStep(spelling, stepsFor(at.y - (head.y + head.height / 2), staffSpace));
    if (pitch === null) return false;
    oninsert(pitch);
    return true;
  }

  // What the gesture would write is reported out as it snaps, so the source
  // column can stand the token in while the pointer is still down.
  $effect(() => {
    const pending = drag.candidate;
    untrack(() => oncandidate?.(pending));
  });

  /** Where the candidate would be drawn, in this page's own coordinates. */
  function ghostFor(element: HTMLElement, candidate: Candidate): Ghost | null {
    const label = spell ? candidate.value : null;
    if (candidate.kind === "pitch") {
      const [head] = headsFor(element, candidate.event);
      if (!head) return null;
      const rise = candidate.distance * staffSpace * STEP_SPACES;
      return { kind: "pitch", rect: { ...head, y: head.y - rise }, label };
    }
    const [box] = boxesFor(element, candidate.event);
    if (!box) return null;
    const reach = candidate.distance * staffSpace * RUNG_SPACES;
    // A shortening drag never draws a span narrower than the note itself:
    // the shape says how far along the ladder the gesture is, and the text
    // says what it would write.
    return {
      kind: "duration",
      rect: { ...box, width: Math.max(box.width + reach, staffSpace) },
      label,
    };
  }

  /** A click on a margin bracket selects what that expansion produced. */
  function selectOccurrence(id: string): void {
    mark("select");
    workspace?.selectOccurrence(id);
  }

  function onpointermove(event: PointerEvent): void {
    if (drag.event !== null) {
      const element = (event.target as Element).closest<HTMLElement>(".arriving") ?? host;
      const at = element ? pointIn(element, event.clientX, event.clientY) : null;
      if (at) drag.move(at.x, at.y, staffSpace);
      // A drag that turned out to be horizontal is a range selection, which
      // is what a drag on the leaf has always meant (§2).
      if (drag.axis === "range") extend(event);
      return;
    }
    hoverFront(event.target as Element);
    const id = eventIdOf(event.target as Element);
    // The focus is reported whether or not there is a note under the pointer:
    // blank paper marks nothing, which is a different answer from "the
    // pointer is elsewhere".
    focus?.point(id);
    // The one cursor change on the page: the handle says, before the press,
    // that a drag here asks about the duration (`03-interaction.md` §2).
    handled = id !== null && onedit !== undefined && onHandle(event, id);
    if (!workspace) return;
    workspace.hovered = id;
  }

  /**
   * The horizontal drag: extend the selection to the note under the pointer.
   *
   * The press captured the pointer, so every later event is aimed at the pane
   * rather than at what is under it. What is under it is the question, so it
   * is asked of the document directly.
   */
  function extend(event: PointerEvent): void {
    const under = document.elementFromPoint(event.clientX, event.clientY);
    const to = under ? eventIdOf(under) : null;
    const from = drag.event;
    if (!workspace || to === null || from === null || to === from) return;
    workspace.select(from);
    workspace.select(to, true);
  }

  function onpointerup(): void {
    const written = drag.release();
    if (written) onedit?.(written);
  }

  /**
   * `Esc` gives up on a gesture, and the document is untouched.
   *
   * On the window rather than on the pane, because a drag captures the
   * pointer and the keyboard focus may be anywhere — the composer pressing
   * `Esc` mid-drag is not thinking about which element has focus.
   */
  $effect(() => {
    if (!drag.editing) return;
    const abort = (pressed: KeyboardEvent) => {
      if (pressed.key !== "Escape") return;
      pressed.preventDefault();
      drag.cancel();
    };
    globalThis.addEventListener("keydown", abort, true);
    return () => globalThis.removeEventListener("keydown", abort, true);
  });

  function onpointerleave(): void {
    hovered = null;
    handled = false;
    focus?.leave();
    drag.cancel();
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
  class:editable
  class:handled
  bind:this={host}
  bind:clientWidth={width}
  bind:clientHeight={height}
  role="application"
  aria-label="Engraved score"
  aria-activedescendant={activeDescendant}
  tabindex="0"
  {onpointerdown}
  {onpointermove}
  {onpointerup}
  onpointercancel={() => drag.cancel()}
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
  <!--
    Over the page, not beside it: the composer is looking at the title, so the
    title is where they type. It sits outside `.pages` so a pinch cannot carry
    it, and its box was measured against this scroll container.
  -->
  {#if hovered && !renaming}
    <div
      class="hairline"
      style:left="{hovered.left}px"
      style:top="{hovered.top + hovered.height}px"
      style:width="{hovered.width}px"
    ></div>
  {/if}
  {#if renaming && frontBox}
    <input
      class="front"
      type="text"
      aria-label={renaming.field}
      spellcheck="false"
      autocomplete="off"
      bind:this={field}
      bind:value={draft}
      style:left="{frontBox.left}px"
      style:top="{renaming.top}px"
      style:width="{frontBox.width}px"
      style:height="{renaming.height}px"
      style:font-size="{renaming.fontSize}px"
      style:text-align={renaming.align}
      onpointerdown={(pressed) => pressed.stopPropagation()}
      onkeydown={onfrontkeydown}
      onblur={commitFront}
    />
  {/if}
</div>

<style>
  .engraving {
    height: 100%;
    overflow: auto;
    /* The front-matter field is placed against this box. */
    position: relative;
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
   * The five printed lines of front matter are controls, and on hover they
   * say so with the same hairline the inspector's rows use.
   *
   * Only on hover, and this is the one place the rest-state underline of
   * the rest-state underline does not apply. The chrome is an interface and can afford to
   * advertise; the page is the artifact, and a title permanently underlined
   * is a page that looks like a web form rather than like an edition — which
   * is the opposite of what the page apparatus was added for. Discovery does
   * not depend on it: the inspector prints the same five fields, and prints
   * them whether or not the piece has filled them in.
   */
  .engraving.editable :global([id^="front-"]) {
    /* The whole line, not only the strokes of its letters: the gaps inside an
       `a` are part of the word as far as a composer aiming at it is concerned,
       and this is the same reason a hollow notehead is hit by its box. */
    pointer-events: bounding-box;
    cursor: text;
  }

  /* Over a note's duration handle: the pointer says which way it would go. */
  .engraving.handled {
    cursor: ew-resize;
  }

  .hairline {
    position: absolute;
    z-index: 1;
    height: 0;
    border-bottom: 1px solid var(--ink-muted);
    pointer-events: none;
  }

  /*
   * The field is the printed line, typed into: the page's own face, the page's
   * own size, no box. The only mark it adds is the focus underline every other
   * editable value in the interface draws.
   */
  .front {
    position: absolute;
    z-index: 1;
    background: var(--leaf);
    border: 0;
    border-bottom: 1px solid var(--plate);
    padding: 0;
    font-family: var(--f-score-text);
    line-height: 1;
    color: var(--ink);
  }

  .front:focus {
    outline: none;
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
   * The blank staff is pointable too, because entry writes where the composer
   * points (`03-interaction.md` §2) and the space between two lines is where
   * a note goes. An SVG group is otherwise hit only where something is drawn,
   * which would make the one place a new note belongs the one place a click
   * cannot land. Notes are drawn inside the staff and are hit first, so this
   * costs nothing that was already hittable.
   */
  .engraving :global(g.staff) {
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
