<script lang="ts">
  /**
   * A fixture shown as engraving alone — the surface the raster goldens of
   * `02-engraving.md` §9 are taken from. No chrome, so a diff is a diff in the
   * engraving and nothing else.
   */
  import Score from "../lib/score/Score.svelte";
  import Leaf from "../lib/ui/Leaf.svelte";
  import type { Fixture } from "../lib/state/fixtures";

  let { fixture }: { fixture: Fixture } = $props();
</script>

<div class="sheet">
  <Leaf>
    <Score mei={fixture.mei} revision={1} zoom={100} />
  </Leaf>
</div>

<style>
  .sheet {
    height: 100%;
    padding: var(--s-6);
  }

  .sheet :global(> *) {
    height: 100%;
  }
</style>
