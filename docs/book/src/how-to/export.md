# Export scores and audio

`musa render` turns a piece into a file another program can read:

```bash
musa render first.musa --to lilypond -o first.ly
musa render first.musa --to mei -o first.mei
musa render first.musa --to musicxml -o first.musicxml
musa render first.musa --to midi -o first.mid
musa render first.musa --to wav -o first.wav
```

The targets:

| Target | What you get |
| --- | --- |
| `mei` | Verovio-compatible MEI, with stable `xml:id`s that trace each note back to its source |
| `lilypond` | LilyPond source for engraving |
| `musicxml` | MusicXML for notation editors |
| `midi` | A MIDI file; `--mode score` (default) or `--mode performance` |
| `wav` | An offline render through the studio graph |

Text formats accept `-o -` for stdout. Binary formats (`midi`, `wav`) require `-o <path>`.

Two debug targets print to stdout instead of writing a file:

```bash
musa render first.musa --to plan          # the backend-neutral notation plan
musa render first.musa --to performance   # the realized performance events
```

As with `check`, `--seed <n>` selects which performance reading to compile when the piece has more than one.

Every target renders the same piece. The exporter reads a shared semantic snapshot; there is no per-backend rewrite of
the music, and no raw escape into backend-specific syntax.
