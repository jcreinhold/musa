<script lang="ts">
  /**
   * The Mix workspace (roadmap §14.4): how the parts sit against each other.
   *
   * The rows are the studio's own statements — a part with the patch that
   * realizes it, a send with its level, a bus with what it does, a route with
   * where it goes. Nothing here is a mixer channel the language does not have:
   * there is no per-part fader in `.musa`, so there is none on this screen
   * either, and a part's level is the `gain` stage its patch actually writes
   * (§11 — inventing a control would mean inventing a second authority for the
   * value behind it).
   *
   * Every level is shown in decibels, whichever unit it was written in, so one
   * scale reads across the whole mix.
   */
  import Margin from "../lib/ui/Margin.svelte";
  import ParamControl from "../lib/ui/ParamControl.svelte";
  import Workspaces from "../lib/ui/Workspaces.svelte";
  import type { Screen } from "../lib/commands/map";
  import type { Session } from "../lib/session/session.svelte";
  import { volumeOf, type ContainerFacts } from "../lib/state/snapshot";
  import type { StudioEditDto } from "../lib/session/generated/StudioEditDto";

  const exact = (value: { numerator: number; denominator: number }) => value.numerator / value.denominator;

  let {
    session,
    onshow,
  }: {
    session: Session;
    onshow: (which: Screen) => void;
  } = $props();

  const snapshot = $derived(session.snapshot);
  /** Whether the switcher offers the contents page (`07-the-volume.md`). */
  const volume = $derived(volumeOf(snapshot) !== null);
  const studio = $derived(snapshot?.studio ?? null);

  /** A part's level is whatever `gain` its patch writes — no more, no less. */
  function levels(patch: string | null): { container: ContainerFacts; stage: number }[] {
    const found = studio?.patches.find((it) => it.name === patch);
    if (!found) return [];
    return found.stages
      .filter((stage) => stage.processor === "gain")
      .map((stage) => ({ container: found, stage: stage.index }));
  }

  function paramOf(container: ContainerFacts, stage: number, name: string) {
    return container.stages[stage]?.params.find((param) => param.name === name);
  }

  function edit(next: StudioEditDto, said?: string): void {
    void session.editStudio(next, said);
  }

  function setParam(container: ContainerFacts, stage: number, param: string, value: number): void {
    edit({
      kind: "setParam",
      container: container.kind,
      name: container.name,
      stage,
      param,
      value,
    });
  }

  /** Where a source's signal goes, in the studio's own words. */
  function destination(source: string): string | null {
    return studio?.routes.find((route) => route.source === source)?.destination ?? null;
  }

  function assetStatus(path: string): string {
    return snapshot?.assets.find((asset) => asset.path === path)?.status ?? "undeclared";
  }

  const sendMax = 6;
  const sendMin = -60;
</script>

{#if snapshot}
  <div class="mix-workspace">
    <Margin side="top">
      <div class="identity">
        <h1 class="title">{snapshot.score?.title ?? ""}</h1>
        <Workspaces current="mix" {volume} {onshow} />
      </div>
      {#if session.notice}
        <p class="notice" class:failure={session.notice.tone === "failure"} role="status">
          {session.notice.message}
        </p>
      {/if}
      {#if session.stale}
        <p class="notice" role="status">
          Mix is showing the last valid prepared plan while the current source is repaired.
        </p>
      {/if}
    </Margin>

    <main class="stage">
      {#if !studio?.declared}
        <p class="empty">
          This piece has no <code>studio</code> block, so there is nothing to balance yet: every part sounds through the built-in
          voice, straight to the master.
        </p>
      {:else}
        <section class="group" aria-label="Parts">
          <h2 class="group-name">Parts</h2>
          {#each studio.assignments as row (row.part)}
            <article class="strip" aria-label={`${row.part} part output`}>
              <header class="strip-head">
                <h3 class="strip-name">{row.part} <span class="kind">part output</span></h3>
                <p class="strip-route">
                  {row.instrument}{#if destination(row.part)}
                    <span class="arrow" aria-hidden="true">→</span>{destination(row.part)}{/if}
                </p>
              </header>
              {#each levels(row.instrument) as level (level.stage)}
                {@const param = paramOf(level.container, level.stage, "gain")}
                {#if param}
                  <ParamControl
                    {param}
                    id={`mix-${row.part}-${level.stage}-gain`}
                    editable={session.live}
                    onchange={(value) => setParam(level.container, level.stage, "gain", value)}
                  />
                {/if}
              {/each}
              {#each studio.sends.filter((send) => send.source === row.part) as send (send.bus)}
                <div class="send">
                  <label class="send-name" for={`send-${send.source}-${send.bus}`}>send to {send.bus}</label>
                  <input
                    id={`send-${send.source}-${send.bus}`}
                    class="track"
                    type="range"
                    min={sendMin}
                    max={sendMax}
                    step="0.1"
                    value={exact(send.decibels)}
                    disabled={!session.live}
                    onchange={(event) =>
                      edit({
                        kind: "setSendLevel",
                        source: send.source,
                        bus: send.bus,
                        decibels: event.currentTarget.valueAsNumber,
                      })}
                  />
                  <output class="send-value" for={`send-${send.source}-${send.bus}`}
                    >{exact(send.decibels).toFixed(1)}<span class="unit">dB</span></output
                  >
                </div>
              {/each}
            </article>
          {/each}
        </section>

        {#if studio.media.length > 0}
          <section class="group" aria-label="Recorded media">
            <h2 class="group-name">Recorded media</h2>
            {#each studio.media as media (media.name)}
              <article class="strip" aria-label={`${media.name} recorded-media source`}>
                <header class="strip-head">
                  <div>
                    <h3 class="strip-name">{media.name} <span class="kind">media source</span></h3>
                    <p class="media-fact">
                      {media.kind === "musical-clip" ? `clip · ${media.fit}` : "fixed cue"} · {media.occurrences}
                      {media.occurrences === 1 ? " occurrence" : " occurrences"} · {assetStatus(media.asset)}
                    </p>
                  </div>
                  <p class="strip-route">
                    {media.asset}{#if destination(media.name)}
                      <span class="arrow" aria-hidden="true">→</span>{destination(media.name)}{/if}
                  </p>
                </header>
                {#each studio.sends.filter((send) => send.source === media.name) as send (send.bus)}
                  <div class="send">
                    <label class="send-name" for={`send-${send.source}-${send.bus}`}>send to {send.bus}</label>
                    <input
                      id={`send-${send.source}-${send.bus}`}
                      class="track"
                      type="range"
                      min={sendMin}
                      max={sendMax}
                      step="0.1"
                      value={exact(send.decibels)}
                      disabled={!session.live}
                      onchange={(event) =>
                        edit({
                          kind: "setSendLevel",
                          source: send.source,
                          bus: send.bus,
                          decibels: event.currentTarget.valueAsNumber,
                        })}
                    />
                    <output class="send-value" for={`send-${send.source}-${send.bus}`}
                      >{exact(send.decibels).toFixed(1)}<span class="unit">dB</span></output
                    >
                  </div>
                {/each}
              </article>
            {/each}
          </section>
        {/if}

        {#if studio.buses.length > 0}
          <section class="group" aria-label="Buses">
            <h2 class="group-name">Buses</h2>
            {#each studio.buses as bus (bus.name)}
              <article
                class="strip"
                aria-label={`${bus.name}: ${bus.stages.map((stage) => stage.summary).join(" ")}`}
                title={bus.stages.map((stage) => `${stage.processor}: ${stage.summary}`).join("\n")}
              >
                <header class="strip-head">
                  <h3 class="strip-name">{bus.name}</h3>
                  <p class="strip-route">
                    {bus.stages.map((stage) => stage.processor).join(" → ")}{#if destination(bus.name)}
                      <span class="arrow" aria-hidden="true">→</span>{destination(bus.name)}{/if}
                  </p>
                </header>
                {#each bus.stages as stage (stage.index)}
                  {#each stage.params as param (param.name)}
                    <ParamControl
                      {param}
                      id={`mix-${bus.name}-${stage.index}-${param.name}`}
                      editable={session.live}
                      onchange={(value) => setParam(bus, stage.index, param.name, value)}
                    />
                  {/each}
                {/each}
              </article>
            {/each}
          </section>
        {/if}
        <section class="group" aria-label="Main output">
          <h2 class="group-name">Main output</h2>
          <article class="strip main-output">
            <h3 class="strip-name">main</h3>
            <p class="media-fact">The terminal output of the source-authored route graph.</p>
          </article>
        </section>
      {/if}
    </main>
  </div>
{:else}
  <Margin side="bottom"><p class="empty">Nothing open.</p></Margin>
{/if}

<style>
  .mix-workspace {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
    min-height: 0;
    max-width: 100%;
    overflow-x: hidden;
  }

  .identity {
    display: flex;
    align-items: baseline;
    gap: var(--s-5);
    min-width: 0;
  }

  .title {
    margin: 0;
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    font-weight: 400;
    color: var(--ink);
  }

  .notice {
    margin: 0;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  .notice.failure {
    color: var(--chalk);
  }

  .stage {
    min-height: 0;
    overflow: auto;
    padding: var(--s-6);
  }

  .group {
    max-width: 40rem;
    margin-bottom: var(--s-8);
  }

  .group-name {
    margin: 0 0 var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    letter-spacing: var(--tracking-micro);
    text-transform: uppercase;
    color: var(--ink-muted);
  }

  /* A strip is a part and what happens to it, not a channel of a console. */
  .strip {
    padding: var(--s-4) 0;
    border-top: 1px solid var(--rule);
  }

  .strip-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-4);
    margin-bottom: var(--s-2);
  }

  .strip-name {
    margin: 0;
    font-family: var(--f-score-text);
    font-size: var(--t-name-size);
    line-height: var(--t-name-line);
    font-weight: 400;
    color: var(--ink);
  }

  .kind {
    padding-left: var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-micro-size);
    color: var(--ink-muted);
  }

  .media-fact {
    margin: var(--s-1) 0 0;
    font-family: var(--f-ui);
    font-size: var(--t-micro-size);
    color: var(--ink-muted);
  }

  .main-output {
    border-bottom: 1px solid var(--rule);
  }

  .strip-route {
    margin: 0;
    font-family: var(--f-mono);
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    color: var(--ink-faint);
  }

  .arrow {
    padding: 0 0.5ch;
  }

  .send {
    display: grid;
    grid-template-columns: 7em minmax(0, 1fr) 6em;
    align-items: baseline;
    gap: var(--s-3);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
  }

  .send-name {
    color: var(--ink-muted);
  }

  .send-value {
    font-family: var(--f-mono);
    font-size: var(--t-value-size);
    line-height: var(--t-value-line);
    color: var(--ink);
    text-align: right;
  }

  .unit {
    padding-left: 0.4ch;
    color: var(--ink-muted);
  }

  .track {
    appearance: none;
    width: 100%;
    height: var(--s-4);
    background: none;
    cursor: pointer;
  }

  .track::-webkit-slider-runnable-track {
    height: 1px;
    background: var(--rule);
  }

  .track::-webkit-slider-thumb {
    appearance: none;
    width: 3px;
    height: var(--s-3);
    margin-top: calc(var(--s-3) / -2);
    background: var(--ink);
    border: 0;
    border-radius: 0;
  }

  .track:disabled::-webkit-slider-thumb {
    background: var(--ink-faint);
  }

  .track:focus-visible::-webkit-slider-thumb {
    outline: 1px solid var(--ink);
    outline-offset: 2px;
  }

  .empty {
    margin: 0;
    max-width: 32rem;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }
</style>
