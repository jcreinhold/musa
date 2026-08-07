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
  }: {
    /** The current value, as the source spells it. */
    value: string;
    /** What this field is, for a screen reader. */
    label: string;
    /** Called with the new text when the composer commits it. */
    onchange: (next: string) => void;
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
    if (next !== "" && next !== value) onchange(next);
    else draft = value;
  }

  function onkeydown(event: KeyboardEvent): void {
    // The score's own keys must not fire while a field has focus, and the
    // application's handler decides that by target — so this only has to stop
    // Return and Escape from travelling any further.
    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      commit();
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
  spellcheck="false"
  autocomplete="off"
  size={Math.max(draft.length, 3)}
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
    color: inherit;
    min-width: 3ch;
    max-width: 100%;
  }

  .field:focus {
    outline: none;
  }
</style>
