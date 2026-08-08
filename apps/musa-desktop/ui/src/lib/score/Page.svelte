<script lang="ts">
  /**
   * One page of the score: a placeholder until it is near the viewport, the
   * engraving once it is, and the overlay layer over it.
   *
   * The placeholder is sized from the layout result, not from the page that
   * has not arrived yet, so the scrollbar is honest from the first layout and
   * does not grow as pages render (`02-engraving.md` §7).
   *
   * A page that is re-engraved cross-fades: the outgoing SVG stays in the
   * document under the incoming one for 90 ms, so there is never a white
   * frame and never a collapse to zero height (§6). Under
   * `prefers-reduced-motion` the swap is instant, which is the same guarantee
   * without the animation.
   */
  import type { Box, PageSvg } from "../engrave/engraver";
  import Overlay from "./Overlay.svelte";
  import type { Marks } from "./geometry";

  let {
    number,
    page,
    box,
    scale,
    staffSpace,
    marks,
    onoccurrence,
  }: {
    /** 1-based page number, which the observer reports back. */
    number: number;
    /** The engraving, once it is resident. */
    page?: PageSvg;
    /** The layout's page box, which sizes the placeholder. */
    box: Box;
    /**
     * CSS pixels per page unit, when the caller fixes the scale rather than
     * the width. Continuous view needs this: its page is as wide as the music
     * happens to be, so a page sized by its container would be drawn at
     * whatever magnification that width implied (§4).
     */
    scale?: number;
    staffSpace: number;
    /** What is drawn over this page, measured in its own coordinates. */
    marks: Marks;
    /** Select an occurrence by clicking its Origin-view bracket. */
    onoccurrence?: (id: string) => void;
  } = $props();

  /** How long the cross-fade lasts (`01-visual-language.md` §6). */
  const FADE_MS = 90;

  let outgoing = $state<string | undefined>();
  let showing = $state<string | undefined>();
  let fade: ReturnType<typeof setTimeout> | undefined;

  const ratio = $derived(box.height > 0 ? box.width / box.height : 210 / 297);

  $effect(() => {
    const arriving = page?.svg;
    if (arriving === showing) return;
    const instant =
      showing === undefined ||
      globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;
    outgoing = instant ? undefined : showing;
    showing = arriving;
    clearTimeout(fade);
    if (!instant) fade = setTimeout(() => (outgoing = undefined), FADE_MS);
  });
</script>

<div
  class="page"
  data-page={number}
  style:aspect-ratio={scale === undefined ? ratio : undefined}
  style:width={scale === undefined ? undefined : `${box.width * scale}px`}
  style:height={scale === undefined ? undefined : `${box.height * scale}px`}
>
  {#if outgoing}
    <div class="ink outgoing">{@html outgoing}</div>
  {/if}
  {#if showing && page}
    <div class="ink arriving">
      {@html showing}
      <Overlay
        box={page.box}
        {staffSpace}
        selection={marks.selection}
        hover={marks.hover}
        focus={marks.focus}
        candidate={marks.candidate}
        playing={marks.playing}
        caret={marks.caret}
        loop={marks.loop}
        flash={marks.flash}
        brackets={marks.brackets}
        trace={marks.trace}
        {onoccurrence}
      />
    </div>
  {/if}
</div>

<style>
  .page {
    position: relative;
    /* Overridden inline when the caller fixes the scale instead. */
    width: 100%;
    flex: none;
    /* The placeholder is the page: same box, no ink. Nothing is drawn for an
       absent page, because an absent page looks like blank paper. */
  }

  .ink {
    position: absolute;
    inset: 0;
  }

  /*
   * The outgoing page is not faded out — it is simply removed once the
   * incoming one has finished fading in over it. Fading both would show the
   * leaf through the middle of the swap, which is the white frame §6 forbids.
   */
  .arriving {
    animation: ink var(--m-fast) linear;
  }

  @keyframes ink {
    from {
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .arriving {
      animation: none;
    }
  }
</style>
