# Layer separation

Musa keeps separate things separate. The design roadmap fixes a table of distinctions, and the architecture enforces
it:

| This | is not | that |
| --- | --- | --- |
| Written pitch | ≠ | MIDI number |
| Notated duration | ≠ | Performed duration |
| Voice | ≠ | Mixer track |
| Part | ≠ | Synthesizer |
| Dynamic marking | ≠ | Decibels |
| Motif definition | ≠ | Its expansions |
| Source | ≠ | Widget state |

Each row names a conflation that real notation software makes and that musa refuses.

- **Written pitch ≠ MIDI number.** A written F♯ and a written G♭ are different spellings with different meanings in
  context; a MIDI number erases the difference. Spelling is preserved until a performance realization chooses pitches
  to sound.
- **Notated duration ≠ performed duration.** A staccato quarter is written as a quarter and played short. The page
  keeps the written value; the performance applies the profile's reading.
- **Voice ≠ mixer track, part ≠ synthesizer.** A voice is a line of music; a mixer track is a signal path. A part is
  assigned to an instrument in the studio, and the assignment can change without touching the notes.
- **Dynamic marking ≠ decibels.** A written *p* is an instruction to a player, and a performance profile says what it
  means for a given instrument. The same marking can read differently on strings and mallets.
- **Motif definition ≠ its expansions.** A motif is defined once and used at many places. Editing the definition
  changes every use; the uses are not copies. This row is what makes [Origin view](provenance.md) possible.
- **Source ≠ widget state.** The `.musa` text is the only document. The interface renders it and edits it; it never
  keeps a musical model of its own.

The table is a diagnostic tool. When a feature seems to need one of these pairs merged — a part that *is* its patch, a
dynamic that *is* a gain — the design says the feature is being specified at the wrong layer.
