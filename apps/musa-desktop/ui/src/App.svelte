<script lang="ts">
  /**
   * The application: one session, one theme, one workspace.
   *
   * Inside the desktop shell the session is live and the URL is ignored.
   * Outside it — `npm run dev`, and the raster goldens — the same components
   * run against the committed fixtures, which is what keeps the engraving
   * reviewable without a Rust build: `?score=` picks one, `?view=sheet` shows
   * it as engraving alone, `?theme=` pins a theme.
   */
  import { onMount } from "svelte";

  import Compose from "./screens/Compose.svelte";
  import Launch from "./screens/Launch.svelte";
  import Sheet from "./screens/Sheet.svelte";
  import { ZOOM_STEPS } from "./lib/engrave/options";
  import { bridge } from "./lib/session/bridge";
  import { dispatch } from "./lib/session/commands";
  import { Session } from "./lib/session/session.svelte";
  import { ThemeChoice } from "./lib/session/theme.svelte";
  import { mark } from "./lib/perf";
  import { fixture } from "./lib/state/fixtures";

  const parameters = new URLSearchParams(globalThis.location?.search ?? "");
  const session = new Session();
  const theme = new ThemeChoice();

  const DEFAULT_STEP = ZOOM_STEPS.indexOf(100);
  let zoomStep = $state(DEFAULT_STEP);
  const zoom = $derived(ZOOM_STEPS[zoomStep] ?? 100);

  function stepZoom(by: number): void {
    zoomStep = Math.min(Math.max(zoomStep + by, 0), ZOOM_STEPS.length - 1);
  }

  const surface = {
    session,
    theme,
    zoom: stepZoom,
    resetZoom: () => (zoomStep = DEFAULT_STEP),
  };

  const pinned = parameters.get("theme");
  if (pinned === "light" || pinned === "dark") theme.chosen = pinned;

  const chosen = fixture(parameters.get("score"));
  if (!session.live && chosen.snapshot) session.snapshot = chosen.snapshot;

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
      void listening.then((stop) => stop());
      void commands.then((stop) => stop());
    };
  });
</script>

{#if !session.live && parameters.get("view") === "sheet"}
  <Sheet fixture={chosen} />
{:else if session.snapshot}
  <Compose {session} {zoom} onzoom={stepZoom} />
{:else}
  <Launch onopen={() => void session.open()} onnew={() => void session.create()} />
{/if}
