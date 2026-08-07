<script lang="ts">
  /**
   * The command palette (`03-interaction.md` §6).
   *
   * It is the discovery surface, and it is what lets the chrome stay free of
   * icon toolbars: everything the application can do is one `⌘K` and a few
   * letters away, named exactly as the result it reports.
   *
   * The list is the keyboard map itself, so a command that exists is a
   * command that appears here, with the binding it actually has.
   */
  import { COMMANDS, spell, type Command } from "../lib/commands/map";
  import Leaf from "../lib/ui/Leaf.svelte";

  let {
    onrun,
    onclose,
  }: {
    onrun: (command: Command) => void;
    onclose: () => void;
  } = $props();

  let query = $state("");
  let at = $state(0);
  let field = $state<HTMLInputElement | undefined>();

  /**
   * Subsequence matching on the title, which is how a composer types at a
   * palette: `exw` finds **Ex**port **W**AV. Ordering is the map's own, so
   * the list does not reshuffle under the fingers as letters arrive.
   */
  function hit(command: Command, typed: string): boolean {
    if (typed === "") return true;
    const title = command.title.toLowerCase();
    let index = 0;
    for (const letter of typed.toLowerCase()) {
      index = title.indexOf(letter, index) + 1;
      if (index === 0) return false;
    }
    return true;
  }

  const shown = $derived(COMMANDS.filter((command) => hit(command, query)));
  const chosen = $derived(shown[Math.min(at, shown.length - 1)]);

  function onkeydown(event: KeyboardEvent): void {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      at = Math.min(Math.max(at + (event.key === "ArrowDown" ? 1 : -1), 0), shown.length - 1);
      return;
    }
    if (event.key === "Enter" && chosen) {
      event.preventDefault();
      onrun(chosen);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
  }

  $effect(() => field?.focus());
</script>

<!--
  The scrim is a click target, not a control: closing by clicking away is
  standard, and the same action has a key (`Esc`) and a place in the map, so
  no operation is pointer-only (§5).
-->
<div
  class="scrim"
  role="presentation"
  onclick={(event) => event.target === event.currentTarget && onclose()}
>
  <div class="palette" role="dialog" aria-modal="true" aria-label="Commands">
    <Leaf>
      <div class="body">
        <input
          bind:this={field}
          bind:value={query}
          class="query"
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="palette-list"
          aria-activedescendant={chosen ? `palette-${chosen.id}` : undefined}
          aria-label="Run a command"
          placeholder="Run a command"
          spellcheck="false"
          oninput={() => (at = 0)}
          {onkeydown}
        />
        <ul class="list" id="palette-list" role="listbox" aria-label="Commands">
          {#each shown as command (command.id)}
            <!--
              The option *is* the row. A button inside it would be a control
              inside a control, which is both an accessibility violation and a
              second thing for the keyboard to land on.
            -->
            <!--
              The keyboard path is the combobox's: arrows move `aria-activedescendant`
              and Enter runs the chosen command, which is the ARIA pattern for a
              listbox. A key handler on each option would be a second, wrong one.
            -->
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <li
              id="palette-{command.id}"
              class="entry"
              role="option"
              aria-selected={command.id === chosen?.id}
              class:chosen={command.id === chosen?.id}
              onclick={() => onrun(command)}
              onmouseenter={() => (at = shown.indexOf(command))}
            >
              <span class="what">{command.title}</span>
              <span class="group">{command.group}</span>
              <span class="binding">{spell(command.accelerator)}</span>
            </li>
          {/each}
          {#if shown.length === 0}
            <li class="none">No command by that name.</li>
          {/if}
        </ul>
      </div>
    </Leaf>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: var(--surround);
    /* The surround, not a black wash: the palette is a small sheet on the
       same desk, not a modal over a document. */
    opacity: 0.98;
    z-index: 10;
  }

  .palette {
    width: min(560px, 90vw);
  }

  .body {
    display: flex;
    flex-direction: column;
    max-height: 60vh;
  }

  .query {
    border: 0;
    border-bottom: 1px solid var(--leaf-edge);
    background: none;
    padding: var(--s-4);
    font-family: var(--f-ui);
    font-size: var(--t-body-size);
    line-height: var(--t-body-line);
    color: var(--ink);
  }

  .query:focus {
    outline: none;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: var(--s-2) 0;
    overflow-y: auto;
  }

  .entry {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: var(--s-4);
    align-items: baseline;
    padding: var(--s-2) var(--s-4);
    font-family: var(--f-ui);
    font-size: var(--t-body-size);
    line-height: var(--t-body-line);
    color: var(--ink);
    cursor: pointer;
  }

  .entry.chosen,
  .entry:hover {
    background: color-mix(in oklab, var(--plate) 10%, transparent);
  }

  .group {
    color: var(--ink-muted);
    font-size: var(--t-small-size);
  }

  .binding {
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    color: var(--ink-muted);
    min-width: 4ch;
    text-align: right;
  }

  .none {
    padding: var(--s-2) var(--s-4);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    color: var(--ink-muted);
  }
</style>
