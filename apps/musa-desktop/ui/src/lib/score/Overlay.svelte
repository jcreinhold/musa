<script lang="ts">
  /**
   * The single overlay layer (`02-engraving.md` §8): selection and hover
   * halos, drawn in the page's own coordinates and measured in staff spaces,
   * which is why they never look pasted on.
   *
   * Selection is a `0.6sp` rounded halo in `--plate` at 18 % fill with a 1 px
   * stroke; hover is the same halo at 8 % and no stroke. Hover never moves
   * anything.
   */
  import type { Rect } from "./geometry";

  let {
    box,
    staffSpace,
    selection,
    hover,
  }: {
    /** The page's coordinate system, adopted verbatim from the engraving. */
    box: { width: number; height: number };
    /** One staff space in those coordinates. */
    staffSpace: number;
    selection: Rect[];
    hover: Rect[];
  } = $props();

  /** Halo corners round at a third of a staff space — the score's own scale. */
  const radius = $derived(staffSpace / 3);
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
</style>
