<script lang="ts">
  /**
   * The left margin: the parts list, right-aligned toward the leaf and
   * connected by a real staff bracket (`01-visual-language.md` §7). It is a
   * miniature of the score's own left margin, which is why it needs no header
   * and no box.
   */
  import PartBracket from "../lib/ui/PartBracket.svelte";
  import type { PartFacts } from "../lib/state/snapshot";
  import type { Workspace } from "../lib/state/selection.svelte";

  let { parts, workspace }: { parts: PartFacts[]; workspace: Workspace } = $props();

  const active = $derived(workspace.active);
  let heights = $state<number[]>([]);
</script>

<div class="parts">
  {#each parts as part, index (part.name)}
    <div class="part">
      <div class="names" bind:clientHeight={heights[index]}>
        <div class="name">{part.name}</div>
        {#each part.voices as voice (voice.name)}
          <button
            type="button"
            class="voice"
            class:generated={voice.generated}
            class:active={active?.part === part.name && active?.voice === voice.name}
            onclick={() => workspace.selectVoice(part.name, voice.name)}
          >
            {voice.name}
          </button>
        {/each}
      </div>
      <PartBracket height={heights[index] ?? 0} />
    </div>
  {/each}
</div>

<style>
  .parts {
    display: flex;
    flex-direction: column;
    gap: var(--s-6);
    align-items: flex-end;
  }

  /* The bracket sits against the names, as it does against the staves. */
  .part {
    display: flex;
    align-items: stretch;
    gap: var(--s-1);
  }

  .names {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: var(--s-1);
    text-align: right;
  }

  .name {
    font-family: var(--f-score-text);
    font-size: var(--t-name-size);
    line-height: var(--t-name-line);
    color: var(--ink);
  }

  .voice {
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
    background: none;
    border: 0;
    border-radius: var(--radius-control);
    padding: 0 var(--s-1);
    cursor: pointer;
  }

  .voice:hover {
    color: var(--ink);
  }

  .voice.active {
    color: var(--plate);
  }

  /* A voice the language produced rather than one that was typed. */
  .generated::after {
    content: "";
    display: inline-block;
    width: var(--s-1);
    height: var(--s-1);
    margin-left: var(--s-1);
    border-radius: 50%;
    background: var(--plate);
    vertical-align: middle;
  }
</style>
