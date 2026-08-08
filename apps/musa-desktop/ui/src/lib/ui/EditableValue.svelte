<script lang="ts">
  /**
   * An inspector value a composer can type into.
   *
   * No box, no button, no edit affordance at rest — the row's underline is
   * the whole signal (`01-visual-language.md` §7). It reads at rest and
   * writes on commit: `Return` sends it, `Esc` puts it back, and blurring
   * commits, because a value the user typed and clicked away from was meant.
   *
   * What it sends is text in the language's own spelling. It parses nothing
   * and validates nothing: an impossible pitch comes back from the core as a
   * refusal, which is the one place that judgement lives.
   */

  let {
    value,
    label,
    onchange,
    allowEmpty = false,
    placeholder,
  }: {
    /** The current value, as the source spells it. */
    value: string;
    /** What this field is, for a screen reader. */
    label: string;
    /** Called with the new text when the composer commits it. */
    onchange: (next: string) => void;
    /**
     * Whether emptying the field is a change rather than a cancellation.
     *
     * A note's pitch cannot be nothing, so clearing that field means "I
     * changed my mind". One of the piece's own statements can: clearing the
     * composer row takes the composer off the page. Adding and removing a
     * line of front matter are then the same gesture, and neither needs a
     * button (prompt 54).
     */
    allowEmpty?: boolean;
    /** What stands in the field's place while it is empty. */
    placeholder?: string;
  } = $props();

  let draft = $state("");
  let editing = $state(false);

  // While the composer is typing, the field is theirs; the moment they are
  // done it goes back to following the document.
  $effect(() => {
    if (!editing) draft = value;
  });

  function commit(): void {
    editing = false;
    const next = draft.trim();
    if (next !== value && (allowEmpty || next !== "")) onchange(next);
    else draft = value;
  }

  function onkeydown(event: KeyboardEvent): void {
    // The score's own keys must not fire while a field has focus, and the
    // application's handler decides that by target — so this only has to stop
    // Return and Escape from travelling any further.
    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      // Blur only: blurring is already a commit, and committing here as well
      // would send the same edit twice — two revisions, and two undos to take
      // one change back.
      (event.currentTarget as HTMLInputElement).blur();
    }
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      draft = value;
      editing = false;
      (event.currentTarget as HTMLInputElement).blur();
    }
  }
</script>

<input
  class="field"
  type="text"
  aria-label={label}
  {placeholder}
  spellcheck="false"
  autocomplete="off"
  size={Math.max(draft.length, placeholder?.length ?? 0, 3)}
  bind:value={draft}
  onfocus={() => (editing = true)}
  onblur={commit}
  {onkeydown}
/>

<style>
  .field {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    /* Stated separately: the `font` shorthand carries a line height, but the
       UA stylesheet overrides it on inputs, and a field one pixel taller than
       the text it replaces moves everything under it. */
    line-height: inherit;
    color: inherit;
    min-width: 3ch;
    max-width: 100%;
  }

  .field:focus {
    outline: none;
  }

  /* An unfilled role is an em dash where the value goes, not a blank: the row
     is saying "this is a thing a piece can name", which a blank does not. */
  .field::placeholder {
    color: var(--ink-muted);
    opacity: 1;
  }
</style>
