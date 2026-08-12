<script lang="ts">
  /**
   * No piece open (`05-states.md` §2).
   *
   * The state is a title page: the leaf is present and unwritten, the name is
   * set on the sheet in the score's own text face, and the page's text block
   * is ruled with empty staves at the resting rastral size. A blank sheet and
   * a ruled sheet are the same amount of nothing, but only one of them is
   * recognisably music paper — and "empty is an invitation" is a rule of this
   * application's voice, not a slogan.
   *
   * The ways in sit beneath the leaf on the surround, as a short ruled list
   * with the menu's own accelerators set beside them. Nothing is placed on the
   * sheet except the name, because on every other screen the sheet is the
   * score and chrome is margin, and a launch screen that broke that would be
   * teaching the wrong thing on the first frame.
   *
   * No splash art, no hero, no spinner.
   */
  import Leaf from "../lib/ui/Leaf.svelte";

  let { onopen, onopenproject, onnew }: { onopen: () => void; onopenproject: () => void; onnew: () => void } = $props();

  /**
   * How many staves the page is ruled with.
   *
   * They are spread across the text block rather than stacked at a fixed
   * pitch, the way a rastrum is spaced to fit the page it is ruling: the top
   * and bottom staves land on the page margins at every window height, and
   * nothing is ever half-cut by the paper's edge.
   */
  const STAVES = 6;

  /** The ways in, in the order a returning composer wants them. */
  const START = [
    { title: "Open a piece", key: "⌘O", run: () => onopen() },
    // A project is a folder of pieces (roadmap §16), and it is a different
    // question to ask the file dialog — so it is a different way in rather
    // than a dialog that guesses from what was chosen.
    { title: "Open a project", key: "⇧⌘O", run: () => onopenproject() },
    { title: "New piece", key: "⌘N", run: () => onnew() },
  ];
</script>

<div class="launch">
  <div class="sheet">
    <Leaf>
      <div class="page">
        <p class="name">musa</p>
        <div class="rastrum" aria-hidden="true">
          {#each { length: STAVES } as _, stave (stave)}
            <div class="stave"></div>
          {/each}
        </div>
      </div>
    </Leaf>
  </div>

  <nav class="start" aria-label="Start">
    {#each START as action, index (action.title)}
      <!-- svelte-ignore a11y_autofocus -->
      <button type="button" class="action" autofocus={index === 0} onclick={action.run}>
        <span>{action.title}</span>
        <span class="key" aria-hidden="true">{action.key}</span>
      </button>
    {/each}
  </nav>
</div>

<style>
  .launch {
    /*
     * One measure for the whole screen: the sheet's width, and therefore the
     * width of the list beneath it. Capped so the page never stretches into
     * the tall narrow band a full-height A4 becomes on a wide screen, and
     * bounded by the window's own height so a short window shrinks the sheet
     * rather than pushing the ways in off the bottom. 190px is what the rest
     * of the screen costs: the padding, the gap, and the list.
     */
    --measure: min(452px, 100%, calc((100vh - 190px) * 210 / 297));
    /*
     * The page's side margin, as a fraction of the measure rather than a
     * percentage: a percentage padding is read against the containing block,
     * and the list's containing block is the window rather than the sheet.
     * One number, used twice, is what puts the list's rules on the same axis
     * as the staves above them.
     */
    --page-margin: calc(var(--measure) * 0.11);
    --page-head: calc(var(--measure) * 0.09);

    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: var(--s-8) var(--s-6);
    gap: var(--s-8);
  }

  .sheet {
    width: var(--measure);
    aspect-ratio: 210 / 297;
  }

  .sheet :global(> *) {
    height: 100%;
  }

  /* Real page margins: the title sits where a title sits. */
  .page {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: var(--page-head) var(--page-margin) var(--page-margin);
  }

  .name {
    margin: 0;
    text-align: center;
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    color: var(--ink);
  }

  .rastrum {
    display: flex;
    flex: 1;
    flex-direction: column;
    justify-content: space-between;
    min-height: 0;
    margin-top: var(--page-margin);
    overflow: hidden;
  }

  /*
   * Five lines at the rastral unit the engraver lays pages out with, drawn in
   * the hairline the staff bracket uses — the same weight the real staff lines
   * on this sheet would have (`01-visual-language.md` §4).
   */
  .stave {
    flex: 0 0 auto;
    height: calc(var(--sp) * 4 + 1px);
    background: repeating-linear-gradient(to bottom, var(--rule) 0 1px, transparent 1px var(--sp));
  }

  /*
   * A ruled list, on the page's own measure and the page's own hairline: each
   * way in is a line of the same width and weight as the staves above it, with
   * its accelerator where a folio would sit. Two words on one line, twenty
   * pixels apart, read as one cramped clump rather than as two choices.
   */
  .start {
    display: flex;
    flex-direction: column;
    width: var(--measure);
    /* Plus the sheet's own hairline edge, which the page's margin sits inside
       of and this list does not: without it the two rules miss by a pixel,
       which on a shared axis is the only pixel anyone would notice. */
    padding: 0 calc(var(--page-margin) + 1px);
  }

  .action {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-4);
    background: none;
    border: 0;
    border-bottom: 1px solid var(--rule);
    padding: var(--s-2) 0;
    font-family: var(--f-ui);
    font-size: var(--t-body-size);
    line-height: var(--t-body-line);
    color: var(--ink);
    text-align: left;
    cursor: pointer;
  }

  /* Hover changes the rule and nothing else, instantly: motion exists here for
     three reasons and a hover fade is not one of them (§6). */
  .action:hover {
    border-bottom-color: var(--ink);
  }

  /* The accelerator, set as the workspace switcher sets it: the binding is
     learned by reading it next to the thing it does. */
  .key {
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    color: var(--ink-muted);
  }
</style>
