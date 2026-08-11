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
   *
   * It is one line of text that may take more than one line to print. A
   * copyright is as long as its licence and a title is as long as its piece,
   * and neither is a length the interface gets to choose — so when the value
   * will not fit the column, the value wraps. It does not scroll inside itself
   * and it does not truncate: `© 2026. Licensed CC BY-` is not a shorter
   * statement of the same thing, it is a different one, and a field that shows
   * it is lying about what the document says. This is the rule the top margin
   * already follows for the same reason (`01-visual-language.md` §7) — what
   * will not fit takes another row.
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
     * button.
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

  /**
   * One line, whatever arrives.
   *
   * The field wraps, so a pasted newline would look like it belonged — and it
   * would reach the source as a break inside a statement. Folding the break
   * into a space keeps what was pasted readable and keeps it one value.
   */
  function oninput(event: Event): void {
    const typed = (event.currentTarget as HTMLTextAreaElement).value;
    if (typed.includes("\n")) draft = typed.replace(/\s*\n\s*/g, " ");
  }

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
      (event.currentTarget as HTMLTextAreaElement).blur();
    }
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      draft = value;
      editing = false;
      (event.currentTarget as HTMLTextAreaElement).blur();
    }
  }
</script>

<!--
  The field and a hidden copy of its own text, stacked in one grid cell. The
  copy is what has a size — it is ordinary flowed text, so it takes the width
  it needs, wraps when the column is narrower than that, and is exactly as
  tall as the result. The field stretches to it.

  This is one mechanism for two places that want opposite things. In the top
  margin, where `4/4` sits in a row of controls, the copy is short and the
  field is short with it — the band wraps its groups whole and no field should
  wrap inside one. In the inspector's 192px column the same copy runs to three
  lines and the field is three lines tall. Neither case is special-cased and
  neither measures anything in script: the browser does the layout, once, in
  the same pass as everything else.
-->
<div class="field">
  <span class="sizer" aria-hidden="true">{draft || placeholder || ""}&nbsp;</span>
  <textarea
    class="entry"
    rows="1"
    cols="1"
    aria-label={label}
    {placeholder}
    spellcheck="false"
    autocomplete="off"
    bind:value={draft}
    onfocus={() => (editing = true)}
    onblur={commit}
    {oninput}
    {onkeydown}
  ></textarea>
</div>

<style>
  .field {
    display: inline-grid;
    /* Not `100%`: a field is as wide as its value up to the room it has, and
       stretching a two-character meter across the column would put its
       underline under nothing. */
    max-width: 100%;
    min-width: 3ch;
    vertical-align: top;
  }

  .field > * {
    grid-area: 1 / 1;
    font: inherit;
    /* Stated separately: the `font` shorthand carries a line height, but the
       UA stylesheet overrides it on form controls, and a field one pixel
       taller than the text it replaces moves everything under it. */
    line-height: inherit;
    /* The two have to break identically or the copy is not a copy. */
    white-space: pre-wrap;
    overflow-wrap: break-word;
    text-align: inherit;
  }

  /* Present to the layout, absent to everything else — including the caret,
     which would otherwise find it on the way through. */
  .sizer {
    visibility: hidden;
    pointer-events: none;
    user-select: none;
  }

  /* `rows`/`cols` are the intrinsic size of a textarea, and the default is a
     20×2 box — which would size the grid track and make every field twenty
     characters wide whatever it holds. One by one hands the sizing to the
     copy, which is the whole idea. */
  .entry {
    min-width: 0;
    background: none;
    border: 0;
    padding: 0;
    margin: 0;
    color: inherit;
    resize: none;
    /* The grid cell is already the height of the text. Anything left to
       scroll here would be the bug this replaced. */
    overflow: hidden;
  }

  .entry:focus {
    outline: none;
  }

  /* An unfilled role is an em dash where the value goes, not a blank: the row
     is saying "this is a thing a piece can name", which a blank does not. */
  .entry::placeholder {
    color: var(--ink-muted);
    opacity: 1;
  }
</style>
