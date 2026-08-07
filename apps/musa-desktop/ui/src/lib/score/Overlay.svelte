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
  import { TICK_SPACES, type Bracket, type Rect, type Trace } from "./geometry";

  let {
    box,
    staffSpace,
    selection,
    hover,
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

  .playhead,
  .caret {
    stroke: var(--plate);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
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
