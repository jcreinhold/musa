# Run the desktop app

The desktop app is a Tauri shell over a Svelte UI, and both are thin: Rust owns the semantics, the frontend owns
ephemeral state only.

```bash
make desktop    # the score editor, with hot-reloading UI
make ui         # the UI alone in a browser, against a stubbed backend
make stop       # stop either one, however it was started
```

`make release` bundles the app for distribution. The app runs fully offline: fonts, the Verovio engraver, and the DSP
are bundled.

## What you are looking at

The score is the main surface — a genuinely engraved page, not a sketch. The text is one keystroke away and never in the
way. Editing the text recompiles the piece; invalid source keeps the last valid score on the page rather than blanking
it.

The app's signature is **Origin view**. Every note is either authored — you typed it — or generated — the compiler
produced it by expanding a motif or transform. Hold the Origin key and the page separates: authored music stays full
ink, generated music falls back, and hovering a generated note traces it back to the occurrence that produced it, on the
page and in the source.

The design behind all of this is explained under [The desktop interface](../concepts/interface.md).
