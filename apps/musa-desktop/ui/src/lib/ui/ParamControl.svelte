<script lang="ts">
  /**
   * One studio parameter: its name, its value, and a way to change it.
   *
   * A hairline track and a small mark — no bevel, no chrome, no wood
   * (`00-thesis.md` §5 rejects skeuomorphic studio hardware, and the same
   * typographic discipline that governs the score governs here).
   *
   * The value is committed on release, not while dragging. One gesture is one
   * edit and therefore one revision, which is what keeps a slider from filling
   * the undo history with a hundred intermediate positions — and what lets the
   * source stay the single authority without a second real-time channel
   *.
   *
   * A parameter the patch never wrote is drawn muted: it is the declared
   * default, not a choice, and moving the control is what makes it one.
   */
  import type { ParamFacts } from "../state/snapshot";

  let {
    param,
    id,
    editable = true,
    onchange,
  }: {
    param: ParamFacts;
    /**
     * Unique within the page. The caller knows which container and stage this
     * is and the control does not, and two oscillators in one patch both have
     * a `frequency` — so a label that pointed at `param-frequency` would point
     * at the wrong one.
     */
    id: string;
    editable?: boolean;
    onchange: (value: number) => void;
  } = $props();

  /** Where the handle is while the pointer holds it, before the core answers. */
  let dragging = $state<number | null>(null);
  const shown = $derived(dragging ?? param.value);

  const span = $derived(Math.max(param.maximum - param.minimum, Number.EPSILON));
  /**
   * A round step near a thousandth of the range: fine enough that a drag feels
   * continuous, and a power of ten so the value under the handle is a number a
   * composer would have typed. A cutoff moves in tens of hertz, a ratio in
   * thousandths.
   */
  const step = $derived(Math.pow(10, Math.floor(Math.log10(span / 500))));

  /**
   * The value as a composer would write it: enough places to distinguish two
   * positions of the control, and never an exponent.
   */
  function reads(value: number): string {
    const places = span >= 100 ? 0 : span >= 1 ? 2 : 4;
    return Number(value.toFixed(places)).toString();
  }
</script>

<div class="param" class:inherited={!param.written}>
  <label class="name" for={id}>{param.name}</label>
  <input
    {id}
    class="track"
    type="range"
    min={param.minimum}
    max={param.maximum}
    {step}
    value={shown}
    disabled={!editable}
    oninput={(event) => (dragging = event.currentTarget.valueAsNumber)}
    onchange={(event) => {
      const next = event.currentTarget.valueAsNumber;
      dragging = null;
      onchange(next);
    }}
  />
  <output class="value" for={id}
    >{reads(shown)}{#if param.unit}<span class="unit">{param.unit}</span>{/if}</output
  >
  <!--
    A modulated parameter's written value is what the signal moves around, so
    the row says so rather than showing a control that appears to disagree
    with what is heard (§13.7).
  -->
  {#if param.modulatedBy}
    <span class="modulated">modulated by {param.modulatedBy}</span>
  {/if}
</div>

<style>
  .param {
    display: grid;
    grid-template-columns: 7em minmax(0, 1fr) 6em;
    align-items: baseline;
    gap: var(--s-3);
    font-family: var(--f-ui);
    font-size: var(--t-small-size);
    line-height: var(--t-small-line);
  }

  .name {
    color: var(--ink-muted);
  }

  /* An unwritten value is the declaration speaking, not the composer. */
  .inherited .name,
  .inherited .value {
    color: var(--ink-faint);
  }

  .value {
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

  .modulated {
    grid-column: 2 / -1;
    font-size: var(--t-micro-size);
    line-height: var(--t-micro-line);
    letter-spacing: var(--tracking-micro);
    color: var(--ink-muted);
  }

  /* A rule with a mark on it. The track is the same hairline every other
     division in the interface is drawn with. */
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

  .track:disabled {
    cursor: default;
  }

  .track:disabled::-webkit-slider-thumb {
    background: var(--ink-faint);
  }

  .track:focus-visible::-webkit-slider-thumb {
    outline: 1px solid var(--ink);
    outline-offset: 2px;
  }
</style>
