<script lang="ts">
  /**
   * The application: one session, one theme, one workspace, one keyboard.
   *
   * Inside the desktop shell the session is live and the URL is ignored.
   * Outside it — `npm run dev`, and the raster goldens — the same components
   * run against the committed fixtures, which is what keeps the engraving
   * reviewable without a Rust build: `?score=` picks one, `?view=sheet` shows
   * it as engraving alone, `?theme=` pins a theme.
   *
   * The keyboard is handled here rather than in the score pane because a
   * binding belongs to the application, not to whatever happens to have focus
   * (`03-interaction.md` §3). Scope is the only thing focus decides: the
   * unmodified keys are the score's, so typing `f` in the source column is an `f`.
   */
  import { onMount, untrack } from "svelte";

  import Compose from "./screens/Compose.svelte";
  import Sound from "./screens/Sound.svelte";
  import Mix from "./screens/Mix.svelte";
  import Source from "./screens/Source.svelte";
  import Launch from "./screens/Launch.svelte";
  import Palette from "./screens/Palette.svelte";
  import KeyboardSheet from "./screens/KeyboardSheet.svelte";
  import Sheet from "./screens/Sheet.svelte";
  import Announcer from "./lib/ui/Announcer.svelte";
  import { ZOOM_STEPS } from "./lib/engrave/options";
  import { bridge } from "./lib/session/bridge";
  import type { Reveal } from "./lib/state/reveal";
  import { commandFor, dispatch, type Screen, type Surface } from "./lib/commands/map";
  import { Session } from "./lib/session/session.svelte";
  import { ThemeChoice } from "./lib/session/theme.svelte";
  import { mark } from "./lib/perf";
  import { fixture } from "./lib/state/fixtures";
  import type { Diagnostic, OutlineFacts, Span } from "./lib/state/snapshot";
  import { Playhead, soundingAt } from "./lib/state/playhead.svelte";
  import { NoteEntry } from "./lib/state/entry.svelte";
  import { played, stroke } from "./lib/state/compose";
  import type { EditDto } from "./lib/session/generated/EditDto";
  import type { InsertAtDto } from "./lib/session/generated/InsertAtDto";
  import type { EditImpact } from "./lib/state/snapshot";
  import { Workspace } from "./lib/state/selection.svelte";
  import type { Selection } from "./lib/state/selection.svelte";
  import { ViewPreferences, stepForPinch } from "./lib/state/view.svelte";

  const parameters = new URLSearchParams(globalThis.location?.search ?? "");
  const session = new Session();
  const theme = new ThemeChoice();
  const chosen = fixture(parameters.get("score"));

  const DEFAULT_STEP = ZOOM_STEPS.indexOf(100);
  let zoomStep = $state(DEFAULT_STEP);
  const zoom = $derived(ZOOM_STEPS[zoomStep] ?? 100);

  const views = new ViewPreferences();
  const piece = $derived(session.snapshot?.name ?? chosen.key);
  const mode = $derived(views.mode(piece));

  // Read through, never copied: the session replaces the snapshot on every
  // revision, and a selection is only meaningful against the current one.
  const workspace = new Workspace(() => session.snapshot);
  const playhead = new Playhead();

  /** Follow is a cycle, not a checkbox: three states, one key (§4). */
  const FOLLOWS = ["page", "continuous", "off"] as const;
  let follow = $state<(typeof FOLLOWS)[number]>("page");

  /** The looped range as event ids, so it survives a re-engraving like any other. */
  let looped = $state<[string, string] | null>(null);

  /** Which workspace is open (roadmap §14.4): all four now exist. */
  let screen = $state<Screen>("compose");

  let paletteOpen = $state(false);
  let keysOpen = $state(false);

  /**
   * Origin view (`04-provenance.md` §2): a held lens, not a mode with state to
   * get lost in. It is held here rather than in the score pane because the
   * parts list and the source column answer to it too — and because a key
   * released while the pointer is over a menu must still release the lens.
   */
  let held = $state(false);
  let pinned = $state(false);

  /**
   * Note entry (prompt 25). A mode, and never a hidden one: the duration
   * glyph sits in the top margin the whole time it is on, and the caret is
   * placed the moment it opens so there is always somewhere for a note to go.
   */
  const entry = new NoteEntry();

  /**
   * An edit against generated music, waiting for the composer to choose
   * (`04-provenance.md` §4). Holding it here rather than in the inspector is
   * what lets Origin view enter and hold itself for as long as the choice is
   * open, which is the whole reason the choice is comprehensible.
   */
  let choice = $state<{ edit: EditDto; impact: EditImpact; restore: Selection } | null>(null);

  /**
   * An extraction waiting on a name (§14.5). Held here rather than in the
   * inspector because the events it covers are the selection at the moment
   * the composer asked, and the selection is this component's to keep.
   */
  let naming = $state<{ events: string[] } | null>(null);

  const origin = $derived(held || pinned || choice !== null);

  /** A source span to put the caret at, once: the inspector's line number,
      or the diagnostic a composer just clicked (`05-states.md` §5). */
  let reveal = $state<Reveal | null>(null);
  /** Where the source caret last was, so the link does not loop (§14.4). */
  let caretAt: number | null = null;
  /** Event ids whose systems flash once, for the same reason. */
  let flash = $state<string[]>([]);

  /** A note the page should bring into view, once, when it changes. */
  let bring = $state<{ id: string } | null>(null);
  let fading: ReturnType<typeof setTimeout> | undefined;

  /** How long a diagnostic's flash lasts, matching the overlay's animation. */
  const FLASH_MS = 900;

  /** What the live regions are currently saying (§5). */
  let selectionSaid = $state("");
  let transportSaid = $state("");

  const events = $derived(session.snapshot?.score?.events ?? []);
  /**
   * The notes sounding right now. Empty while stopped, so a paused score
   * shows the selection rather than a frozen tint of where it stopped.
   */
  const playing = $derived(playhead.playing ? soundingAt(events, playhead.frame) : []);

  function stepZoom(by: number): void {
    const next = Math.min(Math.max(zoomStep + by, 0), ZOOM_STEPS.length - 1);
    if (next === zoomStep) return;
    mark("zoom");
    zoomStep = next;
  }

  /** A settled pinch lands on the nearest rung of the zoom ladder (§5). */
  function pinch(factor: number): void {
    zoomStep = stepForPinch(zoomStep, factor);
  }

  /**
   * `L`: loop the selection, or stop looping if it is already looped.
   *
   * The range is the selection's first and last event; the frames come from
   * the core, because the frontend computes nothing about time (§7).
   */
  function toggleLoop(): void {
    const ids = workspace.selected;
    const first = ids[0];
    const last = ids[ids.length - 1];
    if (looped !== null || first === undefined || last === undefined) {
      looped = null;
      void session.loop(null);
      transportSaid = "Loop off";
      return;
    }
    const from = events.find((event) => event.id === first);
    const to = events.find((event) => event.id === last);
    if (!from || !to) return;
    looped = [first, last];
    void session.loop([from.onsetFrames, to.endFrames]);
    transportSaid = `Looping bars ${from.bar} to ${to.bar}`;
  }

  /**
   * Open the source at a span, opening the source column if it is shut.
   *
   * `focus` is whether going there also takes the keyboard: true when the
   * composer asked to go and fix something, false when the text is merely
   * keeping up with the page.
   */
  function open(span: Span, focus = true): void {
    session.sourceOpen = true;
    // A new object every time, so asking for the same span twice reveals twice.
    reveal = { span: { ...span }, focus };
  }

  /**
   * A diagnostic is a place: the source caret goes there and the system it is
   * about flashes once (`05-states.md` §5). Never a toast, never a state that
   * has to be dismissed — which is why the flash clears itself.
   */
  function showDiagnostic(diagnostic: Diagnostic): void {
    if (diagnostic.span) open(diagnostic.span);
    flash = workspace.eventsForSpan(diagnostic.span);
    clearTimeout(fading);
    fading = setTimeout(() => (flash = []), FLASH_MS);
  }

  /**
   * A structural marker is a place, like a diagnostic: the selection goes to
   * the note it names, the page brings that note into view, and the source
   * follows without taking the keyboard. Nothing is edited — the outline is a
   * table of contents, and annotations are written in the source.
   */
  function goTo(row: OutlineFacts): void {
    if (row.event) workspace.select(row.event);
    // A new object every time, so choosing the same marker twice goes there
    // twice — the page may have been scrolled away in between.
    bring = row.event ? { id: row.event } : null;
    open(row.span, false);
  }

  function cycleFollow(): void {
    follow = FOLLOWS[(FOLLOWS.indexOf(follow) + 1) % FOLLOWS.length] ?? "off";
    transportSaid = follow === "off" ? "Follow off" : `Follow ${follow}`;
  }

  /** `Esc`: the choice first, then entry, then the selection, then the source column. */
  function escape(): void {
    if (paletteOpen || keysOpen) {
      paletteOpen = false;
      keysOpen = false;
      return;
    }
    if (choice) return cancelChoice();
    if (naming) {
      naming = null;
      return;
    }
    if (entry.on) return entry.set(false);
    if (workspace.selection.kind !== "none") workspace.clear();
    else session.sourceOpen = false;
  }

  /**
   * Turn note entry on, putting the caret where the next note would go.
   *
   * Entry always has a position: the selection if there is one, otherwise the
   * end of the voice the composer was last in (`03-interaction.md` §1).
   */
  function toggleEntry(): void {
    entry.toggle();
    // The keyboard is read while notes are being entered and at no other
    // time, so a session that is not entering has no MIDI poll running.
    void session.listenToMidi(entry.on);
    if (!entry.on) return;
    if (workspace.selection.kind === "none") {
      const active = workspace.active;
      if (active) workspace.placeCaret(active.part, active.voice, "end");
    }
  }

  /**
   * Issue an edit — or, when it lands on generated music, ask first.
   *
   * The counts and the affected notes come from the core; the interface's
   * whole contribution is to show them before anything changes and to hold
   * Origin view while it does (`04-provenance.md` §4).
   */
  async function issue(edit: EditDto): Promise<void> {
    const impact = await session.impact(edit);
    if (impact?.generated) {
      choice = { edit, impact, restore: workspace.selection };
      workspace.selection = { kind: "event", events: [...impact.events] };
      return;
    }
    await session.editScore(edit);
  }

  /** Confirm the choice, and report it in musical words (§4). */
  async function confirmChoice(): Promise<void> {
    const pending = choice;
    if (!pending) return;
    const count = pending.impact.occurrences;
    const said = `Edited ${pending.impact.occurrence ?? "the motif"} — ${count} ${
      count === 1 ? "occurrence" : "occurrences"
    } updated.`;
    if (await session.editScore(pending.edit, said)) choice = null;
  }

  /**
   * Take the other answer: write the change onto this occurrence instead of
   * onto the motif, which is what a `with { }` clause is for (§4).
   */
  async function specializeChoice(): Promise<void> {
    const pending = choice;
    if (!pending || pending.edit.kind !== "changePitch") return;
    const edit: EditDto = { ...pending.edit, mode: "specialize" };
    const said = `Specialized ${pending.impact.occurrence ?? "this occurrence"} — one note changed.`;
    if (await session.editScore(edit, said)) choice = null;
  }

  /**
   * Extract the selection into a motif.
   *
   * The name is asked for inline, in the margin, next to the notes it will
   * cover — never in a dialog, which would take the music off the screen at
   * the moment the composer is deciding what to call it.
   */
  function extract(): void {
    const events = workspace.selected;
    if (events.length === 0) return;
    naming = { events };
  }

  /** Name it, and the source column says so. */
  async function nameMotif(name: string): Promise<void> {
    const pending = naming;
    naming = null;
    if (!pending) return;
    await session.editScore(
      { kind: "extractMotif", events: pending.events, name },
      `Extracted ${name}() — ${pending.events.length} notes.`,
    );
  }

  /** Give up on it, putting the selection back where the composer left it. */
  function cancelChoice(): void {
    if (!choice) return;
    workspace.selection = choice.restore;
    choice = null;
  }

  const surface: Surface = {
    session,
    theme,
    workspace,
    zoom: stepZoom,
    resetZoom: () => (zoomStep = DEFAULT_STEP),
    follow: cycleFollow,
    loop: toggleLoop,
    origin: () => (pinned = !pinned),
    entry: toggleEntry,
    extract,
    show: (which) => (screen = which),
    palette: (open) => (paletteOpen = open),
    keys: (open) => (keysOpen = open),
    escape,
  };

  /**
   * Which half of the map a keystroke may reach.
   *
   * The score pane is the only place the unmodified keys mean navigation. A
   * text field never is, whichever pane it happens to sit in.
   */
  function scopeOf(target: EventTarget | null): "global" | "score" {
    const element = target instanceof Element ? target : null;
    if (element?.closest("input, textarea, [contenteditable]")) return "global";
    return element?.closest('[role="application"]') ? "score" : "global";
  }

  function onkeydown(event: KeyboardEvent): void {
    // The palette and the sheet are modal: they own every key while open,
    // and hand back only the one that closes them.
    if (paletteOpen || keysOpen) {
      if (event.key === "Escape") {
        event.preventDefault();
        escape();
      }
      return;
    }
    // The lens is a hold, not a command: it lasts exactly as long as the key
    // is down (`04-provenance.md` §2). `o` belongs to the score pane, so it is
    // still an `o` while the composer is typing in the source column; `⌥` is the
    // second way in, and works wherever the pointer is.
    if (lensKey(event) && !event.repeat) {
      mark("lens");
      held = true;
      if (event.key !== "Alt") event.preventDefault();
    }
    // While entry is on, the score pane's letters and numbers are notes and
    // durations rather than commands. Everything the entry map passes on —
    // arrows, Space, ⌘-anything — still reaches the map below.
    if (entry.on && scopeOf(event.target) === "score") {
      const asked = stroke(event, entry, workspace);
      if (asked.kind !== "pass") {
        event.preventDefault();
        if (asked.kind === "edit") void write(asked.edit, asked.at);
        return;
      }
    }
    const command = commandFor(event, scopeOf(event.target));
    if (!command) return;
    event.preventDefault();
    command.run(surface);
  }

  /**
   * Issue an entry edit and move the caret past what it wrote.
   *
   * The new note has no id until the core answers, so the caret is placed
   * afterwards by position in the voice — which is the only honest way to say
   * "after the note I just entered" when ids are the core's to mint.
   */
  async function write(edit: EditDto, at: InsertAtDto | null): Promise<void> {
    await issue(edit);
    if (at === null || at.kind === "endOfVoice") return;
    const before = at.event;
    const events = session.snapshot?.score?.events ?? [];
    const anchored = events.find((event) => event.id === before);
    if (!anchored) return;
    const voice = events.filter((e) => e.part === anchored.part && e.voice === anchored.voice);
    const index = voice.findIndex((e) => e.id === before);
    const written = at.kind === "before" ? voice[index] : voice[index + 1];
    if (written) workspace.select(written.id);
  }

  /**
   * The caret moved: select what it is inside (§14.4's other direction).
   *
   * Which notes a source offset belongs to is a containment test over spans
   * the core wrote, so the text and the page stay two views of one document
   * without the frontend parsing anything.
   */
  function followCaret(offset: number): void {
    caretAt = offset;
    const events = workspace.eventsForSpan({ start: offset, end: offset + 1 });
    if (events.length === 0) return;
    // Only when it is a different note. The caret and the selection point at
    // each other, so re-announcing what is already chosen would be the two of
    // them talking forever.
    const chosen = workspace.selected;
    if (chosen.length === events.length && chosen.every((id, at) => id === events[at])) return;
    workspace.selection = { kind: "event", events };
  }

  /**
   * The other direction of the link (roadmap §14.4): in the Source workspace,
   * choosing a note on the page is choosing its text, so the caret goes to
   * the span the note came from.
   */
  $effect(() => {
    const span = screen === "source" ? workspace.focused?.origin.span : undefined;
    if (!span) return;
    untrack(() => {
      // Not when the caret is already in that note's text: the note was
      // chosen *by* the caret, and moving the caret to where it already is
      // would be the two views arguing with each other.
      if (caretAt !== null && span.start <= caretAt && caretAt < span.end) return;
      if (reveal?.span.start === span.start && reveal.span.end === span.end) return;
      open(span, false);
    });
  });

  /** Whether a keystroke is the lens: `O` in the score, or `⌥` anywhere. */
  function lensKey(event: KeyboardEvent): boolean {
    if (event.key === "Alt") return true;
    // `⇧O` is the pin, which is a command and not a hold.
    if (event.key !== "o" || event.shiftKey) return false;
    return !event.metaKey && !event.ctrlKey && scopeOf(event.target) === "score";
  }

  function onkeyup(event: KeyboardEvent): void {
    if (event.key === "Alt" || event.key === "o" || event.key === "O") held = false;
  }

  /**
   * A key released while the window is not focused never arrives, so the lens
   * would stick on. Losing focus releases it.
   */
  function onblur(): void {
    held = false;
  }

  const pinnedTheme = parameters.get("theme");
  if (pinnedTheme === "light" || pinnedTheme === "dark") theme.chosen = pinnedTheme;

  if (!session.live && chosen.snapshot) session.snapshot = chosen.snapshot;

  // The engine owns the clock; this is the only place its position enters
  // the interface (§4).
  $effect(() => {
    const playback = session.snapshot?.playback;
    if (playback) untrack(() => playhead.receive(playback));
  });

  $effect(() => {
    const said = playhead.playing ? "Playing" : "Stopped";
    untrack(() => (transportSaid = said));
  });

  // Selection is announced politely, and only when it actually moved.
  $effect(() => {
    const event = workspace.focused;
    const kind = workspace.selection.kind;
    untrack(() => {
      selectionSaid = kind === "none" || !event ? "" : workspace.describe(event);
    });
  });

  /**
   * A note played in on the MIDI keyboard, written where the caret is.
   *
   * The same path a typed note takes — the core spelled the pitch, entry
   * supplies the duration, and the caret moves past what was written — so a
   * played note and a typed one are the same edit and the same undo.
   */
  session.played = ({ pitches }) => {
    if (!entry.on) return;
    const asked = played(pitches, entry, workspace);
    if (asked.kind === "edit") void write(asked.edit, asked.at);
  };

  onMount(() => {
    // The shell frame is on screen now; the score arrives when the worker has
    // laid it out, and must never have been waited for (B6, `05-states.md` §3).
    requestAnimationFrame(() => mark("shell"));
    const stopFollowing = theme.start();
    const listening = session.start();
    const commands = session.live
      ? bridge.on("musa://command", (id) => dispatch(id, surface))
      : Promise.resolve(() => {});
    return () => {
      stopFollowing();
      clearTimeout(fading);
      playhead.dispose();
      void listening.then((stop) => stop());
      void commands.then((stop) => stop());
    };
  });
</script>

<svelte:window {onkeydown} {onkeyup} {onblur} />

{#if !session.live && parameters.get("view") === "sheet"}
  <Sheet fixture={chosen} />
{:else if session.snapshot && screen === "sound"}
  <Sound {session} onshow={(which) => (screen = which)} />
{:else if session.snapshot && screen === "mix"}
  <Mix {session} onshow={(which) => (screen = which)} />
{:else if session.snapshot && screen === "source"}
  <Source
    {session}
    {workspace}
    {zoom}
    {mode}
    {reveal}
    {origin}
    oncaret={followCaret}
    ondiagnostic={showDiagnostic}
    onshow={(which) => (screen = which)}
  />
{:else if session.snapshot}
  <Compose
    {session}
    {workspace}
    {zoom}
    {mode}
    {playing}
    loop={looped}
    {follow}
    {origin}
    {pinned}
    {entry}
    choice={choice?.impact ?? null}
    naming={naming?.events.length ?? null}
    {flash}
    {reveal}
    onzoom={stepZoom}
    onpinch={pinch}
    onmode={(chosenMode) => views.choose(piece, chosenMode)}
    onloop={toggleLoop}
    onfollow={cycleFollow}
    onpin={() => (pinned = !pinned)}
    onentry={toggleEntry}
    onconfirm={() => void confirmChoice()}
    onspecialize={() => void specializeChoice()}
    oncancel={cancelChoice}
    onname={(name) => void nameMotif(name)}
    oncancelname={() => (naming = null)}
    onpitch={(event, pitch) =>
      void issue({ kind: "changePitch", event, pitch, mode: "editDefinition" })}
    onduration={(event, duration) =>
      void issue({ kind: "changeDuration", event, duration, mode: "editDefinition" })}
    onheader={(field, value) => void session.editScore({ kind: "setHeader", field, value })}
    onreveal={open}
    ondiagnostic={showDiagnostic}
    oncaret={followCaret}
    onshow={(which) => (screen = which)}
    onoutline={goTo}
    {bring}
  />
{:else}
  <Launch onopen={() => void session.open()} onnew={() => void session.create()} />
{/if}

{#if paletteOpen}
  <Palette
    onrun={(command) => {
      paletteOpen = false;
      command.run(surface);
    }}
    onclose={() => (paletteOpen = false)}
  />
{/if}

{#if keysOpen}
  <KeyboardSheet onclose={() => (keysOpen = false)} />
{/if}

<Announcer selection={selectionSaid} transport={transportSaid} />
