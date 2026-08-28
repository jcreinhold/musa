# Run the desktop app

The desktop app is a Tauri shell over a Svelte UI, and both are thin: checked Musa source owns declarable semantics,
Rust checks and executes it, and the frontend owns ephemeral state only.

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

A connected MIDI keyboard auditions the selected part's instrument and writes nothing. Turning a phrase you played into
notation is a separate, visible decision — [Your first captured phrase](../tutorials/first-captured-phrase.md) walks
through it, and [Capture and transcription](../reference/capture-and-transcription.md) is the reference.

## Getting a piece to a workstation

**File ▸ Export ▸ Export for a workstation** writes a whole bundle: both MIDI readings, the notation, the rendered mix,
one aligned stem per part and return, and a manifest saying what each file is and what the formats could not carry.
Choose the workstation, then the folder; the sheet that follows lists everything written and everything lost. The
[bundle how-to](daw-bundle.md) and [its reference](../reference/daw-bundle.md) cover what is in it.

The other two crossings are command-line: [live MIDI](live-midi.md) plays the piece to a workstation as it happens, and
the [Audio Units](audio-unit.md) put Musa inside one. All three are readings of the same source, and none of them writes
back to it.

The design behind all of this is explained under [The desktop interface](../concepts/interface.md).
