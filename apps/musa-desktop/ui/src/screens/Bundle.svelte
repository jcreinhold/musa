<script lang="ts">
  /**
   * Export for a workstation: which one, then what was written.
   *
   * Two states in one sheet, because they are two halves of one question. It
   * opens on the choice; once the project answers, the same sheet becomes the
   * report — every file the bundle holds and everything the target formats
   * could not carry.
   *
   * Nothing here is computed. The file list, the digests, and the losses are
   * the project's manifest, arranged (`../lib/state/bundle`); the interface
   * does not rebuild one in TypeScript.
   */
  import { PROFILES, byKind, groups, size, summary } from "../lib/state/bundle";
  import type { DawProfileDto } from "../lib/session/generated/DawProfileDto";
  import type { DawReportDto } from "../lib/session/generated/DawReportDto";
  import Leaf from "../lib/ui/Leaf.svelte";

  let {
    report = null,
    working = false,
    onexport,
    onclose,
  }: {
    /** What the last export wrote, or null before one has been asked for. */
    report?: DawReportDto | null;
    /** Whether one is being written now. */
    working?: boolean;
    onexport: (profile: DawProfileDto) => void;
    onclose: () => void;
  } = $props();
</script>

<div
  class="scrim"
  role="presentation"
  onclick={(event) => event.target === event.currentTarget && !working && onclose()}
>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Export for a workstation">
    <Leaf>
      <div class="body">
        <h2 class="title">Export for a workstation</h2>

        {#if report}
          <p class="said">{summary(report)}</p>
          <p class="where">{report.destination}</p>

          {#each groups(report) as group (group.title)}
            <h3 class="heading">{group.title}</h3>
            <ul class="files">
              {#each group.files as file (file.path)}
                <li><span class="path">{file.path}</span><span class="size">{size(file.bytes)}</span></li>
              {/each}
            </ul>
          {/each}

          {#if report.losses.length > 0}
            <h3 class="heading">What the formats cannot carry</h3>
            <ul class="losses">
              {#each byKind(report) as loss (loss.kind)}
                <li>
                  <span class="kind">{loss.kind}</span>
                  {#each loss.messages as message (message)}
                    <span class="said">{message}</span>
                  {/each}
                </li>
              {/each}
            </ul>
          {/if}

          <p class="note">
            How to bring this into Logic Pro or GarageBand is written up in the guide, under
            <em>Import a Musa bundle into Logic Pro or GarageBand</em>. The manifest in the folder says the same things
            this report does, and outlives it.
          </p>
          <button type="button" class="close" onclick={onclose}>Close</button>
        {:else if working}
          <p class="said" role="status">
            Packaging the piece. The audio is rendered once and the folder is installed whole, so there is nothing
            half-written to stop.
          </p>
        {:else}
          <div class="row">
            <span class="label" id="bundle-profile">Package for</span>
          </div>
          <ul class="choices" aria-labelledby="bundle-profile">
            {#each PROFILES as profile (profile.value)}
              <li>
                <button type="button" onclick={() => onexport(profile.value)}>{profile.label}</button>
                <span class="said">{profile.note}</span>
              </li>
            {/each}
          </ul>
          <p class="note">
            The music is the same either way. The profile chooses which files the folder holds and what the guidance in
            it says — it never changes what the piece sounds like.
          </p>
          <button type="button" class="close" onclick={onclose}>Cancel</button>
        {/if}
      </div>
    </Leaf>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: center;
    background: var(--surround);
    opacity: 0.98;
    z-index: 10;
  }

  .sheet {
    width: min(560px, 92vw);
    max-height: 84vh;
    overflow: auto;
  }

  .body {
    padding: var(--s-6);
  }

  .title {
    margin: 0 0 var(--s-4);
    font-family: var(--f-score-text);
    font-size: var(--t-title-size);
    line-height: var(--t-title-line);
    font-weight: 400;
    color: var(--ink);
  }

  .heading {
    margin: var(--s-4) 0 var(--s-1);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    font-weight: 400;
    color: var(--ink-muted);
  }

  .row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-4);
    padding: var(--s-2) 0;
    border-top: 1px solid var(--rule);
  }

  .label {
    font-family: var(--f-ui);
    font-size: var(--t-body-size);
    line-height: var(--t-body-line);
    color: var(--ink);
  }

  .choices,
  .files,
  .losses {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .choices li {
    display: flex;
    flex-direction: column;
    gap: var(--s-1);
    padding: var(--s-2) 0;
    border-top: 1px solid var(--rule);
  }

  .losses li {
    display: flex;
    flex-direction: column;
    gap: var(--s-1);
    padding: var(--s-2) 0;
    border-top: 1px solid var(--rule);
  }

  /* A file list is a table of two columns ruled by the row above, which is
     the inspector's shape (`01-visual-language.md` §7). */
  .files li {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-4);
    padding: var(--s-1) 0;
    border-top: 1px solid var(--rule);
  }

  .path {
    font-family: var(--f-mono, var(--f-ui));
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink);
  }

  .size,
  .said,
  .where {
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  .where {
    margin: 0 0 var(--s-2);
    word-break: break-all;
  }

  .kind {
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink);
  }

  /* The hairline is the whole affordance, as everywhere else in the
     interface: a word, underlined when it is pointed at. */
  .choices button {
    align-self: flex-start;
    background: none;
    border: 0;
    padding: 0;
    font-family: var(--f-ui);
    font-size: var(--t-body-size);
    line-height: var(--t-body-line);
    color: var(--ink);
    border-bottom: 1px solid var(--rule);
    cursor: pointer;
  }

  .choices button:hover {
    border-bottom-color: var(--plate);
  }

  .note {
    margin: var(--s-4) 0 0;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
  }

  .close {
    margin-top: var(--s-4);
    background: none;
    border: 0;
    padding: var(--s-1) var(--s-2);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    color: var(--ink-muted);
    cursor: pointer;
  }

  .close:hover {
    color: var(--ink);
  }
</style>
