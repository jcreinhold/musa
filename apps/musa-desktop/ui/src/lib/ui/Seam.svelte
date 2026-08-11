<script lang="ts">
  /**
   * The seam between the source column and the page, as a control.
   *
   * `01-visual-language.md` §7 gives the two halves one `--rule` hairline
   * between them, and that hairline is the most splitter-looking thing in the
   * window. This makes it one: drag to widen, double-click for the measure,
   * arrows for everyone who does not drag.
   *
   * It takes no space. The strip is 9px of pointer target laid *over* the
   * column's own border, negatively margined at both edges so the layout is
   * exactly what it was without it, and the border underneath is what changes
   * weight — `--rule`, `--ink-muted`, `--plate` — which is the same three-state
   * hairline the inspector's fields use (§7). No handle and no grip: adding
   * furniture to a seam that already reads as one is how a hairline becomes a
   * gutter.
   *
   * **It sizes the column it is in.** It lives inside the source column, on
   * that column's own right edge, rather than as a sibling between the two
   * halves — a seam standing outside both is page content belonging to no
   * landmark, which is exactly what a screen reader cannot place. Inside, it
   * is part of the region it moves.
   *
   * The width it reports is measured by that column and handed down, so what
   * the separator announces is what is on screen rather than what was last
   * asked for: the layout may hold the column narrower than the ask, and a
   * separator that says otherwise is confidently wrong.
   */

  /** A nudge, and a stride for holding shift. */
  const STEP = 16;
  const STRIDE = 96;

  let {
    width,
    floor,
    spare,
    label,
    onwiden,
    onreset,
  }: {
    /** What the column measures right now, in pixels. */
    width: number;
    /** The narrowest the column may be dragged, in pixels. */
    floor: number;
    /** How much more the column may take right now, measured when asked. */
    spare: () => number;
    /** The name of the thing being sized, spoken by the separator. */
    label: string;
    onwiden: (width: number) => void;
    onreset: () => void;
  } = $props();

  let node: HTMLElement | null = $state(null);
  /** Where the pointer went down, and what the column measured then. */
  let from: { x: number; width: number } | null = $state(null);

  /** As wide as it is now, plus whatever the page can still give up. */
  function ceiling(): number {
    return Math.max(floor, width + spare());
  }

  function clamp(next: number): number {
    return Math.min(Math.max(next, floor), ceiling());
  }

  function down(event: PointerEvent): void {
    // Only the primary button, and never the double-click's second press:
    // that one belongs to `ondblclick`, and starting a drag from it moves the
    // column a pixel or two before putting it back.
    if (event.button !== 0 || event.detail > 1) return;
    from = { x: event.clientX, width };
    node?.setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent): void {
    if (from === null) return;
    // Nothing is committed per frame that the composer cannot see: the width
    // *is* the feedback, so there is no ghost line and no preview to keep in
    // step with it.
    onwiden(clamp(from.width + (event.clientX - from.x)));
  }

  function up(): void {
    from = null;
  }

  /**
   * The keys, per the ARIA window-splitter pattern, with one deviation:
   * `Home` is the measure rather than the minimum. The minimum is a floor
   * nobody wants to sit at and is two seconds of `←` away; the measure is
   * where the column belongs and is the one position worth a key.
   */
  function key(event: KeyboardEvent): void {
    const stride = event.shiftKey ? STRIDE : STEP;
    switch (event.key) {
      case "ArrowLeft": {
        onwiden(clamp(width - stride));
        break;
      }
      case "ArrowRight": {
        onwiden(clamp(width + stride));
        break;
      }
      case "Home": {
        onreset();
        break;
      }
      case "End": {
        onwiden(ceiling());
        break;
      }
      default: {
        return;
      }
    }
    // Only once it is one of ours: an unhandled key is still the window's.
    event.preventDefault();
    event.stopPropagation();
  }
</script>

<!--
  A `separator` with a `tabindex` is the ARIA window-splitter widget, and the
  rule below only knows the decorative kind — the one that separates and does
  nothing. This one takes keys, which is the whole reason it may take focus.
-->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={node}
  class="seam"
  class:live={from !== null}
  role="separator"
  tabindex="0"
  aria-orientation="vertical"
  aria-label="{label} width"
  aria-valuenow={Math.round(width)}
  aria-valuemin={Math.round(floor)}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  ondblclick={onreset}
  onkeydown={key}
></div>

<style>
  /*
   * Nine pixels of target laid down the column's right edge, over the one-pixel
   * line that is already there. Absolute, so it takes no room and the layout is
   * exactly what it was before the seam existed.
   */
  .seam {
    position: absolute;
    z-index: 1;
    top: 0;
    right: 0;
    bottom: 0;
    width: 9px;
    background: none;
    border: 0;
    cursor: col-resize;
    /* The hairline it paints is the column's own border, which this sits on. */
    border-right: 1px solid transparent;
    touch-action: none;
  }

  .seam:hover {
    border-right-color: var(--ink-muted);
  }

  .seam:focus-visible {
    outline: none;
    border-right-color: var(--plate);
  }

  /* While the drag is live the seam keeps the focus weight wherever the
     pointer has gone, which is the only thing on screen saying the gesture
     still belongs to it. */
  .seam.live {
    border-right-color: var(--plate);
  }

  /*
   * Where the columns stop being columns (§7), the seam has nothing to sit
   * between: the source is a section above the page and its width is the
   * window's. A control that resizes nothing is worse than no control.
   */
  @media (max-width: 860px) {
    .seam {
      display: none;
    }
  }
</style>
