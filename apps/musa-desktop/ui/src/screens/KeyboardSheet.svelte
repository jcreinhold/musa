<script lang="ts">
  /**
   * The keyboard sheet (`03-interaction.md` §3), rendered from the map.
   *
   * Not a written document: a projection of the same list the key handler
   * reads, so a binding that changed is a sheet that changed. A keyboard
   * instrument whose manual is out of date is an instrument nobody learns.
   */
  import { COMMANDS, spell, type Group } from "../lib/commands/map";
  import Leaf from "../lib/ui/Leaf.svelte";

  let { onclose }: { onclose: () => void } = $props();

  const GROUPS: Group[] = ["Score", "Transport", "View", "File", "Edit", "Settings", "Help"];

  const bound = $derived(COMMANDS.filter((command) => command.accelerator !== null));
</script>

<div class="scrim" role="presentation" onclick={(event) => event.target === event.currentTarget && onclose()}>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Keyboard">
    <Leaf>
      <div class="body">
        <h2 class="title">Keyboard</h2>
        <div class="groups">
          {#each GROUPS as group (group)}
            {@const commands = bound.filter((command) => command.group === group)}
            {#if commands.length > 0}
              <section>
                <h3 class="group">{group}</h3>
                <dl>
                  {#each commands as command (command.id)}
                    <div class="row">
                      <dt>{command.title}</dt>
                      <dd>{spell(command.accelerator)}</dd>
                    </div>
                  {/each}
                </dl>
              </section>
            {/if}
          {/each}
        </div>
        <button type="button" class="close" onclick={onclose}>Close</button>
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
    align-items: center;
    background: var(--surround);
    opacity: 0.98;
    z-index: 10;
  }

  .sheet {
    width: min(720px, 92vw);
    max-height: 84vh;
    overflow: auto;
  }

  .body {
    padding: var(--s-6);
  }

  .title {
    margin: 0 0 var(--s-4);
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    font-weight: 400;
    color: var(--ink);
  }

  /* Two columns of a printed reference card, not a settings screen. */
  .groups {
    columns: 2;
    column-gap: var(--s-6);
  }

  section {
    break-inside: avoid;
    margin-bottom: var(--s-5);
  }

  .group {
    margin: 0 0 var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-muted);
    font-weight: 400;
  }

  dl {
    margin: 0;
  }

  .row {
    display: flex;
    justify-content: space-between;
    gap: var(--s-4);
    padding: 0.1em 0;
  }

  dt {
    font-family: var(--f-ui);
    font-size: var(--t-body-size);
    line-height: var(--t-body-line);
    color: var(--ink);
  }

  dd {
    margin: 0;
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    color: var(--ink-muted);
    white-space: nowrap;
  }

  .close {
    margin-top: var(--s-4);
    background: none;
    border: 0;
    padding: var(--s-1) var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    color: var(--ink-muted);
    cursor: pointer;
  }

  .close:hover {
    color: var(--ink);
  }
</style>
