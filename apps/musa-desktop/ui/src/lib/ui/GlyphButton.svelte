<script lang="ts">
  /**
   * A control whose visible mark is a SMuFL glyph from the score's own font,
   * with a real accessible name. Musa ships no icon font of its own: if a
   * symbol is not something an engraver already draws, it is set as a word.
   */
  let {
    glyph,
    label,
    disabled = false,
    active = false,
    onclick,
  }: {
    /** A SMuFL codepoint, rendered in Bravura. */
    glyph: string;
    /** The accessible name; also the tooltip. */
    label: string;
    disabled?: boolean;
    active?: boolean;
    onclick?: () => void;
  } = $props();
</script>

<button
  type="button"
  class="glyph"
  class:active
  {disabled}
  aria-label={label}
  aria-pressed={active}
  title={label}
  {onclick}
>
  <span aria-hidden="true">{glyph}</span>
</button>

<style>
  .glyph {
    font-family: var(--f-notation);
    font-size: var(--t-name-size);
    line-height: 1;
    background: none;
    border: 0;
    border-radius: var(--radius-control);
    color: var(--ink-muted);
    padding: var(--s-1) var(--s-2);
    cursor: pointer;
  }

  .glyph:hover:not(:disabled) {
    color: var(--ink);
  }

  .glyph.active {
    color: var(--plate);
  }

  .glyph:disabled {
    color: var(--ink-faint);
    cursor: default;
  }
</style>
