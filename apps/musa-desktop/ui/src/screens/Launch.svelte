<script lang="ts">
  /**
   * No piece open (`05-states.md` §2): the leaf is present but blank, the
   * name is set on the sheet in the score's own text face, and the three
   * things you can do are plain text links on the surround beneath it.
   *
   * No splash art, no hero, no spinner. An empty application should look like
   * an empty page, because that is what it is.
   */
  import Leaf from "../lib/ui/Leaf.svelte";

  let { onopen, onnew }: { onopen: () => void; onnew: () => void } = $props();
</script>

<div class="launch">
  <div class="stage">
    <Leaf>
      <div class="blank">
        <p class="name">musa</p>
      </div>
    </Leaf>
  </div>
  <nav class="actions" aria-label="Start">
    <button type="button" onclick={onopen}>Open a piece</button>
    <button type="button" onclick={onnew}>New piece</button>
  </nav>
</div>

<style>
  .launch {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    justify-items: center;
    height: 100%;
    padding: var(--s-6) 0;
    gap: var(--s-5);
  }

  .stage {
    display: flex;
    justify-content: center;
    min-height: 0;
  }

  .stage :global(> *) {
    height: 100%;
    aspect-ratio: 210 / 297;
    max-width: 100%;
  }

  .blank {
    display: flex;
    justify-content: center;
    height: 100%;
    padding-top: 12%;
  }

  .name {
    margin: 0;
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    color: var(--ink-muted);
  }

  .actions {
    display: flex;
    gap: var(--s-5);
  }

  /* Text links on the surround, not buttons: nothing here is a chrome
     control, and a row of filled buttons would read as a dialog. */
  .actions button {
    background: none;
    border: 0;
    padding: var(--s-1) 0;
    font-family: var(--f-ui);
    font-size: var(--t-body-size);
    line-height: var(--t-body-line);
    color: var(--ink-muted);
    text-decoration: underline;
    text-underline-offset: 3px;
    text-decoration-color: var(--rule);
    cursor: pointer;
  }

  .actions button:hover {
    color: var(--ink);
    text-decoration-color: var(--plate);
  }
</style>
