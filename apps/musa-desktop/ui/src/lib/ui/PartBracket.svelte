<script lang="ts">
  /**
   * The parts list's bracket — drawn with the score's own glyphs.
   *
   * `01-visual-language.md` §7 asks for "a real staff bracket", and this is
   * literally it: SMuFL `bracketTop` (U+E003) and `bracketBottom` (U+E004)
   * from Bravura, joined by the bracket's vertical spine, in `--rule`. The
   * left margin is a miniature of the score's own left margin, which is why
   * the list needs no header and no box.
   */

  /** SMuFL bracket horns. Their design height is 0.295 em above the baseline. */
  const BRACKET_TOP = "\uE003";
  const BRACKET_BOTTOM = "\uE004";
  const HORN_RATIO = 0.295;
  /** Em size of the horns; also fixes the spine's proportions. */
  const SIZE = 20;
  /** The bracket's spine, in the same proportion Bravura draws it. */
  const SPINE = 2;
  const WIDTH = 10;

  let { height }: { height: number } = $props();

  const horn = SIZE * HORN_RATIO;
  const spine = $derived(Math.max(height - horn * 2, 0));
</script>

{#if height > horn * 2}
  <svg class="bracket" width={WIDTH} {height} viewBox="0 0 {WIDTH} {height}" aria-hidden="true">
    <text x="0" y={horn} font-size={SIZE}>{BRACKET_TOP}</text>
    <rect x="0" y={horn} width={SPINE} height={spine} />
    <text x="0" y={height - horn} font-size={SIZE}>{BRACKET_BOTTOM}</text>
  </svg>
{/if}

<style>
  .bracket {
    color: var(--rule);
    fill: currentColor;
    overflow: visible;
    flex: none;
  }

  text {
    font-family: var(--f-notation);
  }
</style>
