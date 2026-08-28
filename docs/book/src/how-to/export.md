# Export scores and audio

`musa render` turns a piece into a file another program can read:

```bash
musa render first.musa --to lilypond -o first.ly
musa render first.musa --to mei -o first.mei
musa render first.musa --to musicxml -o first.musicxml
musa render first.musa --to midi -o first.mid
musa render first.musa --to wav -o first.wav
musa render first.musa --to daw --profile logic -o "first for a workstation"
```

The targets:

| Target | What you get |
| --- | --- |
| `mei` | Verovio-compatible MEI, with stable `xml:id`s that trace each note back to its source |
| `lilypond` | LilyPond source for engraving |
| `musicxml` | MusicXML for notation editors |
| `midi` | A MIDI file; `--mode score` (default) or `--mode performance` |
| `wav` | An offline render through the studio graph |
| `daw` | A whole folder for a workstation — see [the bundle how-to](daw-bundle.md) |

Text formats accept `-o -` for stdout. Binary formats (`midi`, `wav`) require `-o <path>`.

Two debug targets print to stdout instead of writing a file:

```bash
musa render first.musa --to plan          # the backend-neutral notation plan
musa render first.musa --to performance   # the realized performance events
```

As with `check`, `--seed <n>` selects which performance reading to compile when the piece has more than one.

Every target renders the same piece. The exporter reads a shared semantic snapshot; there is no per-backend rewrite of
the music, and no raw escape into backend-specific syntax.

## Handing a piece to a workstation

Each of these files can be imported into a digital audio workstation. Logic Pro documents importing Standard MIDI Files
and MusicXML; GarageBand documents importing MIDI and audio. Export the ones your program reads and import them there —
or export `--to daw`, which packages all of them at once with a manifest saying what each is and what it lost. That is
[its own how-to](daw-bundle.md).

What crosses is a *presentation* of the piece, and each format loses something specific. A MIDI file carries note
numbers, not written pitch: spelling, voices, ties, beams, and notated durations do not survive it. A WAV carries one
mix at one moment. MusicXML carries notation but not the studio.

What does not exist, and is not planned:

- Musa does not read a workstation's session document, and it does not write `.logicx` or `.band`.
- There is no round trip. Nothing you do in a workstation comes back into `.musa`, because no reverse conversion could
  be exact — the source stays the master record.
- Musa does not host Audio Unit, CLAP, or VST plug-ins. Its studio is the room of the work, not a plug-in rack.

The bundle above is the first of the deeper integrations. Live MIDI to a running host and a Musa Audio Unit that Logic
or GarageBand can load are specified but not yet built. The contract they must meet is
[`docs/rules/across-stages/06-daw-boundary.md`](../../../rules/across-stages/06-daw-boundary.md) in the repository, and
[the implementation map](../../../plan/code-map/spec-to-implementation-map.md) says how much of it exists. Today the
answer is: the files above, and the bundle that packages them.
