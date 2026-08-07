<script lang="ts">
  /**
   * The bottom drawer: source and diagnostics. It slides up from the surround
   * and **pushes** the leaf; it never overlaps it (`01-visual-language.md`
   * §7). Closed by default in Compose.
   *
   * With no diagnostics the pane shows nothing at all — not "0 problems".
   * Absence is the message (`05-states.md` §2).
   */
  import type { Diagnostic } from "../lib/state/snapshot";

  let {
    source,
    diagnostics,
    editable = false,
    onedit,
    open = $bindable(false),
  }: {
    source: string;
    diagnostics: Diagnostic[];
    editable?: boolean;
    onedit?: (source: string) => void;
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
      {#if editable}
        <!--
          Plain text, in the mono face the value type already uses. Syntax
          highlighting and structural editing are prompts 25 and 26; typing
          into a textarea is the whole of source editing here, and it is
          already the fastest path from an idea to a sound.
        -->
        <textarea
          class="source"
          spellcheck="false"
          aria-label="Source"
          value={source}
          oninput={(event) => onedit?.(event.currentTarget.value)}
        ></textarea>
      {:else}
        <pre class="source">{source}</pre>
      {/if}
      {#if diagnostics.length > 0}
        <ul class="diagnostics">
          {#each diagnostics as diagnostic, index (index)}
            <li>
              <span class="glyph {diagnostic.severity}" aria-hidden="true"></span>
              <span class="message">{diagnostic.message}</span>
              {#if diagnostic.span}<span class="where">{diagnostic.span.start}</span>{/if}
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

  .diagnostics {
    flex: 1;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--t-body-size);
    line-height: var(--t-body-line);
  }

  .where {
    font-family: var(--f-mono);
    color: var(--ink-muted);
    margin-left: var(--s-2);
  }
</style>
