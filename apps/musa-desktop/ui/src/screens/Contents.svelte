<script lang="ts">
  /**
   * The volume's front matter (`docs/interface/07-the-volume.md`).
   *
   * A project is not a file tree. Roadmap §16 fixes its shape — a manifest,
   * `pieces/`, `library/` — so disclosure triangles would model a freedom the
   * format does not have while burying the one thing that matters, which is
   * the order the pieces go in. A bound volume already has the two devices
   * this needs: a contents page with a running order, and an editorial note at
   * the foot listing the material the pieces rest on.
   *
   * One typographic rule carries every row: **what the composer wrote is set
   * in Academico, what the filesystem knows is set in mono**. Title and file
   * name, two faces, one line — which is `01-visual-language.md` §3's existing
   * assignment applied, not a new idiom.
   *
   * Nothing here is composed: `title`, `edited` and `in use` were all decided
   * in Rust (`03-interaction.md` §7). This screen arranges facts.
   */
  import Leaf from "../lib/ui/Leaf.svelte";
  import Workspaces from "../lib/ui/Workspaces.svelte";
  import Margin from "../lib/ui/Margin.svelte";
  import type { Screen } from "../lib/commands/map";
  import type { ContentsFacts, EntryFacts } from "../lib/state/snapshot";

  let {
    contents,
    onchoose,
    onshow,
  }: {
    contents: ContentsFacts;
    onchoose: (file: string) => void;
    onshow: (which: Screen) => void;
  } = $props();

  /**
   * The position numeral, in the volume's own counting.
   *
   * Numbering is earned here rather than decorative: this is an album's
   * running order, and the files in `examples/album/` are literally named
   * `01-` and `02-`. Material is not numbered, because material has no
   * position — it is apparatus.
   */
  const numeral = (index: number) => String(index + 1).padStart(2, "0");
</script>

<div class="workspace">
  <Margin side="top">
    <div class="identity">
      <!--
        The frame names what is open, and here what is open is the volume —
        the same rule Compose follows when it sets the piece's title there.
        The word "Contents" belongs to the switcher, which is the one place
        that says where you can go.
      -->
      <h1 class="here">{contents.name}</h1>
      <Workspaces current="contents" volume={true} {onshow} />
    </div>
  </Margin>

  <main class="stage">
    <Leaf>
      <div class="page">
        <header class="head">
          <p class="volume">{contents.name}</p>
          {#if contents.composer}
            <p class="composer">{contents.composer}</p>
          {/if}
        </header>

        <nav class="order" aria-label="Running order">
          {#each contents.pieces as entry, index (entry.file)}
            {@render row(entry, numeral(index))}
          {/each}
        </nav>

        <!--
          The editorial note at the foot of the text block: what the pieces
          draw on, set as apparatus because that is what it is — a library
          declares and does not sound.
        -->
        {#if contents.material.length > 0}
          <section class="material" aria-label="Material">
            <h2 class="apparatus">Material</h2>
            <nav class="order">
              {#each contents.material as entry (entry.file)}
                {@render row(entry, null)}
              {/each}
            </nav>
          </section>
        {/if}
      </div>
    </Leaf>
  </main>
</div>

{#snippet row(entry: EntryFacts, position: string | null)}
  <button
    type="button"
    class="entry"
    class:apparatus={position === null}
    aria-current={entry.current ? "page" : undefined}
    onclick={() => onchoose(entry.file)}
  >
    <span class="position" aria-hidden="true">{position ?? ""}</span>
    <span class="title">{entry.title}</span>
    <span class="file">{entry.file}</span>
    <!--
      Words, not dots. A mark you have to be taught says nothing the first
      time it is seen; these are in the application's own vocabulary, and
      `unsaved` and `used` were both decided by the core.
    -->
    {#if entry.unsaved}<span class="note">edited</span>{/if}
    {#if entry.used}<span class="note">in use</span>{/if}
  </button>
{/snippet}

<style>
  .workspace {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
    max-width: 100%;
    overflow-x: hidden;
  }

  .identity {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--s-2) var(--s-4);
    min-width: 0;
  }

  .here {
    flex: 0 1 auto;
    margin: 0;
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    font-weight: 400;
    color: var(--ink);
  }

  .stage {
    display: flex;
    justify-content: center;
    min-height: 0;
    overflow-y: auto;
    padding: var(--s-8) var(--s-6);
  }

  .stage :global(> *) {
    width: min(640px, 100%);
    height: max-content;
  }

  /*
   * Real page margins, on the proportion the launch sheet uses: a contents
   * page is a page of the same book, and its text block starts where a title
   * would.
   */
  .page {
    display: flex;
    flex-direction: column;
    padding: calc(var(--s-16) + var(--s-4)) var(--s-16) var(--s-16);
  }

  .head {
    margin-bottom: var(--s-12);
  }

  .volume {
    margin: 0;
    font-family: var(--f-score-text);
    font-size: var(--t-large-size);
    line-height: var(--t-large-line);
    color: var(--ink);
  }

  .composer {
    margin: var(--s-2) 0 0;
    font-family: var(--f-score-text);
    font-size: var(--t-name-size);
    line-height: var(--t-name-line);
    color: var(--ink-muted);
  }

  .order {
    display: flex;
    flex-direction: column;
  }

  /*
   * One ruled line per piece, on the page's own hairline — the rule the ways
   * in on the launch screen are set with, because they are the same kind of
   * list: a short set of places this page can take you.
   */
  .entry {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    align-items: baseline;
    gap: var(--s-4);
    width: 100%;
    background: none;
    border: 0;
    border-bottom: 1px solid var(--rule);
    /* Four pixels short at the foot, because the title below carries them:
       the underline that marks the current row needs room inside a box that
       clips, and the rule must not move to give it that room. */
    padding: var(--s-3) 0 calc(var(--s-3) - var(--s-1));
    text-align: left;
    cursor: pointer;
  }

  .entry:hover {
    border-bottom-color: var(--ink);
  }

  /* The composer's own counting, in the score's own figures. */
  .position {
    font-family: var(--f-score-text);
    font-variant-numeric: tabular-nums;
    font-size: var(--t-name-size);
    color: var(--ink-faint);
  }

  /* What the composer called it. */
  .title {
    min-width: 0;
    /* A long title is cut with an ellipsis, which needs the box to clip — and
       a box that clips also clips the underline drawn beneath the baseline.
       The padding is the room that underline is drawn in; the row's own
       bottom padding is short by the same amount, so the rule does not move.
       Baseline alignment means it does not move the text either. */
    padding-bottom: var(--s-1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--f-score-text);
    font-size: var(--t-name-size);
    line-height: var(--t-name-line);
    color: var(--ink);
  }

  /* What the filesystem calls it. A different kind of knowledge, so a
     different face — this is the whole information design of the row. */
  .file {
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-faint);
  }

  .note {
    font-family: var(--f-ui);
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    letter-spacing: var(--tracking-micro);
    text-transform: lowercase;
    color: var(--ink-muted);
    white-space: nowrap;
  }

  /* Current is ink and underlined, never colour alone (`03-interaction.md` §5). */
  .entry[aria-current="page"] .title {
    text-decoration: underline;
    text-underline-offset: 0.3em;
  }

  /*
   * The apparatus: one type step down, after the text block, where an
   * editorial note sits. It is the one deliberate risk on this page, and it
   * is a true one — material is not a smaller kind of piece, it is a
   * different kind of thing.
   */
  .material {
    /* Space, and no second rule: the last piece's own hairline is the one the
       apparatus sits under. Two rules with nothing between them read as an
       empty entry in the running order. */
    margin-top: var(--s-12);
  }

  .apparatus {
    margin: 0 0 var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    letter-spacing: var(--tracking-micro);
    text-transform: uppercase;
    font-weight: 400;
    color: var(--ink-faint);
  }

  .entry.apparatus {
    padding: var(--s-2) 0 calc(var(--s-2) - var(--s-1));
    text-transform: none;
    letter-spacing: normal;
  }

  .entry.apparatus .title {
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  /* Its own file name is already its title: printing it twice says nothing. */
  .entry.apparatus .file {
    display: none;
  }
</style>
