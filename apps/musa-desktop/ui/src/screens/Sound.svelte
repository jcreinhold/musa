<script lang="ts">
  /**
   * The Sound workspace (roadmap §14.4): what a part sounds like.
   *
   * A patch is a chain, and it is drawn as one — top to bottom, in the order
   * the `|>` operator runs it. Not a node canvas: the source has no canvas,
   * and a view that invented one would have to invent positions to remember,
   * which is a second document (roadmap §11).
   *
   * Every control here writes source. There is no local model of a patch: the
   * values come from the compiled studio and go back as text edits, so what
   * the composer hears and what the file says cannot come apart.
   */
  import Margin from "../lib/ui/Margin.svelte";
  import ParamControl from "../lib/ui/ParamControl.svelte";
  import Workspaces from "../lib/ui/Workspaces.svelte";
  import type { Screen } from "../lib/commands/map";
  import type { Session } from "../lib/session/session.svelte";
  import { volumeOf, type ContainerFacts } from "../lib/state/snapshot";
  import type { StudioEditDto } from "../lib/session/generated/StudioEditDto";

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
  const parts = $derived(studio?.assignments ?? []);

  /** Which part's sound is being read. The first, until one is chosen. */
  let chosen = $state<string | null>(null);
  const part = $derived(parts.find((row) => row.part === chosen) ?? parts[0] ?? null);

  const patch = $derived(
    part?.patch ? (studio?.patches.find((it) => it.name === part.patch) ?? null) : null,
  );

  /** Every patch a part could be pointed at. */
  const choices = $derived(studio?.patches.map((it) => it.name) ?? []);

  function edit(edit: StudioEditDto, said?: string): void {
    void session.editStudio(edit, said);
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
</script>

{#if snapshot}
  <div class="sound-workspace">
    <Margin side="top">
      <div class="identity">
        <h1 class="title">{snapshot.score?.title ?? ""}</h1>
        <Workspaces current="sound" volume={volume} {onshow} />
      </div>
      {#if session.notice}
        <p class="notice" class:failure={session.notice.tone === "failure"} role="status">
          {session.notice.message}
        </p>
      {/if}
    </Margin>

    <div class="body">
      <Margin side="left" label="Parts">
        <ul class="parts">
          {#each parts as row (row.part)}
            <li>
              <button
                type="button"
                class="part"
                aria-current={row.part === part?.part ? "true" : undefined}
                onclick={() => (chosen = row.part)}
              >
                <span class="part-name">{row.part}</span>
                <span class="part-patch">{row.patch ?? "built-in voice"}</span>
              </button>
            </li>
          {/each}
        </ul>
      </Margin>

      <main class="stage">
        {#if !studio?.declared}
          <!-- A piece with no studio is not broken; it sounds through the
               built-in voice, and saying so is more use than an empty rack
               (`05-states.md` §2, §14.8). -->
          <p class="empty">
            This piece has no <code>studio</code> block, so every part sounds through the built-in
            voice. Write one in the Source workspace to shape it.
          </p>
        {:else if part}
          <section class="patch" aria-label="Patch">
            <header class="assignment">
              <h2 class="heading">{part.part}</h2>
              <label class="picker">
                <span class="picker-label">plays through</span>
                <select
                  disabled={!session.live || choices.length === 0}
                  value={part.patch ?? ""}
                  onchange={(event) => {
                    const patchName = event.currentTarget.value;
                    if (patchName) {
                      edit(
                        { kind: "assignPatch", part: part.part, patch: patchName },
                        `${part.part} plays through ${patchName}.`,
                      );
                    }
                  }}
                >
                  {#if !part.patch}
                    <option value="">the built-in voice</option>
                  {/if}
                  {#each choices as name (name)}
                    <option value={name}>{name}</option>
                  {/each}
                </select>
              </label>
            </header>

            {#if patch}
              <ol class="chain">
                {#each patch.stages as stage (stage.index)}
                  <li class="stage-row">
                    <h3 class="stage-name">
                      <span class="processor">{stage.processor}</span>{#if stage.label}<span
                          class="label">{stage.label}</span
                        >{/if}
                    </h3>
                    {#if stage.params.length === 0}
                      <p class="no-params">nothing to set</p>
                    {:else}
                      {#each stage.params as param (param.name)}
                        <ParamControl
                          {param}
                          id={`${patch.name}-${stage.index}-${param.name}`}
                          editable={session.live}
                          onchange={(value) => setParam(patch, stage.index, param.name, value)}
                        />
                      {/each}
                    {/if}
                  </li>
                {/each}
              </ol>
            {:else}
              <p class="empty">
                {part.part} has no patch of its own, so it sounds through the built-in voice.
              </p>
            {/if}
          </section>
        {:else}
          <p class="empty">This piece has no parts yet.</p>
        {/if}
      </main>

      <Margin side="right" label="Signals">
        {#if studio && studio.signals.length > 0}
          <h2 class="aside-heading">Signals</h2>
          {#each studio.signals as signal (signal.name)}
            <section class="signal">
              <h3 class="stage-name">{signal.name}</h3>
              {#each signal.stages as stage (stage.index)}
                {#each stage.params as param (param.name)}
                  <ParamControl
                    {param}
                    id={`${signal.name}-${stage.index}-${param.name}`}
                    editable={session.live}
                    onchange={(value) => setParam(signal, stage.index, param.name, value)}
                  />
                {/each}
              {/each}
            </section>
          {/each}
        {/if}
      </Margin>
    </div>
  </div>
{:else}
  <Margin side="bottom"><p class="empty">Nothing open.</p></Margin>
{/if}

<style>
  .sound-workspace {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
    min-height: 0;
    max-width: 100%;
    overflow-x: hidden;
  }

  .body {
    display: grid;
    grid-template-columns: minmax(9rem, auto) minmax(0, 1fr) minmax(12rem, auto);
    min-height: 0;
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

  .parts {
    margin: 0;
    padding: 0;
    list-style: none;
    text-align: right;
  }

  .part {
    display: block;
    width: 100%;
    background: none;
    border: 0;
    padding: var(--s-1) 0;
    font: inherit;
    color: var(--ink-muted);
    text-align: right;
    cursor: pointer;
  }

  .part[aria-current="true"] {
    color: var(--ink);
  }

  .part-name {
    display: block;
    font-family: var(--f-ui);
    font-size: var(--t-name-size);
    line-height: var(--t-name-line);
  }

  .part-patch {
    display: block;
    font-family: var(--f-mono);
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    color: var(--ink-faint);
  }

  .stage {
    min-width: 0;
    min-height: 0;
    overflow: auto;
    padding: var(--s-6);
  }

  .patch {
    max-width: 34rem;
  }

  .assignment {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-4);
    padding-bottom: var(--s-3);
    border-bottom: 1px solid var(--rule);
  }

  .heading {
    margin: 0;
    font-family: var(--f-score-text);
    font-size: var(--t-large-size);
    line-height: var(--t-large-line);
    font-weight: 400;
    color: var(--ink);
  }

  .picker {
    display: flex;
    align-items: baseline;
    gap: var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  .picker select {
    background: none;
    border: 0;
    border-bottom: 1px solid var(--rule);
    padding: 0 0 2px;
    font-family: var(--f-mono);
    font-size: var(--t-small-size);
    color: var(--ink);
  }

  /*
   * The chain reads down the page in the order the `|>` runs it, and each
   * stage is separated by the same hairline every other division uses. No
   * cards: a patch is one thing, not a stack of panels.
   */
  .chain {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .stage-row {
    padding: var(--s-4) 0;
    border-bottom: 1px solid var(--rule);
  }

  .stage-name {
    margin: 0 0 var(--s-2);
    font-family: var(--f-mono);
    font-size: var(--t-name-size);
    line-height: var(--t-name-line);
    font-weight: 400;
    color: var(--ink);
  }

  .label {
    padding-left: var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-micro-size);
    letter-spacing: var(--tracking-micro);
    color: var(--ink-muted);
  }

  .no-params,
  .empty {
    margin: 0;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  .empty {
    max-width: 32rem;
  }

  .aside-heading {
    margin: 0;
    font-family: var(--f-ui);
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    letter-spacing: var(--tracking-micro);
    text-transform: uppercase;
    color: var(--ink-muted);
  }

  .signal {
    display: flex;
    flex-direction: column;
    gap: var(--s-1);
  }
</style>
