<script lang="ts">
  /**
   * The single overlay layer (`02-engraving.md` §8): everything musa draws
   * over the engraving, in the page's own coordinates and measured in staff
   * spaces, which is why none of it looks pasted on.
   *
   * Selection is a `0.6sp` rounded halo in `--plate` at 18 % fill with a 1 px
   * stroke; hover is the same halo at 8 % and no stroke. The playhead is the
   * sounding note tinted, plus a hairline through the system it is in
   * (`03-interaction.md` §4) — a tint that says *which note*, which a bare
   * line cannot. The caret is a `2sp` hairline at the insertion point (§1).
   * Loop is a pair of repeat brackets in the margin, never a coloured
   * rectangle.
   */
  import { TICK_SPACES, type Bracket, type Ghost, type Rect, type Trace } from "./geometry";

  let {
    box,
    staffSpace,
    selection,
    hover,
    focus = [],
    candidate = null,
    playing = [],
    caret = null,
    loop = null,
    flash = [],
    brackets = [],
    trace = null,
    onoccurrence,
  }: {
    /** The page's coordinate system, adopted verbatim from the engraving. */
    box: { width: number; height: number };
    /** One staff space in those coordinates. */
    staffSpace: number;
    selection: Rect[];
    hover: Rect[];
    /** The noteheads the focus marks, as heads, not event boxes. */
    focus?: Rect[];
    /** What a live pointer gesture would write. */
    candidate?: Ghost | null;
    /** The notes sounding right now. */
    playing?: Rect[];
    /** Where the caret sits, if it is placed on this page. */
    caret?: Rect | null;
    /** The looped region's first and last note on this page. */
    loop?: { from: Rect; to: Rect } | null;
    /** Systems a diagnostic points at, flashed once and then gone. */
    flash?: Rect[];
    /** Origin view's margin brackets (`04-provenance.md` §2). */
    brackets?: Bracket[];
    /** The hover trace from a generated note out to its bracket. */
    trace?: Trace | null;
    /** Select an occurrence by clicking its bracket. */
    onoccurrence?: (id: string) => void;
  } = $props();

  /** Halo corners round at a third of a staff space — the score's own scale. */
  const radius = $derived(staffSpace / 3);

  /** How far above and below a staff the loop bracket reaches. */
  const BRACKET_SPACES = 1.5;

  /** The caret's height: two staff spaces of hairline (§1). */
  const CARET_SPACES = 2;

  /** The bracket label, set in the score's own unit rather than in pixels. */
  const LABEL_SIZE = $derived(staffSpace * 1.3);

  /** How wide the focus hairline is, in staff spaces. */
  const FOCUS_SPACES = 1;

  /** How far it clears the head, in staff spaces: enough to not be a stem. */
  const FOCUS_CLEARANCE = 0.35;

  /** How far the candidate's label sits from the ink it names, in staff spaces. */
  const LABEL_CLEARANCE = 1.6;

  /** The candidate label's size, in the score's own unit rather than in pixels. */
  const CANDIDATE_SIZE = $derived(staffSpace * 1.2);
</script>

<svg
  class="overlay"
  viewBox="0 0 {box.width} {box.height}"
  aria-hidden="true"
  style:display={box.width > 0 ? undefined : "none"}
>
  {#each hover as rect, index (index)}
    <rect class="hover" x={rect.x} y={rect.y} width={rect.width} height={rect.height} rx={radius} />
  {/each}

  {#each playing as rect, index (index)}
    <rect
      class="playing"
      x={rect.x}
      y={rect.y}
      width={rect.width}
      height={rect.height}
      rx={radius}
    />
    <!-- The hairline sits at the note's onset edge, through its staff. -->
    <line
      class="playhead"
      x1={rect.x}
      x2={rect.x}
      y1={rect.y - staffSpace * BRACKET_SPACES}
      y2={rect.y + rect.height + staffSpace * BRACKET_SPACES}
    />
  {/each}

  <!--
    The focus: a hairline directly under the notehead, one staff space wide. A shape, not a hue — the halo says *selected*, the re-inked
    staff says *generated*, and this says *this one, and its siblings*. All
    three can be true of one note at once, so no two of them may be the same
    mark.
  -->
  {#each focus as rect, index (index)}
    {@const middle = rect.x + rect.width / 2}
    {@const reach = (staffSpace * FOCUS_SPACES) / 2}
    <line
      class="focus"
      x1={middle - reach}
      x2={middle + reach}
      y1={rect.y + rect.height + staffSpace * FOCUS_CLEARANCE}
      y2={rect.y + rect.height + staffSpace * FOCUS_CLEARANCE}
    />
  {/each}

  <!--
    A live gesture, before anything is written. A respelling draws
    the head where it would land; a renotation draws the span the note would
    take. Both in `--plate`, which already means *derived, or live*, and both
    over the engraving rather than in it: the page does not move while a
    gesture is deciding what to ask for.
  -->
  {#if candidate}
    {#if candidate.kind === "pitch"}
      <ellipse
        class="candidate"
        cx={candidate.rect.x + candidate.rect.width / 2}
        cy={candidate.rect.y + candidate.rect.height / 2}
        rx={candidate.rect.width / 2}
        ry={candidate.rect.height / 2}
      />
    {:else}
      {@const right = candidate.rect.x + candidate.rect.width}
      {@const tick = staffSpace * TICK_SPACES}
      <path
        class="candidate span"
        d="M {candidate.rect.x} {candidate.rect.y - tick} L {candidate.rect.x} {candidate.rect
          .y} L {right} {candidate.rect.y} L {right} {candidate.rect.y - tick}"
      />
    {/if}
    {#if candidate.label}
      <text
        class="candidate label"
        x={candidate.rect.x + candidate.rect.width / 2}
        y={candidate.rect.y - staffSpace * LABEL_CLEARANCE}
        font-size={CANDIDATE_SIZE}>{candidate.label}</text
      >
    {/if}
  {/if}

  {#each selection as rect, index (index)}
    <rect
      class="selection"
      x={rect.x}
      y={rect.y}
      width={rect.width}
      height={rect.height}
      rx={radius}
    />
  {/each}

  {#if caret}
    <line
      class="caret"
      x1={caret.x}
      x2={caret.x}
      y1={caret.y + caret.height / 2 - staffSpace * CARET_SPACES}
      y2={caret.y + caret.height / 2 + staffSpace * CARET_SPACES}
    />
  {/if}

  {#if loop}
    {@const top = Math.min(loop.from.y, loop.to.y) - staffSpace * BRACKET_SPACES}
    {@const bottom =
      Math.max(loop.from.y + loop.from.height, loop.to.y + loop.to.height) +
      staffSpace * BRACKET_SPACES}
    {@const left = loop.from.x - staffSpace}
    {@const right = loop.to.x + loop.to.width + staffSpace}
    <path
      class="loop"
      d="M {left + staffSpace} {top} L {left} {top} L {left} {bottom} L {left +
        staffSpace} {bottom}"
    />
    <path
      class="loop"
      d="M {right - staffSpace} {top} L {right} {top} L {right} {bottom} L {right -
        staffSpace} {bottom}"
    />
    <circle class="loop dot" cx={left + staffSpace / 2} cy={top + staffSpace} r={staffSpace / 4} />
    <circle
      class="loop dot"
      cx={right - staffSpace / 2}
      cy={bottom - staffSpace}
      r={staffSpace / 4}
    />
  {/if}

  <!--
    A diagnostic points at a place in the music, so the music says where: the
    system flashes once, in --chalk, and is then gone (`05-states.md` §5). Not
    a toast, and not a state that has to be dismissed.
  -->
  {#each flash as rect, index (index)}
    <rect
      class="flash"
      x={rect.x}
      y={rect.y}
      width={rect.width}
      height={rect.height}
      rx={radius}
    />
  {/each}

  <!--
    Origin view's brackets: an editorial span over exactly the notes one
    expansion produced (`04-provenance.md` §2). The extent is the first signal
    and the label the second, because colour is never the only one
    (`03-interaction.md` §5). Both sit above the ink, so nothing moves.
  -->
  {#each brackets as bracket (bracket.id + bracket.x)}
    {@const tick = staffSpace * TICK_SPACES}
    {@const right = bracket.x + bracket.width}
    <g
      class="bracket"
      role="button"
      tabindex="-1"
      aria-label="Select the notes from {bracket.label}"
      onpointerdown={(event) => event.stopPropagation()}
      onclick={() => onoccurrence?.(bracket.id)}
      onkeydown={(event) => {
        if (event.key === "Enter" || event.key === " ") onoccurrence?.(bracket.id);
      }}
    >
      <path
        class="span"
        d="M {bracket.x} {bracket.y + tick} L {bracket.x} {bracket.y} L {right} {bracket.y} L {right} {bracket.y + tick}"
      />
      <text class="label" x={bracket.x} y={bracket.y - tick} font-size={LABEL_SIZE}>
        {bracket.label}
      </text>
      <!-- The band above the run is the hit area; a hairline is not clickable. -->
      <rect
        class="hit"
        x={bracket.x}
        y={bracket.y - tick * 2}
        width={bracket.width}
        height={tick * 3}
      />
    </g>
  {/each}

  {#if trace}
    <line class="trace" x1={trace.x1} y1={trace.y1} x2={trace.x2} y2={trace.y2} />
  {/if}
</svg>

<style>
  .overlay {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .hover {
    fill: var(--plate);
    fill-opacity: 0.08;
  }

  .selection {
    fill: var(--plate);
    fill-opacity: 0.18;
    stroke: var(--plate);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }

  .playing {
    fill: var(--plate);
    fill-opacity: 0.28;
  }

  /* No transition: the hairline appears and disappears. §6 allows three
     animations and this is not one of them. */
  .focus,
  .playhead,
  .caret {
    stroke: var(--plate);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }

  /* No transition and no ease: a candidate appears and disappears (§6). */
  .candidate {
    fill: none;
    stroke: var(--plate);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
  }

  .candidate.span {
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .candidate.label {
    fill: var(--plate);
    stroke: none;
    font-family: var(--f-mono);
    text-anchor: middle;
  }

  .loop {
    fill: none;
    stroke: var(--plate);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
  }

  .loop.dot {
    fill: var(--plate);
    stroke: none;
  }

  .flash {
    fill: var(--chalk);
    fill-opacity: 0;
    animation: flash 900ms var(--e-out);
  }

  @keyframes flash {
    30% {
      fill-opacity: 0.14;
    }
  }

  .bracket {
    pointer-events: auto;
    cursor: pointer;
  }

  .span {
    fill: none;
    stroke: var(--plate);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }

  .label {
    fill: var(--plate);
    font-family: var(--f-ui);
    font-weight: 500;
    letter-spacing: var(--tracking-micro);
    text-transform: uppercase;
  }

  .hit {
    fill: transparent;
  }

  .trace {
    stroke: var(--plate);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }

  /* The trace is drawn instantly; it does not animate along its path (§2). */
  @media (prefers-reduced-motion: reduce) {
    .flash {
      animation: none;
      fill-opacity: 0.14;
    }
  }
</style>
