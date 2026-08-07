<script lang="ts">
  /**
   * Which workspace is open (roadmap §14.4).
   *
   * Four, in the order they are numbered, because the order is an argument:
   * what the music is, what it sounds like, how it is balanced, what it says
   * in text. The current workspace is the one set in ink; the others are an
   * offer.
   *
   * The accelerator is written beside the name because these are the bindings
   * a composer uses most and reading them here is how they are learned
   * (`03-interaction.md` §3).
   */
  import type { Screen } from "../commands/map";

  let {
    current,
    onshow,
  }: {
    current: Screen;
    onshow: (which: Screen) => void;
  } = $props();

  const OPEN = [
    { id: "compose", name: "Compose", key: "⌘1" },
    { id: "sound", name: "Sound", key: "⌘2" },
    { id: "mix", name: "Mix", key: "⌘3" },
    { id: "source", name: "Source", key: "⌘4" },
  ] as const;
</script>

<nav class="workspaces" aria-label="Workspace">
  {#each OPEN as workspace (workspace.id)}
    <button
      type="button"
      class="text"
      aria-current={current === workspace.id ? "page" : undefined}
      onclick={() => onshow(workspace.id)}
    >
      {workspace.name}<span class="key" aria-hidden="true">{workspace.key}</span>
    </button>
  {/each}
</nav>

<style>
  /*
   * Four names and their bindings are one object, and they break as one: the
   * switcher wraps to its own row rather than splitting "Mix" from "Source",
   * because a list you have to read across two rows is no longer a list of
   * four places.
   */
  .workspaces {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--s-1);
  }

  .text {
    white-space: nowrap;
    background: none;
    border: 0;
    border-radius: var(--radius-control);
    padding: var(--s-1) var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
    cursor: pointer;
  }

  .text:hover {
    color: var(--ink);
  }

  /* Current is ink, and underlined: never colour alone (`03-interaction.md` §5). */
  .text[aria-current="page"] {
    color: var(--ink);
    text-decoration: underline;
    text-underline-offset: 0.35em;
  }

  .key {
    padding-left: var(--s-2);
    font-family: var(--f-mono);
    color: var(--ink-muted);
  }
</style>
