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
  import Seam from "./Seam.svelte";
  import SourceEditor from "./SourceEditor.svelte";
  import Ticked from "./Ticked.svelte";
  import { applyFix, asControl, labelOf, onlyFix, placeOf } from "../state/fix";
  import type { Snippet } from "svelte";
  import type { Reveal } from "../state/reveal";
  import type {
    Diagnostic,
    NameFacts,
    Span,
    TermFacts,
  } from "../state/snapshot";

  let {
    source,
    diagnostics,
    editable = false,
    highlight = [],
    focus = null,
    sounding = [],
    candidate = null,
    reveal = null,
    modal = false,
    width = null,
    floor = 0,
    spare,
    onwiden,
    onreset,
    onedit,
    oncaret,
    onpoint,
    onundo,
    onredo,
    onsave,
    ondiagnostic,
    onhide,
    terms = [],
    names = [],
    library = null,
    onclose,
    onlibrary,
    footer,
  }: {
    source: string;
    diagnostics: Diagnostic[];
    editable?: boolean;
    /** Every declaration in scope, for hover and completion. */
    terms?: TermFacts[];
    /** Every resolved name, for what the hover is about. */
    names?: NameFacts[];
    /**
     * The bundled module this column is showing instead of the piece
     * (`08-elaboration.md` §3). It says what it is and takes no edits.
     */
    library?: { name: string } | null;
    /** Put the library module away and go back to the piece. */
    onclose?: () => void;
    /** Follow a term into a bundled module. */
    onlibrary?: (uri: string, start: number, end: number) => void;
    /** What the column shows beneath the problems — the findings panel. */
    footer?: Snippet;
    /** Spans to mark in the source: the provenance of what is on screen. */
    highlight?: Span[];
    /** The focus, in the text: what spells the music and what placed it. */
    focus?: { definition: Span | null; place: Span | null } | null;
    /** The statements that made the music on the page in view. */
    sounding?: Span[];
    /** The token a live pointer gesture would replace, and what it would write. */
    candidate?: { start: number; end: number; text: string } | null;
    /** A place to put the caret, once, when it changes. */
    reveal?: Reveal | null;
    /** Vim mode in the editor — the composer's preference. */
    modal?: boolean;
    /**
     * What the composer asked this column to be, in pixels. `null` is the
     * source measure, which is what it opens at. The ask is not
     * the answer: a window with no room for it renders the cap instead.
     */
    width?: number | null;
    /** The narrowest the seam may drag this column. */
    floor?: number;
    /** How much more room the column may take right now, measured on demand. */
    spare?: () => number;
    /**
     * What to do with a width the composer dragged to, and how to go back to
     * the measure. Given both, the column grows a seam; given neither, it is
     * the fixed column it was.
     */
    onwiden?: (width: number) => void;
    onreset?: () => void;
    onedit?: (source: string) => void;
    /** Where the caret is now, so the score can follow it. */
    oncaret?: (offset: number) => void;
    /** Where the pointer is in the text, so the page can mark what it wrote. */
    onpoint?: (line: { from: number; to: number } | null) => void;
    /** What vim's `u`, `⌃r`, and `:w` reach: the project's own commands. */
    onundo?: () => void;
    onredo?: () => void;
    onsave?: () => void;
    ondiagnostic?: (diagnostic: Diagnostic) => void;
    /**
     * How to put the column away — given only where putting it away is a
     * thing, which is Compose. The head exists for this control and for the
     * problem count beside it; without one there is nothing for it to hold.
     */
    onhide?: () => void;
  } = $props();

  const errors = $derived(
    diagnostics.filter((diagnostic) => diagnostic.severity === "error"),
  );

  /**
   * What this column actually measures, for the seam to speak and to drag
   * from — which is not the same as `width`, the ask: the layout may hold the
   * column narrower than what was asked for.
   */
  let measured = $state(0);

  /**
   * Apply a diagnostic's fix by rewriting the source, the same way a keystroke
   * does. There is no separate edit path and there should not be: an applied
   * fix is undoable with `⌘Z` because it is an ordinary edit.
   */
  function fix(diagnostic: Diagnostic): void {
    const only = onlyFix(diagnostic);
    if (!only || !editable) return;
    onedit?.(applyFix(source, only));
  }
</script>

<section
  class="source-pane"
  aria-label="Source"
  style={width === null
    ? undefined
    : `--asked: min(${width}px, var(--source-room, 60vw))`}
  bind:clientWidth={measured}
>
  <!--
    A bundled module says what it is, in words and not only in a shape: it is
    library material, it is read-only, and closing it comes back to the piece
    (`08-elaboration.md` §3). The head is always present here, even where the
    piece's would not be, because a document that cannot be written into has
    to say so somewhere.
  -->
  {#if library}
    <div class="head library">
      <span class="what">{library.name}</span>
      <span class="badge">Standard library — read-only</span>
      {#if onclose}<button type="button" class="hide" onclick={onclose}
          >Close</button
        >{/if}
    </div>
  {:else if onhide}
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
    <SourceEditor
      {source}
      {diagnostics}
      {editable}
      {highlight}
      {focus}
      {sounding}
      {candidate}
      {reveal}
      {modal}
      {onedit}
      {oncaret}
      {onpoint}
      {onundo}
      {onredo}
      {onsave}
      {terms}
      {names}
      {onlibrary}
    />
  </div>

  <!--
    Problems, beneath the text they are about. With none the column shows
    nothing at all — not "0 problems" (`05-states.md` §2).
  -->
  {#if diagnostics.length > 0}
    <ul class="diagnostics">
      {#each diagnostics as diagnostic, index (index)}
        {@const place = placeOf(diagnostic)}
        {@const label = labelOf(diagnostic)}
        {@const only = onlyFix(diagnostic)}
        <li>
          <!--
            A diagnostic is a place, not a notification: clicking it puts the
            caret there and flashes the system it is about. Never a toast.

            Three lines at most, in the order they are read: what is wrong,
            what is wrong at that character, and what to do. The label is not
            the message again — it says something the message does not — and
            the help line only appears when there is advice worth a line.
          -->
          <button
            type="button"
            class="problem"
            onclick={() => ondiagnostic?.(diagnostic)}
          >
            <span class="glyph {diagnostic.severity}" aria-hidden="true"></span>
            <span class="message"><Ticked text={diagnostic.message} /></span>
            {#if place}<span class="where">{place}</span>{/if}
          </button>
          {#if label}<p class="label"><Ticked text={label} /></p>{/if}
          {#if diagnostic.help}<p class="help">
              <Ticked text={diagnostic.help} />
            </p>{/if}
          <!--
            One fix, or none. A control that applies "the fix" when there are
            two is how an editor applies the wrong one, so `onlyFix` refuses
            to choose.
          -->
          {#if only && editable}
            <button type="button" class="fix" onclick={() => fix(diagnostic)}
              ><Ticked text={asControl(only.title)} /></button
            >
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <!-- A reading of the piece, beneath the problems with it (§5's contrast). -->
  {#if footer}{@render footer()}{/if}

  <!--
    The seam, last so it is over everything, and inside the column so it is
    inside the column's landmark.
  -->
  {#if onwiden && onreset}
    <Seam
      label="Source"
      width={measured}
      {floor}
      spare={spare ?? (() => 0)}
      {onwiden}
      {onreset}
    />
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
   *
   * `--asked` is the width the composer dragged the seam to, and
   * it replaces the whole expression rather than sitting inside it. `--source-cap`
   * keeps the *default* from crushing a small window — there the column is
   * asking for room nobody granted it — and a composer who drags the seam has
   * granted it. What still bounds the ask is `--source-room`: the same
   * arithmetic with the page's own floor rather than the automatic one, so a
   * width chosen on a large display narrows on a laptop and comes back whole.
   */
  .source-pane {
    /* The seam is positioned against this edge. */
    position: relative;
    font-family: var(--f-mono);
    font-size: var(--t-value-size);
    /* 48 characters, the line-number gutter, and the column's own margins. */
    --measure: calc(48ch + 40px + var(--s-5) * 2);

    display: flex;
    flex-direction: column;
    width: var(--asked, min(var(--measure), var(--source-cap, 42vw)));
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

  /*
   * A library module is named the way the language names it, in the mono face
   * a module path is written in, and the badge beside it says in words what
   * the missing caret says by absence.
   */
  .head.library .what {
    font-family: var(--f-mono);
    color: var(--ink);
  }

  .badge {
    font-size: var(--t-micro-size);
    color: var(--ink-faint);
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
   * The label and the help sit under the message, indented to clear the
   * severity glyph so the three read as one block rather than three rows.
   * Both are quieter than the claim they hang off: the message is what
   * happened, these are the detail that follows from it.
   */
  .label,
  .help {
    margin: 0 0 0 calc(7px + var(--s-1));
    color: var(--ink-muted);
  }

  .help::before {
    /* Named rather than styled: the reader should be able to tell the advice
       from the observation without knowing the colour scheme. */
    content: "help: ";
  }

  /*
   * The fix. `--plate` because it is the application acting for you, the same
   * hue every derived-or-live thing wears; a hairline underline because
   * §7 of `01-visual-language.md` allows exactly that and no box.
   */
  .fix {
    margin: var(--s-1) 0 0 calc(7px + var(--s-1));
    background: none;
    border: 0;
    border-bottom: 1px solid var(--rule);
    border-radius: 0;
    padding: 0;
    font: inherit;
    color: var(--plate);
    cursor: pointer;
  }

  .fix:hover {
    border-bottom-color: var(--ink-muted);
  }

  .fix:focus-visible {
    border-bottom-color: var(--plate);
  }

  /* One problem is one block; the next one starts far enough away to read as
     a different problem. */
  .diagnostics li + li {
    margin-top: var(--s-2);
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
