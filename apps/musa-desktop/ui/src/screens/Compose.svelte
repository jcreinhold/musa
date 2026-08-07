<script lang="ts">
  /**
   * The Compose workspace: leaf and margin, exactly the arrangement of
   * `01-visual-language.md` §7.
   *
   * Everything musical is read from the snapshot and everything interactive
   * is owned above: the workspace, the playhead, and the keyboard all belong
   * to the application, so this screen only arranges them.
   */
  import { untrack } from "svelte";

  import Score from "../lib/score/Score.svelte";
  import type { ViewMode } from "../lib/engrave/options";
  import Leaf from "../lib/ui/Leaf.svelte";
  import Margin from "../lib/ui/Margin.svelte";
  import GlyphButton from "../lib/ui/GlyphButton.svelte";
  import TransportReadout from "../lib/ui/TransportReadout.svelte";
  import Workspaces from "../lib/ui/Workspaces.svelte";
  import { REPEAT_RIGHT_LEFT } from "../lib/ui/glyphs";
  import type { Screen } from "../lib/commands/map";
  import type { Session } from "../lib/session/session.svelte";
  import type { Workspace } from "../lib/state/selection.svelte";
  import type { Reveal } from "../lib/state/reveal";
  import type { Diagnostic, EditImpact, Span } from "../lib/state/snapshot";
  import type { NoteEntry } from "../lib/state/entry.svelte";
  import Drawer from "./Drawer.svelte";
  import Inspector from "./Inspector.svelte";
  import PartsList from "./PartsList.svelte";

  let {
    session,
    workspace,
    zoom,
    mode,
    playing,
    loop,
    follow,
    origin,
    pinned,
    entry,
    choice,
    naming,
    flash,
    reveal,
    onzoom,
    onpinch,
    onmode,
    onloop,
    onfollow,
    onpin,
    onentry,
    onconfirm,
    onspecialize,
    oncancel,
    onname,
    oncancelname,
    onpitch,
    onduration,
    onreveal,
    ondiagnostic,
    oncaret,
    onshow,
  }: {
    session: Session;
    workspace: Workspace;
    zoom: number;
    mode: ViewMode;
    /** The event ids sounding right now. */
    playing: string[];
    /** The looped range, as its first and last event ids. */
    loop: [string, string] | null;
    follow: "off" | "page" | "continuous";
    /** Origin view, held or pinned (`04-provenance.md` §2). */
    origin: boolean;
    /** Whether it is pinned, which is what the toggle reports. */
    pinned: boolean;
    /** Note entry: what the next note would be, and whether letters are notes. */
    entry: NoteEntry;
    /** An edit against generated music waiting to be confirmed (§4). */
    choice: EditImpact | null;
    /** How many notes an extraction is waiting on a name for, or null. */
    naming: number | null;
    /** Event ids a diagnostic points at; their systems flash once. */
    flash: string[];
    /** A place to put the caret in the source, once, when it changes. */
    reveal: Reveal | null;
    onzoom: (by: number) => void;
    onpinch: (factor: number) => void;
    onmode: (mode: ViewMode) => void;
    onloop: () => void;
    onfollow: () => void;
    onpin: () => void;
    onentry: () => void;
    onconfirm: () => void;
    /** Take the other answer: change this occurrence only. */
    onspecialize: () => void;
    oncancel: () => void;
    onname: (name: string) => void;
    oncancelname: () => void;
    onpitch: (event: string, pitch: string) => void;
    onduration: (event: string, duration: string) => void;
    onreveal: (span: Span) => void;
    ondiagnostic: (diagnostic: Diagnostic) => void;
    /** The source caret moved; the score follows it (roadmap §14.4). */
    oncaret: (offset: number) => void;
    onshow: (which: Screen) => void;
  } = $props();

  const snapshot = $derived(session.snapshot);
  const score = $derived(snapshot?.score ?? null);
  const focused = $derived(workspace.focused);
  const problems = $derived(snapshot?.diagnostics.filter((d) => d.severity === "error") ?? []);

  /**
   * What the drawer marks: the declaration and the use of the expansion in
   * view. The workspace decides which expansion that is, so the drawer and
   * the Source workspace cannot mark different things.
   */
  const highlight = $derived(workspace.sourceSpans(origin));

  // A new score is a new set of events. The selection is by id and usually
  // survives it untouched; when the note it was on is gone, this is what moves
  // it to a neighbour and tells the inspector to say so (`02-engraving.md` §6).
  $effect(() => {
    void snapshot?.scoreRevision;
    untrack(() => workspace.reconcile());
  });
</script>

{#if snapshot && score}
  <div class="workspace">
    <Margin side="top">
      <div class="identity">
        <h1 class="title">{score.title}</h1>
        <!--
          Whether the file has the work. Autosave means unsaved work is not
          lost work, so the mark says which of the two it is rather than
          nagging (roadmap §15.7).
        -->
        {#if snapshot.unsaved}
          <!--
            Not a live region: this changes on the first keystroke of every
            edit, and announcing it each time would talk over the composer.
            What is announced is the save itself, in the margin's notice.
          -->
          <span class="state"
            >{snapshot.autosaved ? "Unsaved — recovery copy kept" : "Unsaved"}</span
          >
        {/if}
        <Workspaces current="compose" onshow={onshow} />
      </div>

      <!--
        The top margin carries whatever the interface currently has to say:
        a completed operation for three seconds, a failure until it is
        superseded, and — the one that matters — how far behind the page is
        while the source has problems (`05-states.md` §4).
      -->
      <!--
        Work a previous session did not get to save takes the margin before
        anything else does: it is the only line here that is about work that
        could still be lost. It is offered rather than applied, because taking
        it is an edit and declining it is the composer's to decide (§15.7).
      -->
      {#if snapshot.recovery !== null}
        <div class="recovery" role="group" aria-label="Unsaved work from the last session">
          <p>Unsaved work from a session that did not close.</p>
          <button type="button" class="text" onclick={() => void session.recover(true)}
            >Restore it</button
          >
          <button type="button" class="text" onclick={() => void session.recover(false)}
            >Discard it</button
          >
        </div>
      {:else if session.notice}
        <p class="notice" class:failure={session.notice.tone === "failure"} role="status">
          {session.notice.message}
        </p>
      {:else if session.stale && session.shownRevision !== null}
        <p class="notice stale" role="status">
          Showing revision {session.shownRevision} — the current source has {problems.length}
          {problems.length === 1 ? "problem" : "problems"}
        </p>
      {/if}

      <div class="controls">
        <div class="transport">
          <button
            type="button"
            class="text"
            disabled={!session.live}
            aria-pressed={snapshot.playback.playing}
            onclick={() => void session.toggle()}>{snapshot.playback.playing ? "Pause" : "Play"}</button
          >
          <button
            type="button"
            class="text"
            disabled={!session.live || !snapshot.playback.playing}
            onclick={() => void session.stop()}>Stop</button
          >
          <GlyphButton
            glyph={REPEAT_RIGHT_LEFT}
            label="Loop the selection"
            active={loop !== null}
            disabled={!session.live}
            onclick={onloop}
          />
          <button
            type="button"
            class="text"
            aria-pressed={follow !== "off"}
            onclick={onfollow}>Follow</button
          >
        </div>

        <!--
          Note entry, and what it would write. The duration is the glyph an
          engraver draws rather than a word, because the composer reading it
          writes that glyph for a living (`03-interaction.md` §3).
        -->
        <button
          type="button"
          class="text entry"
          aria-pressed={entry.on}
          title="Write notes with the letter keys — N"
          onclick={onentry}
          >Notes{#if entry.on}<span class="duration" aria-hidden="true">{entry.glyph}</span
            ><span class="visually-hidden"> — duration {entry.duration}</span>{/if}</button
        >

        <!--
          Which keyboard the notes would come from. Only while entry is on,
          because that is the only time one is being read, and silent when
          there is none: not owning a MIDI keyboard is not a problem to report
          (roadmap §14.8).
        -->
        {#if entry.on && snapshot.midiPort}
          <span class="port" title="Notes played here are written at the caret"
            >{snapshot.midiPort}</span
          >
        {/if}

        <!--
          The lens is held — `O` or `⌥` — and this pins it, for anyone who
          cannot hold a key while working the pointer (`04-provenance.md` §2).
          It reports the pin, not the lens, because that is the state a click
          changes.
        -->
        <button
          type="button"
          class="text origin"
          aria-pressed={pinned}
          title="Show where the music came from — hold O"
          onclick={onpin}>Origin</button
        >
        <TransportReadout
          {score}
          playback={snapshot.playback}
          stale={session.stale}
          bar={focused?.bar ?? 1}
          beat={focused?.beat ?? { numerator: 1, denominator: 1 }}
        />
        <!--
          Two words, one of them current: page or continuous (§4). A toggle
          named for what it shows, rather than a pair of icons a reader has to
          learn.
        -->
        <div class="view" role="group" aria-label="View">
          <button
            type="button"
            class="text"
            aria-pressed={mode === "page"}
            onclick={() => onmode("page")}>Pages</button
          >
          <button
            type="button"
            class="text"
            aria-pressed={mode === "continuous"}
            onclick={() => onmode("continuous")}>Continuous</button
          >
        </div>

        <div class="zoom">
          <button type="button" class="text" onclick={() => onzoom(-1)} aria-label="Zoom out"
            >−</button
          >
          <span class="level">{zoom}&thinsp;%</span>
          <button type="button" class="text" onclick={() => onzoom(1)} aria-label="Zoom in"
            >+</button
          >
        </div>
      </div>
    </Margin>

    <div class="body">
      <Margin side="left" label="Parts">
        <PartsList parts={score.parts} {workspace} {origin} />
      </Margin>

      <main class="stage" class:continuous={mode === "continuous"}>
        <Leaf stale={session.stale}>
          <Score
            mei={snapshot.mei ?? ""}
            revision={snapshot.scoreRevision ?? snapshot.revision}
            {zoom}
            {mode}
            {workspace}
            {onpinch}
            {playing}
            {loop}
            {follow}
            {origin}
            {flash}
          />
        </Leaf>
      </main>

      <Margin side="right" label="Inspector">
        <Inspector
          event={focused}
          adrift={workspace.adrift}
          occurrence={workspace.selectedOccurrence}
          {choice}
          {naming}
          onorigin={(depth) => workspace.selectOrigin(depth)}
          {onconfirm}
          {onspecialize}
          {oncancel}
          {onname}
          {oncancelname}
          onpitch={session.live && focused ? (pitch) => onpitch(focused.id, pitch) : undefined}
          onduration={session.live && focused
            ? (duration) => onduration(focused.id, duration)
            : undefined}
          {onreveal}
        />
      </Margin>
    </div>

    <Margin side="bottom">
      <Drawer
        source={session.text}
        editable={session.live}
        diagnostics={snapshot.diagnostics}
        {highlight}
        {reveal}
        onedit={(text) => session.edit(text)}
        {oncaret}
        {ondiagnostic}
        bind:open={session.drawerOpen}
      />
    </Margin>
  </div>
{/if}

<style>
  .workspace {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    height: 100%;
    /* The workspace fills the window and never exceeds it sideways: the score
       scrolls inside the leaf, the layout itself does not. */
    max-width: 100%;
    overflow-x: hidden;
  }

  /* Which piece, and which of its workspaces: the two facts about where you
     are, together at the head of the window. */
  .identity {
    display: flex;
    align-items: baseline;
    gap: var(--s-5);
    min-width: 0;
  }

  .title {
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    font-weight: 400;
    margin: 0;
    color: var(--ink);
  }

  .state,
  .port {
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
    white-space: nowrap;
  }

  .recovery {
    display: flex;
    align-items: baseline;
    gap: var(--s-4);
    min-width: 0;
  }

  .recovery p {
    margin: 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink);
  }

  .notice {
    margin: 0;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  .notice.stale,
  .notice.failure {
    color: var(--chalk);
  }

  .controls {
    display: flex;
    align-items: center;
    gap: var(--s-6);
  }

  .transport,
  .view,
  .zoom {
    display: flex;
    align-items: center;
    gap: var(--s-1);
  }

  /* The current view is the one set in ink; the other is an offer. */
  .text[aria-pressed="true"] {
    color: var(--ink);
  }

  /*
   * Entry is a mode, so it says so with a glyph beside its name rather than
   * with colour alone (`03-interaction.md` §5).
   */
  .duration {
    font-family: var(--f-notation);
    font-size: 1.4em;
    line-height: 1;
    padding-left: var(--s-2);
    vertical-align: -0.12em;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* Except the lens, whose whole subject is provenance. */
  .text.origin[aria-pressed="true"] {
    color: var(--plate);
  }

  /*
   * Transport and zoom are words, not pictograms. Musa ships no icon font of
   * its own, and the thesis rejects an icon toolbar: a control whose symbol
   * an engraver does not already draw is set as a word.
   */
  .text {
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

  .text:hover:not(:disabled) {
    color: var(--ink);
  }

  .text:disabled {
    color: var(--ink-faint);
    cursor: default;
  }

  .level {
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    font-variant-numeric: tabular-nums;
    color: var(--ink-muted);
    min-width: 5ch;
    text-align: center;
  }

  .body {
    display: grid;
    grid-template-columns: minmax(0, 200px) minmax(0, 1fr) minmax(0, 240px);
    min-height: 0;
  }

  /*
   * Page view is the default, so the leaf is a page: an upright sheet of
   * portrait proportions, centred, with the surround visible around it. A
   * leaf stretched to the window is a text editor with staves in it — the
   * music would be laid out across a metre-wide sheet and read as a ribbon
   * along the top.
   */
  .stage {
    display: flex;
    justify-content: center;
    min-width: 0;
    padding: 0 var(--s-4) var(--s-6);
  }

  .stage :global(> *) {
    height: 100%;
    aspect-ratio: 210 / 297;
    max-width: 100%;
    min-width: 0;
  }

  /*
   * Continuous is not a page, so the leaf stops pretending to be one: it
   * takes the full width and the music scrolls sideways through it (§4).
   */
  .stage.continuous :global(> *) {
    aspect-ratio: auto;
    width: 100%;
  }

  /*
   * Below 1100px the inspector collapses to the right margin only, and the
   * parts list narrows; the leaf never shrinks below a legible staff size.
   */
  @media (max-width: 1100px) {
    .body {
      grid-template-columns: minmax(0, 150px) minmax(0, 1fr) minmax(0, 200px);
    }

    .controls {
      gap: var(--s-4);
    }
  }

  /*
   * Narrow — a small window, or 200 % browser zoom, which WCAG asks to work
   * without loss of content. Three columns cannot hold their minimums here,
   * so the margins stop being margins and become sections above and below
   * the leaf. Nothing is hidden: a control that disappears at a zoom level
   * is a control someone cannot reach.
   */
  @media (max-width: 860px) {
    .body {
      grid-template-columns: minmax(0, 1fr);
      grid-auto-rows: min-content;
      overflow-y: auto;
    }

    .stage {
      /* Tall enough to be a page rather than a letterbox. */
      min-height: 60vh;
    }

    .controls {
      flex-wrap: wrap;
      gap: var(--s-2) var(--s-4);
    }
  }
</style>
