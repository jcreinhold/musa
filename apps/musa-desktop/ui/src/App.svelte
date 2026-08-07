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
   * unmodified keys are the score's, so typing `f` in the drawer is an `f`.
   */
  import { onMount, untrack } from "svelte";

  import Compose from "./screens/Compose.svelte";
  import Launch from "./screens/Launch.svelte";
  import Palette from "./screens/Palette.svelte";
  import KeyboardSheet from "./screens/KeyboardSheet.svelte";
  import Sheet from "./screens/Sheet.svelte";
  import Announcer from "./lib/ui/Announcer.svelte";
  import { ZOOM_STEPS } from "./lib/engrave/options";
  import { bridge } from "./lib/session/bridge";
  import { commandFor, dispatch, type Surface } from "./lib/commands/map";
  import { Session } from "./lib/session/session.svelte";
  import { ThemeChoice } from "./lib/session/theme.svelte";
  import { mark } from "./lib/perf";
  import { fixture } from "./lib/state/fixtures";
  import { Playhead, soundingAt } from "./lib/state/playhead.svelte";
  import { Workspace } from "./lib/state/selection.svelte";
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

  let paletteOpen = $state(false);
  let keysOpen = $state(false);

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

  function cycleFollow(): void {
    follow = FOLLOWS[(FOLLOWS.indexOf(follow) + 1) % FOLLOWS.length] ?? "off";
    transportSaid = follow === "off" ? "Follow off" : `Follow ${follow}`;
  }

  /** `Esc`: give up the selection first, the drawer second. */
  function escape(): void {
    if (paletteOpen || keysOpen) {
      paletteOpen = false;
      keysOpen = false;
      return;
    }
    if (workspace.selection.kind !== "none") workspace.clear();
    else session.drawerOpen = false;
  }

  const surface: Surface = {
    session,
    theme,
    workspace,
    zoom: stepZoom,
    resetZoom: () => (zoomStep = DEFAULT_STEP),
    follow: cycleFollow,
    loop: toggleLoop,
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
    const command = commandFor(event, scopeOf(event.target));
    if (!command) return;
    event.preventDefault();
    command.run(surface);
  }

  const pinned = parameters.get("theme");
  if (pinned === "light" || pinned === "dark") theme.chosen = pinned;

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
      playhead.dispose();
      void listening.then((stop) => stop());
      void commands.then((stop) => stop());
    };
  });
</script>

<svelte:window {onkeydown} />

{#if !session.live && parameters.get("view") === "sheet"}
  <Sheet fixture={chosen} />
{:else if session.snapshot}
  <Compose
    {session}
    {workspace}
    {zoom}
    {mode}
    {playing}
    loop={looped}
    {follow}
    onzoom={stepZoom}
    onpinch={pinch}
    onmode={(chosenMode) => views.choose(piece, chosenMode)}
    onloop={toggleLoop}
    onfollow={cycleFollow}
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
