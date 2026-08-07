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
  import { untrack } from "svelte";

  import type { Diagnostic, Span } from "../lib/state/snapshot";

  let {
    source,
    diagnostics,
    editable = false,
    highlight = [],
    reveal = null,
    onedit,
    ondiagnostic,
    open = $bindable(false),
  }: {
    source: string;
    diagnostics: Diagnostic[];
    editable?: boolean;
    /** Spans to mark in the source: the provenance of what is on screen. */
    highlight?: Span[];
    /** A span to put the caret at and scroll to, once, when it changes. */
    reveal?: Span | null;
    onedit?: (source: string) => void;
    ondiagnostic?: (diagnostic: Diagnostic) => void;
    open?: boolean;
  } = $props();

  const errors = $derived(diagnostics.filter((diagnostic) => diagnostic.severity === "error"));

  /**
   * The source cut into marked and unmarked pieces.
   *
   * Overlapping spans are merged rather than nested, because a mark inside a
   * mark reads as a darker mark and means nothing.
   */
  const pieces = $derived.by(() => {
    const wanted = highlight
      .filter((span) => span.end > span.start)
      .sort((a, b) => a.start - b.start);
    const merged: Span[] = [];
    for (const span of wanted) {
      const last = merged[merged.length - 1];
      if (last && span.start <= last.end) last.end = Math.max(last.end, span.end);
      else merged.push({ ...span });
    }
    const cut: { text: string; marked: boolean }[] = [];
    let at = 0;
    for (const span of merged) {
      if (span.start > at) cut.push({ text: source.slice(at, span.start), marked: false });
      cut.push({ text: source.slice(span.start, span.end), marked: true });
      at = span.end;
    }
    cut.push({ text: source.slice(at), marked: false });
    return cut;
  });

  let area = $state<HTMLTextAreaElement | undefined>();
  let ghost = $state<HTMLElement | undefined>();
  let page = $state<HTMLElement | undefined>();

  /** The ghost carries the marks; the textarea carries the text over it. */
  function sync(): void {
    if (!ghost || !area) return;
    ghost.scrollTop = area.scrollTop;
    ghost.scrollLeft = area.scrollLeft;
  }

  /**
   * Put the caret where the caller pointed and bring it into view.
   *
   * In an editable source that is a real text selection — the caret a composer
   * would then type at. In a read-only one there is no caret, so the marked
   * run is scrolled to instead.
   */
  $effect(() => {
    const span = reveal;
    void open;
    void pieces;
    if (!span) return;
    untrack(() => {
      const field = area;
      if (field) {
        field.focus();
        field.setSelectionRange(span.start, span.end);
        const marked = ghost?.querySelector("mark");
        if (marked instanceof HTMLElement) {
          field.scrollTop = Math.max(marked.offsetTop - field.clientHeight / 3, 0);
          sync();
        }
        return;
      }
      const marked = page?.querySelector("mark");
      if (marked instanceof HTMLElement && page) {
        page.scrollTop = Math.max(marked.offsetTop - page.clientHeight / 3, 0);
      }
    });
  });
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
      {#if editable}
        <!--
          Plain text, in the mono face the value type already uses. Syntax
          highlighting and structural editing are prompts 25 and 26; typing
          into a textarea is the whole of source editing here, and it is
          already the fastest path from an idea to a sound.

          The marks live in a copy behind it, set in the same metrics: a
          textarea cannot hold a mark, and painting one over the text is the
          only way to keep the field a real, editable field.
        -->
        <div class="field">
          <pre class="source ghost" aria-hidden="true" bind:this={ghost}>{#each pieces as piece, index (index)}{#if piece.marked}<mark
                >{piece.text}</mark
              >{:else}{piece.text}{/if}{/each}</pre>
          <textarea
            class="source"
            spellcheck="false"
            aria-label="Source"
            bind:this={area}
            onscroll={sync}
            value={source}
            oninput={(event) => onedit?.(event.currentTarget.value)}
          ></textarea>
        </div>
      {:else}
        <pre class="source" bind:this={page}>{#each pieces as piece, index (index)}{#if piece.marked}<mark
              >{piece.text}</mark
            >{:else}{piece.text}{/if}{/each}</pre>
      {/if}
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
    max-height: 40vh;
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
    display: flex;
    gap: var(--s-6);
    padding: 0 var(--s-6) var(--s-4);
    overflow: auto;
  }

  .field {
    position: relative;
    flex: 1;
    display: flex;
    min-width: 0;
  }

  .source {
    flex: 1;
    margin: 0;
    padding: 0;
    background: none;
    border: 0;
    resize: none;
    font-family: var(--f-mono);
    font-size: var(--t-value-size);
    line-height: var(--t-value-line);
    color: var(--ink-muted);
    white-space: pre;
    overflow: auto;
  }

  /*
   * The copy is invisible except for its marks, and takes no events: it exists
   * so that a highlight can sit under real, editable text.
   */
  .ghost {
    position: absolute;
    inset: 0;
    overflow: hidden;
    color: transparent;
    pointer-events: none;
  }

  /* Provenance, so --plate; a wash rather than a fill, so the text still reads. */
  mark {
    background: var(--plate-wash);
    color: inherit;
    box-shadow: -1px 0 0 var(--plate);
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
