<script lang="ts">
  /**
   * Settings (prompt 59): the three decisions the application makes about
   * itself, in one place.
   *
   * Every row is a *set of choices* rather than a switch, because a control
   * named "Switch theme" does not say what it will switch to, and a checkbox
   * called "Vim mode" does not say what it is now. Four rungs of text size
   * and three states of theme are all real, so all of them are shown and the
   * one in force is marked.
   *
   * Nothing here is the document's. Tempo, key, and title are the piece's and
   * are edited where they are printed (prompt 54).
   */
  import type { Preferences, TextSize } from "../lib/session/preferences.svelte";
  import { TEXT_SIZES } from "../lib/session/preferences.svelte";
  import type { Theme, ThemeChoice } from "../lib/session/theme.svelte";
  import Leaf from "../lib/ui/Leaf.svelte";

  let {
    preferences,
    theme,
    onclose,
  }: {
    preferences: Preferences;
    theme: ThemeChoice;
    onclose: () => void;
  } = $props();

  /** `null` is the third state the class always had: follow the system. */
  const THEMES: { value: Theme | null; label: string }[] = [
    { value: null, label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ];

  const SIZES: Record<TextSize, string> = {
    small: "Small",
    normal: "Normal",
    large: "Large",
    larger: "Larger",
  };
</script>

<div
  class="scrim"
  role="presentation"
  onclick={(event) => event.target === event.currentTarget && onclose()}
>
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Settings">
    <Leaf>
      <div class="body">
        <h2 class="title">Settings</h2>

        <div class="row">
          <span class="label" id="settings-theme">Theme</span>
          <div class="choices" role="group" aria-labelledby="settings-theme">
            {#each THEMES as option (option.label)}
              <button
                type="button"
                aria-pressed={theme.chosen === option.value}
                onclick={() => theme.choose(option.value)}>{option.label}</button
              >
            {/each}
          </div>
        </div>

        <div class="row">
          <span class="label" id="settings-text">Text size</span>
          <div class="choices" role="group" aria-labelledby="settings-text">
            {#each TEXT_SIZES as size (size)}
              <button
                type="button"
                aria-pressed={preferences.textSize === size}
                onclick={() => preferences.chooseText(size)}>{SIZES[size]}</button
              >
            {/each}
          </div>
        </div>

        <div class="row">
          <span class="label" id="settings-vim">Vim mode</span>
          <div class="choices" role="group" aria-labelledby="settings-vim">
            <button
              type="button"
              aria-pressed={!preferences.vim}
              onclick={() => preferences.setVim(false)}>Off</button
            >
            <button
              type="button"
              aria-pressed={preferences.vim}
              onclick={() => preferences.setVim(true)}>On</button
            >
          </div>
        </div>

        <p class="note">
          These are the application's, not the piece's — they follow you between scores and never
          appear in the file.
        </p>

        <button type="button" class="close" onclick={onclose}>Close</button>
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
    width: min(520px, 92vw);
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

  /* The inspector's shape: a name at the left, its value at the right,
     ruled by the row above rather than boxed (`01-visual-language.md` §7). */
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

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-3);
  }

  /* The hairline is the whole affordance: at rest a word, underlined once it
     is the one in force. No boxes, no fill, no pill. */
  .choices button {
    background: none;
    border: 0;
    padding: 0;
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
    color: var(--ink-muted);
    border-bottom: 1px solid transparent;
    cursor: pointer;
  }

  .choices button:hover {
    color: var(--ink);
    border-bottom-color: var(--ink-muted);
  }

  .choices button[aria-pressed="true"] {
    color: var(--ink);
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
