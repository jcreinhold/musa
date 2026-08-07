<script lang="ts">
  /**
   * The piece's structure, under the parts list: form markers and named
   * phrases, in the order they are played (`docs/prompts/35`).
   *
   * It is a table of contents, not an editor. Annotations are written in the
   * source — there is no way to add a section from here, deliberately — so
   * every row does one thing: take you to the place it names. A phrase sits
   * indented under the section it falls in, which is the shape a composer
   * already has in their head.
   *
   * Nothing here is derived: the bar, the beat, and the notehead to reveal
   * were all computed by the core (`03-interaction.md` §7).
   */
  import type { OutlineFacts } from "../lib/state/snapshot";

  let {
    outline,
    active = [],
    onselect,
  }: {
    outline: OutlineFacts[];
    /**
     * The rows the selection is inside, by identity: both come from
     * `outline`. A phrase and the section it sits in are both true at once,
     * which is why this is a list rather than a row.
     */
    active?: OutlineFacts[];
    onselect: (row: OutlineFacts) => void;
  } = $props();

  const key = (row: OutlineFacts) => `${row.kind}:${row.name}:${row.bar}`;
</script>

{#if outline.length > 0}
  <nav class="outline" aria-label="Structure">
    {#each outline as row (key(row))}
      <button
        type="button"
        class="row"
        class:phrase={row.kind === "phrase"}
        class:active={active.includes(row)}
        onclick={() => onselect(row)}
      >
        <span class="name">{row.name}</span>
        <span class="bar" aria-label="bar {row.bar}">{row.bar}</span>
      </button>
    {/each}
  </nav>
{/if}

<style>
  .outline {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--s-1);
    /*
     * The parts list ends where the staves do; the outline is a separate
     * reading of the same page, so it starts after a rest rather than a rule.
     */
    margin-top: var(--s-8);
    width: 100%;
  }

  .row {
    display: flex;
    align-items: baseline;
    justify-content: flex-end;
    gap: var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
    background: none;
    border: 0;
    border-radius: var(--radius-control);
    padding: 0 var(--s-1);
    cursor: pointer;
    text-align: right;
  }

  .row:hover {
    color: var(--ink);
  }

  .row.active {
    color: var(--plate);
  }

  /* A phrase belongs to the section above it, and reads as its subordinate. */
  .phrase .name {
    font-style: italic;
    padding-right: var(--s-2);
  }

  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The bar number is a coordinate, not a word: it sets like one. */
  .bar {
    font-family: var(--f-score-text);
    font-variant-numeric: tabular-nums;
    color: var(--ink-faint);
    opacity: 0.75;
  }
</style>
