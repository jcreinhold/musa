<script lang="ts">
  /**
   * A margin: everything that is not the leaf. Matte, unlit, no fill of its
   * own, no border, no box (`01-visual-language.md` §7). The leaf's shadow is
   * the only separation between a margin and the page.
   *
   * `side` sets the margin's own alignment, which is the one thing that
   * differs between them: the left margin is a miniature of the score's left
   * margin and therefore reads toward the leaf.
   *
   * Each side is also a landmark, so a screen reader can move between the
   * regions of the workspace the way a pointer moves between them: the top
   * margin is the header, the bottom the footer, the side margins are asides
   * named by what they hold (`03-interaction.md` §5).
   */
  import type { Snippet } from "svelte";

  let {
    side,
    label,
    children,
  }: {
    side: "top" | "left" | "right" | "bottom";
    label?: string;
    children: Snippet;
  } = $props();

  const LANDMARKS = {
    top: "header",
    left: "aside",
    right: "aside",
    bottom: "footer",
  } as const;
</script>

<svelte:element this={LANDMARKS[side]} class="margin {side}" aria-label={label}>
  {@render children()}
</svelte:element>

<style>
  .margin {
    background: none;
    border: 0;
    min-width: 0;
  }

  /*
   * The top margin is a band of 48px rows, not a 48px bar.
   *
   * A fixed height here is the one thing that can make chrome hide itself:
   * what does not fit is not clipped, it is drawn on top of whatever is
   * already there, and the composer gets a piece title printed through a key
   * signature. So the height is a floor and the band wraps — one row while one
   * row holds it, another row when it does not (`01-visual-language.md` §7).
   * This is also what makes 200 % browser zoom reflow instead of collide.
   */
  .top {
    display: flex;
    align-items: center;
    align-content: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--s-2) var(--s-6);
    min-height: var(--s-12);
    padding: 0 var(--s-6);
  }

  .left {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    padding: var(--s-6) var(--s-3) var(--s-6) var(--s-6);
    overflow-y: auto;
  }

  .right {
    display: flex;
    flex-direction: column;
    gap: var(--s-4);
    padding: var(--s-6);
    overflow-y: auto;
  }

  .bottom {
    background: var(--surround-in);
    border-top: 1px solid var(--rule);
    overflow: hidden;
  }
</style>
