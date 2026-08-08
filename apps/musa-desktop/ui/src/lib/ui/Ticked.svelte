<script lang="ts">
  /**
   * A line of the compiler's prose, with the code in it set as code.
   *
   * Diagnostics are written once and read in two places. In the terminal
   * ``missing `;` `` is right, because backticks are what a terminal has for
   * "this is a character in your file". On a page they are leaked markup: the
   * app has a mono face and should use it. So the words cross the wire in the
   * compiler's own spelling and the difference is a rendering decision here,
   * not a second set of strings in the core.
   *
   * Pairs only. An odd backtick is a typo in a message, and printing it as
   * written is how it gets noticed.
   */
  let { text }: { text: string } = $props();

  const parts = $derived(text.split("`"));
  const paired = $derived(parts.length % 2 === 1);
</script>

{#if paired}
  {#each parts as part, index (index)}
    {#if index % 2 === 1}<code>{part}</code>{:else}{part}{/if}
  {/each}
{:else}
  {text}
{/if}

<style>
  code {
    font-family: var(--f-mono);
    /* The mono face runs large beside the UI face at the same nominal size. */
    font-size: 0.92em;
  }
</style>
