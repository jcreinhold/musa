<script lang="ts">
  /**
   * What a screen reader hears (`03-interaction.md` §5).
   *
   * Two regions, because the two kinds of news are not equally urgent: a
   * selection change is polite and waits for a pause, transport starting or
   * stopping is assertive and interrupts. Position is announced never — a
   * playhead that spoke every frame would make the app unusable with a screen
   * reader, which is the opposite of accessible.
   */
  let { selection = "", transport = "" }: { selection?: string; transport?: string } = $props();
</script>

<div class="announcer">
  <p aria-live="polite" aria-atomic="true">{selection}</p>
  <p aria-live="assertive" aria-atomic="true">{transport}</p>
</div>

<style>
  /*
   * Present to assistive technology, absent to everyone else. `display: none`
   * would remove it from the accessibility tree as well, which is the usual
   * way live regions end up announcing nothing.
   */
  .announcer {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .announcer p {
    margin: 0;
  }
</style>
