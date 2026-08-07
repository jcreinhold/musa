<script lang="ts">
  /**
   * The bottom drawer: source and diagnostics. It slides up from the surround
   * and **pushes** the leaf; it never overlaps it (`01-visual-language.md`
   * §7). Closed by default in Compose.
   *
   * With no diagnostics the pane shows nothing at all — not "0 problems".
   * Absence is the message (`05-states.md` §2).
   *
   * It is also where the source answers back. Origin view highlights the
   * `motif` declaration and the `use` statement that produced what the pointer
   * is over (`04-provenance.md` §2), and a diagnostic puts the caret at the
   * place it is complaining about (`05-states.md` §5). Both arrive as spans
   * from the core; this file turns spans into marks and never reads the text.
   */
  import SourceEditor from "../lib/ui/SourceEditor.svelte";
  import type { Reveal } from "../lib/state/reveal";
  import type { Diagnostic, Span } from "../lib/state/snapshot";

  let {
    source,
    diagnostics,
    editable = false,
    highlight = [],
    reveal = null,
    onedit,
    oncaret,
    ondiagnostic,
    open = $bindable(false),
  }: {
    source: string;
    diagnostics: Diagnostic[];
    editable?: boolean;
    /** Spans to mark in the source: the provenance of what is on screen. */
    highlight?: Span[];
    /** A place to put the caret, once, when it changes. */
    reveal?: Reveal | null;
    onedit?: (source: string) => void;
    /** Where the caret is now, so the score can follow it (prompt 26). */
    oncaret?: (offset: number) => void;
    ondiagnostic?: (diagnostic: Diagnostic) => void;
    open?: boolean;
  } = $props();

  const errors = $derived(diagnostics.filter((diagnostic) => diagnostic.severity === "error"));
</script>

<div class="drawer" class:open>
  <button type="button" class="handle" onclick={() => (open = !open)} aria-expanded={open}>
    <span class="what">Source</span>
    {#if errors.length > 0}
      <span class="problems"
        ><span class="glyph error" aria-hidden="true"></span>{errors.length}
        {errors.length === 1 ? "problem" : "problems"}</span
      >
    {/if}
    <span class="hint">{open ? "Hide" : "Show"}</span>
  </button>

  {#if open}
    <div class="panes">
      <!--
        The source, set in the editor of roadmap §14.1 — highlighting derived
        from the real token list, the compiler's diagnostics in the gutter,
        and provenance marked in the one hue that ever means it. Read-only
        when the shell is not live, because a field that takes text nothing
        will read is a lie.
      -->
      <div class="field">
        <SourceEditor
          {source}
          {diagnostics}
          {editable}
          {highlight}
          {reveal}
          {onedit}
          {oncaret}
        />
      </div>
      {#if diagnostics.length > 0}
        <ul class="diagnostics">
          {#each diagnostics as diagnostic, index (index)}
            <li>
              <!--
                A diagnostic is a place, not a notification: clicking it puts
                the caret there and flashes the system it is about
                (`05-states.md` §5). It is never a toast.
              -->
              <button type="button" class="problem" onclick={() => ondiagnostic?.(diagnostic)}>
                <span class="glyph {diagnostic.severity}" aria-hidden="true"></span>
                <span class="message">{diagnostic.message}</span>
                {#if diagnostic.span}<span class="where">{diagnostic.span.start}</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>

<style>
  .drawer {
    display: flex;
    flex-direction: column;
  }

  /*
   * Open, the drawer is a shelf of a fixed size, not a box that grows with
   * its text. A drawer that resized itself as the source changed would move
   * the page under the composer on every keystroke, and the page not moving
   * is the whole point (`01-visual-language.md` §7).
   */
  .drawer.open {
    height: 34vh;
  }

  .handle {
    display: flex;
    align-items: baseline;
    gap: var(--s-3);
    background: none;
    border: 0;
    padding: var(--s-2) var(--s-6);
    color: var(--ink-muted);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    cursor: pointer;
    text-align: left;
  }

  .handle:hover {
    color: var(--ink);
  }

  .hint {
    margin-left: auto;
    color: var(--ink-muted);
  }

  .problems {
    color: var(--chalk);
  }

  /* Severity is a shape as well as a colour: filled for errors, hollow for
     warnings (`03-interaction.md` §5). */
  .glyph {
    display: inline-block;
    width: 7px;
    height: 7px;
    margin-right: var(--s-1);
  }

  .glyph.error {
    background: var(--chalk);
  }

  .glyph.warning {
    border: 1px solid var(--chalk);
  }

  .panes {
    flex: 1;
    display: flex;
    gap: var(--s-6);
    min-height: 0;
    padding: 0 var(--s-6) var(--s-4);
  }

  .field {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    /* The editor scrolls itself; the drawer does not scroll around it. */
    overflow: hidden;
  }

  .diagnostics {
    flex: 1;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--t-body-size);
    line-height: var(--t-body-line);
  }

  .problem {
    display: block;
    width: 100%;
    background: none;
    border: 0;
    border-radius: var(--radius-control);
    padding: 0;
    font: inherit;
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }

  .problem:hover .message,
  .problem:focus-visible .message {
    text-decoration: underline;
  }

  .where {
    font-family: var(--f-mono);
    color: var(--ink-muted);
    margin-left: var(--s-2);
  }
</style>
