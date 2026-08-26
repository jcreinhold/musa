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
  import type { ViewMode } from "musa-engrave";
  import Leaf from "../lib/ui/Leaf.svelte";
  import Margin from "../lib/ui/Margin.svelte";
  import GlyphButton from "../lib/ui/GlyphButton.svelte";
  import TransportReadout from "../lib/ui/TransportReadout.svelte";
  import Workspaces from "../lib/ui/Workspaces.svelte";
  import { REPEAT_RIGHT_LEFT } from "../lib/ui/glyphs";
  import type { Screen } from "../lib/commands/map";
  import { SOURCE_FLOOR, type Preferences } from "../lib/session/preferences.svelte";
  import type { Session } from "../lib/session/session.svelte";
  import type { Workspace } from "../lib/state/selection.svelte";
  import type { Reveal } from "../lib/state/reveal";
  import type { Focus } from "../lib/state/focus.svelte";
  import type { Candidate } from "../lib/state/gesture.svelte";
  import {
    volumeOf,
    type AssignmentFacts,
    type Diagnostic,
    type EditImpact,
    type OutlineFacts,
    type Span,
  } from "../lib/state/snapshot";
  import type { HeaderFieldDto } from "../lib/session/generated/HeaderFieldDto";
  import type { NoteEntry } from "../lib/state/entry.svelte";
  import SourcePane from "../lib/ui/SourcePane.svelte";
  import Inspector from "./Inspector.svelte";
  import Outline from "./Outline.svelte";
  import PartsList from "./PartsList.svelte";
  import RunningOrder from "./RunningOrder.svelte";

  let {
    session,
    preferences,
    workspace,
    zoom,
    mode,
    playing,
    loop,
    follow,
    origin,
    focus,
    sounding,
    onvisible,
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
    candidate,
    onedit,
    oncandidate,
    oninsert,
    onpitch,
    onduration,
    onheader,
    onkeep,
    onreveal,
    ondiagnostic,
    oncaret,
    onshow,
    onoutline,
    onchoose,
    bring,
  }: {
    session: Session;
    /** Text size and vim mode, for the source column. */
    preferences: Preferences;
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
    /** The shared focus, marked in both columns at once. */
    focus: Focus;
    /** Where the music on the visible pages was written, as source spans. */
    sounding: Span[];
    /** The score reports what it has engraved on screen. */
    onvisible: (ids: string[]) => void;
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
    /** What a pointer gesture in flight would write, drawn in the source. */
    candidate: { start: number; end: number; text: string } | null;
    /** A gesture came up: write it. */
    onedit: (candidate: Candidate) => void;
    /** A gesture moved: ask what it would write. */
    oncandidate: (candidate: Candidate | null) => void;
    /** A click on an empty staff step, with entry armed. */
    oninsert: (pitch: string) => void;
    onpitch: (event: string, pitch: string) => void;
    onduration: (event: string, duration: string) => void;
    /** Rewrite one of the piece's own statements. */
    onheader: (field: HeaderFieldDto, value: string) => void;
    /** Keep a decision as it came out, or let it go back to being drawn. */
    onkeep: (path: string, keep: boolean) => void;
    onreveal: (span: Span) => void;
    ondiagnostic: (diagnostic: Diagnostic) => void;
    /** The source caret moved; the score follows it (roadmap §14.4). */
    oncaret: (offset: number) => void;
    onshow: (which: Screen) => void;
    /** A structural marker was chosen: go to the place it names. */
    onoutline: (row: OutlineFacts) => void;
    /** A piece of the project was chosen: turn to it. */
    onchoose: (file: string) => void;
    /** A note to bring into view, once, when it changes. */
    bring: { id: string } | null;
  } = $props();

  const snapshot = $derived(session.snapshot);
  const score = $derived(snapshot?.score ?? null);
  /**
   * The project's listing, when there is more than one file in it. Null for a
   * loose piece, and the margin then shows nothing at all — a project of one
   * must look exactly as it did before projects existed.
   */
  const contents = $derived(volumeOf(snapshot));
  const focused = $derived(workspace.focused);
  const sound = $derived.by((): AssignmentFacts | null => {
    if (!chosen) return null;
    return snapshot?.studio?.assignments.find((assignment) => assignment.part === chosen.part) ?? null;
  });
  // What the inspector describes is what the composer picked, not where work
  // is happening: with nothing picked it shows the piece instead.
  const chosen = $derived(workspace.chosen);
  // Which decision the picked note was played under. The core resolved that;
  // this is the lookup it hands over, and it is null in every determinate
  // piece.
  const decision = $derived(
    chosen?.origin.decision !== null && chosen?.origin.decision !== undefined
      ? (score?.decisions?.[chosen.origin.decision] ?? null)
      : null,
  );
  const problems = $derived(snapshot?.diagnostics.filter((d) => d.severity === "error") ?? []);

  function midiSeconds(micros: number): string {
    return `${(micros / 1_000_000).toFixed(micros < 10_000_000 ? 1 : 0)} s`;
  }

  function setRecentMidi(enabled: boolean): void {
    preferences.setRecentMidi(enabled);
  }

  /**
   * The narrowest a leaf is still a page. Below this the staves are a ribbon
   * and the composer has stopped looking at music, so it is where the source
   * column stops taking room.
   */
  const STAGE_MIN = 320;

  /** The stage, so the seam can ask how much of it is still spare. */
  let stage: HTMLElement | null = $state(null);

  /**
   * Where the selection is in the piece's structure: every passage that
   * contains it, so a phrase and the section it sits in both light. Reading
   * position, not a second selection — clicking a row moves the selection,
   * and the row lights because the selection is there.
   */
  const outlineAt = $derived.by(() => {
    const rows = score?.outline ?? [];
    const at = focused?.onsetFrames;
    if (at === undefined) return [];
    return rows.filter((row) => at >= row.onsetFrames && at < row.endFrames);
  });

  /**
   * What the source column marks: the declaration and the use of the expansion in
   * view. The workspace decides which expansion that is, so the column and
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

{#if snapshot}
  <div class="workspace">
    <Margin side="top">
      <div class="identity">
        <h1 class="title">{score?.title ?? snapshot.name}</h1>
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
          <span class="state">{snapshot.autosaved ? "Unsaved — recovery copy kept" : "Unsaved"}</span>
        {/if}
        <Workspaces current="compose" volume={contents !== null} {onshow} />
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
          <button type="button" class="text" onclick={() => void session.recover(true)}>Restore it</button>
          <button type="button" class="text" onclick={() => void session.recover(false)}>Discard it</button>
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
          <button type="button" class="text" aria-pressed={follow !== "off"} onclick={onfollow}>Follow</button>
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
          >Notes{#if entry.on}<span class="duration" aria-hidden="true">{entry.glyph}</span><span
              class="visually-hidden"
            >
              — duration {entry.duration}</span
            >{/if}</button
        >

        <div class="midi" role="group" aria-label="MIDI keyboard capture">
          {#if snapshot.midiDevices.length > 1 || snapshot.midiPort === null}
            <label>
              <span class="visually-hidden">MIDI input</span>
              <select
                aria-label="MIDI input"
                value={snapshot.midiDevices.find((device) => device.selected)?.id ?? ""}
                onchange={(event) => {
                  const id = event.currentTarget.value;
                  if (id) void session.selectMidiInput(id);
                }}
              >
                <option value="">Choose MIDI input</option>
                {#each snapshot.midiDevices as device (device.id)}
                  <option value={device.id}>{device.name}</option>
                {/each}
              </select>
            </label>
          {/if}
          {#if snapshot.midiPort}
            <span class="port" title="Auditions the selected source instrument; never writes notes">
              {sound?.instrument ?? "Instrument"} — {snapshot.midiPort}
            </span>
            <button
              type="button"
              class="text"
              aria-pressed={snapshot.midiCapture.state === "capturing"}
              onclick={() =>
                void (snapshot.midiCapture.state === "capturing"
                  ? session.stopMidiCapture()
                  : session.startMidiCapture(workspace.caretAt?.id ?? null))}
              >{snapshot.midiCapture.state === "capturing" ? "Finish" : "Capture"}</button
            >
            {#if snapshot.midiCapture.recentEnabled}
              <button
                type="button"
                class="text"
                disabled={snapshot.midiCapture.recentEvents === 0}
                onclick={() => void session.keepRecentMidi(workspace.caretAt?.id ?? null)}>Keep that</button
              >
              <button
                type="button"
                class="text recent"
                aria-pressed="true"
                title="Turn off and clear recent phrase memory"
                onclick={() => setRecentMidi(false)}
              >
                {snapshot.midiCapture.state === "capturing"
                  ? `Capturing ${midiSeconds(snapshot.midiCapture.captureMicros)}`
                  : snapshot.midiCapture.recentTruncated
                    ? `Recent suffix only · ${midiSeconds(snapshot.midiCapture.recentMicros)}`
                    : `Recent phrase on · ${midiSeconds(snapshot.midiCapture.recentMicros)}`}
              </button>
              <button
                type="button"
                class="text"
                disabled={snapshot.midiCapture.recentEvents === 0}
                onclick={() => void session.clearRecentMidi()}>Clear</button
              >
            {:else}
              <button type="button" class="text" onclick={() => setRecentMidi(true)}>Recent phrase off</button>
            {/if}
          {:else}
            <span class="port" role="status">
              {snapshot.midiCapture.state === "disconnected"
                ? "Keyboard disconnected — capture preserved"
                : "No MIDI keyboard — choose an input device"}
            </span>
          {/if}
        </div>

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
        <!-- Where you are in a score there is not: nothing to read out. -->
        {#if score}
          <TransportReadout
            {score}
            playback={snapshot.playback}
            stale={session.stale}
            bar={focused?.bar ?? 1}
            beat={focused?.beat ?? { numerator: 1, denominator: 1 }}
            onheader={session.live ? onheader : undefined}
          />
        {/if}
        <!--
          Two words, one of them current: page or continuous (§4). A toggle
          named for what it shows, rather than a pair of icons a reader has to
          learn.
        -->
        <div class="view" role="group" aria-label="View">
          <button type="button" class="text" aria-pressed={mode === "page"} onclick={() => onmode("page")}>Pages</button
          >
          <button type="button" class="text" aria-pressed={mode === "continuous"} onclick={() => onmode("continuous")}
            >Continuous</button
          >
        </div>

        <div class="zoom">
          <button type="button" class="text" onclick={() => onzoom(-1)} aria-label="Zoom out">−</button>
          <span class="level">{zoom}&thinsp;%</span>
          <button type="button" class="text" onclick={() => onzoom(1)} aria-label="Zoom in">+</button>
        </div>
      </div>
    </Margin>

    <div class="body" class:with-source={session.sourceOpen}>
      <!--
        The source, when it is asked for: a column at the left edge on its own
        material, not a band across the bottom. It takes width, which this
        screen has spare, and gives back height, which the page needs — and it
        stands where the Source workspace puts it, so `⌘4` changes the
        proportion rather than the arrangement (`01-visual-language.md` §7).
      -->
      {#if session.sourceOpen}
        <SourcePane
          source={session.text}
          editable={session.live}
          diagnostics={snapshot.diagnostics}
          {highlight}
          {reveal}
          focus={focus.marked}
          {sounding}
          {candidate}
          onpoint={(line) => focus.pointLine(line)}
          modal={preferences.vim}
          onedit={(text) => session.edit(text)}
          {oncaret}
          onundo={() => void session.undo()}
          onredo={() => void session.redo()}
          onsave={() => void session.save()}
          {ondiagnostic}
          onhide={() => (session.sourceOpen = false)}
          width={preferences.sourceWidth}
          floor={SOURCE_FLOOR}
          spare={() => (stage?.clientWidth ?? STAGE_MIN) - STAGE_MIN}
          onwiden={(width) => preferences.widenSource(width)}
          onreset={() => preferences.resetSource()}
        />
      {/if}

      <!--
        The margin reads outside in: volume, piece, structure. The running
        order is only there when there is more than one file to choose
        between (`07-the-volume.md`).
      -->
      <Margin side="left" label="Parts">
        {#if contents}
          <RunningOrder {contents} {onchoose} />
        {/if}
        {#if score}
          <div class="parts" class:after={contents !== null}>
            <PartsList parts={score.parts} {workspace} {origin} />
            <Outline outline={score.outline} active={outlineAt} onselect={onoutline} />
          </div>
        {/if}
      </Margin>

      <main class="stage" class:continuous={mode === "continuous"} bind:this={stage}>
        <Leaf stale={session.stale}>
          {#if score}
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
              {focus}
              {onvisible}
              {entry}
              spell={!session.sourceOpen}
              onedit={session.live ? onedit : undefined}
              oncandidate={session.live ? oncandidate : undefined}
              oninsert={session.live ? oninsert : undefined}
              {flash}
              {bring}
              header={score.header}
              onheader={session.live ? onheader : undefined}
            />
          {:else}
            <!--
              A file that parses and yields no score is not a blank window.
              Material has no score by definition and opens in the text
              instead; a piece that has never compiled keeps the frame, and
              what it has to say is in its diagnostics.
            -->
            <p class="empty">
              {problems.length > 0 ? "No score yet — the source has problems." : "No score in this file."}
            </p>
          {/if}
        </Leaf>
      </main>

      <Margin side="right" label="Inspector">
        {#if score}
          <Inspector
            event={chosen}
            adrift={workspace.adrift}
            occurrence={workspace.selectedOccurrence}
            {choice}
            {naming}
            onorigin={(depth) => workspace.selectOrigin(depth)}
            {decision}
            onkeep={session.live ? onkeep : undefined}
            siblings={focus.spelled(chosen?.origin.definitionSpan)}
            {onconfirm}
            {onspecialize}
            {oncancel}
            {onname}
            {oncancelname}
            onpitch={session.live && chosen ? (pitch) => onpitch(chosen.id, pitch) : undefined}
            onduration={session.live && chosen ? (duration) => onduration(chosen.id, duration) : undefined}
            {onreveal}
            {sound}
            onsound={session.live
              ? (part) =>
                  void session.editStudio({ kind: "makeSoundExplicit", part }, `${part}'s sound is now written.`)
              : undefined}
            header={score.header}
            onheader={session.live ? onheader : undefined}
          />
        {/if}
      </Margin>
    </div>
  </div>
{/if}

<style>
  .workspace {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
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
    flex-wrap: wrap;
    gap: var(--s-2) var(--s-4);
    min-width: 0;
  }

  /*
   * The title is the one thing in the band whose length nobody controls, so it
   * is the one thing that gives way: it sets on one line and truncates, rather
   * than wrapping to a second line inside a row that is one line tall. A
   * control never gives way to a long name.
   */
  .title {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    font-weight: 400;
    margin: 0;
    color: var(--ink);
  }

  .state {
    flex: none;
  }

  /* The running order and the parts are two readings of two different
     things, so they are separated by a rest rather than a rule — the same
     separation the outline takes from the parts above it. */
  .parts.after {
    margin-top: var(--s-8);
  }

  /* A page with nothing engraved on it still has margins: the message sits
     where the first system would. */
  .empty {
    margin: 0;
    padding: var(--s-16);
    font-family: var(--f-score-text);
    font-size: var(--t-name-size);
    line-height: var(--t-name-line);
    color: var(--ink-muted);
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

  /*
   * The controls keep the right edge of the band whether they share the first
   * row with the title or take a row of their own — `margin-left: auto` does
   * both, where `justify-content` can only do the first. They wrap among
   * themselves for the same reason the band does: at the window's own minimum
   * size there is more here than one line holds, and every one of these is
   * either the only way to reach something (the view mode) or a control a
   * pointer user at 200 % zoom still has to be able to click.
   */
  .controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    justify-content: flex-end;
    margin-left: auto;
    gap: var(--s-2) var(--s-5);
  }

  /* Each of these is one control that happens to be several buttons, so each
     breaks whole: "Zoom out 100 % Zoom in" split across two rows is not a
     zoom control any more. */
  .transport,
  .midi,
  .view,
  .zoom {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--s-1);
  }

  .recent {
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
    white-space: nowrap;
  }

  .midi select {
    max-width: 14rem;
    border: 0;
    background: transparent;
    color: var(--ink);
    font: inherit;
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
   * The source column sizes itself; the track just follows. What this screen
   * owns is the cap: the page is the subject here, so it is served first, and
   * the text gets what is left over after the two margins and a stage wide
   * enough to hold an upright A4 leaf. Above about 1250px that arithmetic
   * comes out larger than the measure and the measure wins, which is every
   * window this app is happy in; below it the column narrows a character at a
   * time rather than the page collapsing all at once.
   *
   * The floor stops the trade before the column becomes a gutter: 300px is
   * still wider than nine in ten lines in the language.
   */
  .body.with-source {
    --source-cap: max(300px, calc(100vw - 440px - var(--stage-floor)));
    /* What a *deliberate* ask may take: the same arithmetic against the floor
       below which a leaf stops being a page, rather than the roomier one the
       automatic width is held to. */
    --source-room: max(300px, calc(100vw - 440px - 320px));
    --stage-floor: 420px;

    grid-template-columns: auto minmax(0, 200px) minmax(0, 1fr) minmax(0, 240px);
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

    .body.with-source {
      /* Narrower margins here, so the column gets the difference back. */
      --source-cap: max(300px, calc(100vw - 350px - var(--stage-floor)));
      --source-room: max(300px, calc(100vw - 350px - 320px));

      grid-template-columns: auto minmax(0, 150px) minmax(0, 1fr) minmax(0, 200px);
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
    .body,
    .body.with-source {
      grid-template-columns: minmax(0, 1fr);
      grid-auto-rows: min-content;
      overflow-y: auto;
    }

    .stage {
      /* Tall enough to be a page rather than a letterbox. */
      min-height: 60vh;
    }
  }
</style>
