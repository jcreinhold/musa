<script lang="ts">
  /**
   * The prototype's shell. There is no router and no IPC: prompt 20 exists to
   * settle the look and the engraving before a single command is wired.
   *
   * `?score=` picks a fixture. `?view=sheet` shows it as engraving alone,
   * which is the surface the raster goldens compare. `?theme=` pins a theme
   * for review; without it the application follows the system.
   */
  import Compose from "./screens/Compose.svelte";
  import Sheet from "./screens/Sheet.svelte";
  import { fixture } from "./lib/state/fixtures";

  const parameters = new URLSearchParams(globalThis.location?.search ?? "");
  const chosen = fixture(parameters.get("score"));
  const theme = parameters.get("theme");

  if (theme === "light" || theme === "dark") {
    document.documentElement.setAttribute("data-theme", theme);
  }
</script>

{#if chosen.snapshot && parameters.get("view") !== "sheet"}
  <Compose snapshot={chosen.snapshot} />
{:else}
  <Sheet fixture={chosen} />
{/if}
