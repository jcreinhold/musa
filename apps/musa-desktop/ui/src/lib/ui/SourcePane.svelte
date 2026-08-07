<script lang="ts">
  /**
   * The source and its problems, as one column of type.
   *
   * The same object in both workspaces that show text (`01-visual-language.md`
   * §7 and §8): `--surround-in` behind one `--rule` hairline, the text set at
   * the source measure, the problems listed beneath it. Compose shows it at
   * the left edge of the body and can hide it; the Source workspace shows it
   * permanently and gives the page the rest. That is the only difference
   * between the two, which is why there is one component rather than two.
   *
   * It is also where the source answers back. Origin view highlights the
   * `motif` declaration and the `use` statement that produced what the pointer
   * is over (`04-provenance.md` §2), and a diagnostic puts the caret at the
   * place it is complaining about (`05-states.md` §5). Both arrive as spans
   * from the core; this file turns spans into marks and never reads the text.
   */
  import SourceEditor from "./SourceEditor.svelte";
  import type { Reveal } from "../state/reveal";
  import type { Diagnostic, Span } from "../state/snapshot";

  let {
    source,
    diagnostics,
    editable = false,
    highlight = [],
    reveal = null,
    onedit,
    oncaret,
    ondiagnostic,
    onhide,
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
    /**
     * How to put the column away — given only where putting it away is a
     * thing, which is Compose. The head exists for this control and for the
     * problem count beside it; without one there is nothing for it to hold.
     */
    onhide?: () => void;
  } = $props();

  const errors = $derived(diagnostics.filter((diagnostic) => diagnostic.severity === "error"));
</script>

<section class="source-pane" aria-label="Source">
  {#if onhide}
    <div class="head">
      <span class="what">Source</span>
      {#if errors.length > 0}
        <span class="problems"
          ><span class="glyph error" aria-hidden="true"></span>{errors.length}
          {errors.length === 1 ? "problem" : "problems"}</span
        >
      {/if}
      <button type="button" class="hide" onclick={onhide}>Hide</button>
    </div>
  {/if}

  <!--
    The source, set in the editor of roadmap §14.1 — highlighting derived from
    the real token list, the compiler's diagnostics in the gutter, and
    provenance marked in the one hue that ever means it. Read-only when the
    shell is not live, because a field that takes text nothing will read is a
    lie.
  -->
  <div class="field">
    <SourceEditor {source} {diagnostics} {editable} {highlight} {reveal} {onedit} {oncaret} />
  </div>

  <!--
    Problems, beneath the text they are about. With none the column shows
    nothing at all — not "0 problems" (`05-states.md` §2).
  -->
  {#if diagnostics.length > 0}
    <ul class="diagnostics">
      {#each diagnostics as diagnostic, index (index)}
        <li>
          <!--
            A diagnostic is a place, not a notification: clicking it puts the
            caret there and flashes the system it is about. Never a toast.
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
</section>

<style>
  /*
   * The source measure (`01-visual-language.md` §8). The width is set here,
   * in the face the source is actually set in, so `ch` means what it says:
   * 48 characters of Recursive Mono Linear.
   *
   * The number is the language's. Across `examples/`, ignoring comments, half
   * of all lines are 23 characters or shorter and 19 in 20 fit in 37; 48
   * clears that with eleven characters of headroom and still lets the longest
   * lines reach for the edge, which is what stops a column of type from
   * reading as a column of margin. The handful that run longer — comment
   * prose, one studio chain — scroll, as they would in any editor.
   *
   * The measure is what the column wants, not what it insists on. `--source-cap`
   * is how much room the screen it is standing in can actually spare, and the
   * column takes the smaller of the two: text that gives way is a shorter line,
   * a page that gives way stops being a page. A screen that sets no cap gets
   * 42 % of the window, which is the Source workspace, where the page has the
   * whole of the rest.
   */
  .source-pane {
    font-family: var(--f-mono);
    font-size: var(--t-value-size);
    /* 48 characters, the line-number gutter, and the column's own margins. */
    --measure: calc(48ch + 40px + var(--s-5) * 2);

    display: flex;
    flex-direction: column;
    width: min(var(--measure), var(--source-cap, 42vw));
    min-width: 0;
    min-height: 0;
    padding: var(--s-3) var(--s-5) var(--s-4);
    background: var(--surround-in);
    border-right: 1px solid var(--rule);
    overflow: hidden;
  }

  /*
   * One line of chrome, and only in Compose: what this column is, whether the
   * compiler has anything to say about it, and the way to put it away. Set in
   * the muted ink the rest of the chrome uses so it reads as a caption to the
   * text rather than as a title bar over it.
   */
  .head {
    display: flex;
    align-items: baseline;
    gap: var(--s-3);
    padding-bottom: var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  .problems {
    color: var(--chalk);
  }

  .hide {
    margin-left: auto;
    background: none;
    border: 0;
    border-radius: var(--radius-control);
    padding: 0;
    font: inherit;
    color: var(--ink-muted);
    cursor: pointer;
  }

  .hide:hover {
    color: var(--ink);
  }

  .field {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    /* The editor scrolls itself; the column does not scroll around it. */
    overflow: hidden;
  }

  .diagnostics {
    flex: 0 0 auto;
    max-height: 30%;
    margin: var(--s-3) 0 0;
    padding: var(--s-3) 0 0;
    border-top: 1px solid var(--rule);
    list-style: none;
    overflow: auto;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
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

  /*
   * Where the margins stop being margins and become sections above and below
   * the leaf, the source becomes the first of them and takes the full width:
   * a window this narrow genuinely is a stack (§7).
   */
  @media (max-width: 860px) {
    .source-pane {
      width: auto;
      /* A height, not a minimum: the editor is as tall as its document, and
         a section that grew with the file would put the page below the fold
         and move it on every keystroke. */
      height: 40vh;
      border-right: 0;
      border-bottom: 1px solid var(--rule);
    }
  }
</style>
