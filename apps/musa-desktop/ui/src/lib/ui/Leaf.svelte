<script lang="ts">
  /**
   * The sheet. The only lit surface in the application and the only elevated
   * one (`00-thesis.md`, `01-visual-language.md` §5): leaf fill, a `--leaf-edge`
   * hairline, one shadow, zero radius. Nothing else in the app may use this
   * treatment except the popovers that are, conceptually, small sheets.
   */
  import type { Snippet } from "svelte";

  /**
   * `stale` turns the hairline `--chalk` while the source has problems and
   * the engraving is older than the text (`05-states.md` §4). The score
   * itself is untouched — not dimmed, not blurred, not overlaid: it is still
   * correct, it is only older.
   */
  let { children, stale = false }: { children: Snippet; stale?: boolean } =
    $props();
</script>

<div class="leaf" class:stale>{@render children()}</div>

<style>
  .leaf {
    position: relative;
    background: var(--leaf);
    border: 1px solid var(--leaf-edge);
    border-radius: var(--radius-leaf);
    box-shadow: var(--leaf-shadow);
  }

  .leaf.stale {
    border-color: var(--chalk);
  }
</style>
