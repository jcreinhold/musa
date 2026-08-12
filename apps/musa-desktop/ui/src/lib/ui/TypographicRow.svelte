<script lang="ts">
  /**
   * One inspector row: a `--t-micro` label over a value
   * (docs/rules/desktop/01-visual-language.md §7).
   *
   * There are no field boxes. An editable value carries a hairline underline
   * in three weights — `--rule` at rest, `--ink-muted` under the pointer,
   * `--plate` while it has focus — and that is the entire affordance. It is
   * there at rest on purpose: an affordance that appears only on hover can be
   * found only by someone who already suspected it was there, which is not
   * discovery. A row that only reports a value gets no underline,
   * and the difference between the two kinds of row is then readable without
   * touching either.
   */
  import type { Snippet } from "svelte";

  let {
    label,
    editable = false,
    trailing,
    children,
  }: {
    label: string;
    editable?: boolean;
    trailing?: Snippet;
    children: Snippet;
  } = $props();
</script>

<div class="row">
  <div class="label">{label}</div>
  <div class="line">
    <div class="value" class:editable>{@render children()}</div>
    {#if trailing}<div class="trailing">{@render trailing()}</div>{/if}
  </div>
</div>

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: var(--s-1);
  }

  .label {
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    font-weight: 500;
    letter-spacing: var(--tracking-micro);
    text-transform: uppercase;
    color: var(--ink-muted);
  }

  .line {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-2);
  }

  .value {
    font-family: var(--f-mono);
    font-size: var(--t-value-size);
    line-height: var(--t-value-line);
    color: var(--ink);
    min-width: 0;
    border-bottom: 1px solid transparent;
  }

  .editable {
    border-bottom-color: var(--rule);
  }

  .editable:hover {
    border-bottom-color: var(--ink-muted);
  }

  .editable:focus-within {
    border-bottom-color: var(--plate);
  }

  .trailing {
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
    white-space: nowrap;
  }
</style>
