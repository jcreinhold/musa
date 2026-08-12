<script lang="ts">
  /**
   * A musical position, `bar:beat`, bar dominant
   * (docs/interface/01-visual-language.md §3). A reader scans for the bar, so
   * the bar carries the weight and the beat sits one step down in
   * `--ink-muted`. A bare tick counter never appears in the UI.
   */
  import type { Fraction as Rational } from "../state/snapshot";
  import Fraction from "./Fraction.svelte";

  let {
    bar,
    beat,
    size = "large",
  }: { bar: number; beat: Rational; size?: "large" | "value" } = $props();

  const whole = $derived(beat.denominator === 1);
</script>

<!--
  `img` because it is a graphic with a name: the parts are set typographically
  and hidden from assistive technology, and the whole is read as one phrase.
-->
<span
  class="position {size}"
  role="img"
  aria-label="bar {bar} beat {beat.numerator}/{beat.denominator}"
>
  <span class="bar" aria-hidden="true">{bar}</span><span
    class="colon"
    aria-hidden="true">:</span
  ><span class="beat" aria-hidden="true">
    {#if whole}{beat.numerator}{:else}<Fraction value={beat} />{/if}
  </span>
</span>

<style>
  .position {
    font-family: var(--f-mono);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    display: inline-flex;
    align-items: baseline;
  }

  .large .bar {
    font-size: var(--t-large-size);
    line-height: var(--t-large-line);
    font-weight: 300;
  }

  .large .colon,
  .large .beat {
    font-size: var(--t-value-size);
    line-height: var(--t-value-line);
  }

  .value .bar,
  .value .colon,
  .value .beat {
    font-size: var(--t-value-size);
    line-height: var(--t-value-line);
  }

  .colon,
  .beat {
    color: var(--ink-muted);
  }

  .colon {
    padding: 0 0.08em;
  }
</style>
