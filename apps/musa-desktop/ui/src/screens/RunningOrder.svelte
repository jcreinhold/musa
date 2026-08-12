<script lang="ts">
  /**
   * The running order, in the left margin, above the parts
   * (`docs/rules/desktop/07-the-volume.md`).
   *
   * The margin reads outside in: volume, piece, structure. This is the first
   * of the three, and it is `Outline.svelte`'s idiom unchanged — right-aligned
   * rows, `--ink-muted` → `--ink` on hover → `--plate` when current, and the
   * coordinate in the trailing slot, which for a volume is the position in the
   * running order rather than a bar number.
   *
   * With one piece in the project nothing renders. A loose `.musa` file must
   * be indistinguishable from what it was before projects existed, and a list
   * of one is not a list.
   */
  import type { ContentsFacts } from "../lib/state/snapshot";

  let {
    contents,
    onchoose,
  }: {
    contents: ContentsFacts;
    onchoose: (file: string) => void;
  } = $props();

  const numeral = (index: number) => String(index + 1).padStart(2, "0");
</script>

{#if contents.pieces.length > 1}
  <nav class="order" aria-label="Contents">
    {#each contents.pieces as entry, index (entry.file)}
      <button
        type="button"
        class="row"
        class:current={entry.current}
        aria-current={entry.current ? "page" : undefined}
        onclick={() => onchoose(entry.file)}
      >
        <span class="name">{entry.title}</span>
        {#if entry.unsaved}<span class="note">edited</span>{/if}
        <span class="position" aria-hidden="true">{numeral(index)}</span>
      </button>
    {/each}
  </nav>
{/if}

<style>
  .order {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--s-1);
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

  .row.current {
    color: var(--plate);
    text-decoration: underline;
    text-underline-offset: 0.35em;
  }

  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .note {
    font-size: var(--t-micro-size);
    letter-spacing: var(--tracking-micro);
    color: var(--ink-muted);
    white-space: nowrap;
  }

  /* The position is a coordinate, not a word: it sets like one, in the slot
     the outline puts a bar number in. */
  .position {
    font-family: var(--f-score-text);
    font-variant-numeric: tabular-nums;
    color: var(--ink-faint);
    opacity: 0.75;
  }
</style>
