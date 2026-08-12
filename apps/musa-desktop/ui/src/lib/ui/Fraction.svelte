<script lang="ts">
  /**
   * Exact time, set as a real fraction with a true diagonal bar
   * (docs/rules/desktop/01-visual-language.md §3). `7/8`, never `0.875`: musa's
   * time is rational, and the interface says so wherever it appears.
   */
  import type { Fraction } from "../state/snapshot";

  let { value, title }: { value: Fraction; title?: string } = $props();
</script>

<span class="fraction" {title} aria-label="{value.numerator} over {value.denominator}">
  <span class="num">{value.numerator}</span><span class="bar" aria-hidden="true">/</span><span class="den"
    >{value.denominator}</span
  >
</span>

<style>
  .fraction {
    font-family: var(--f-mono);
    font-size: var(--t-value-size);
    line-height: var(--t-value-line);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* The numerals sit either side of the bar rather than above and below it: a
     stacked fraction at 13px is a smudge, a diagonal one is legible. The bar
     is the mono face's own solidus — musa's `/` is a fraction bar, and the
     face was chosen partly because it does not prettify it into anything
     else. */
  .num,
  .den {
    font-feature-settings: "tnum" 1;
  }

  .bar {
    color: var(--ink-muted);
  }
</style>
