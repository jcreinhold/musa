---
id: 36
slug: imports-and-curves
status: done
depends_on: [34, 35]
phase: 3
---

# Relative Imports and Tempo/Expression Curves

## Task

Finish Phase 3 with the two features that make larger works practical: declarative relative imports
(`use "../library/patches.musa";`) for shared motif/patch libraries, and tempo/expression curves (tempo changes and
dynamic curves over time) flowing from source through the integrated tempo map to performance and playback.

## Read

- Roadmap §16 (imports: declarative, acyclic, local, side-effect-free; a piece remains independently compilable; no
  package registry, no dependency solver), §18 Phase 3 (tempo and expression curves), §6.4 (tempo map integration),
  §10.6 (pipeline).
- Prompt 05's resolution pass, prompt 15's `IntegratedTempoMap` (designed piecewise for this), prompt 28's profiles.

## Design

- Imports:
  - `use "relative/path.musa";` at piece top level; paths resolve relative to the importing file; only declarations
    (`motif`, `patch`, `profile`, library-level items) are importable — a `score` in an imported file is ignored with a
    warning or rejected (pick reject-with-diagnostic: simplest honest rule; document).
  - Acyclicity: build the import graph during resolution; cycles are diagnostics listing the cycle files. No diamond
    deduplication subtleties: import the same file twice → resolved once, shared.
  - Names enter the importing piece's namespace prefixed or flat? Flat with collision diagnostics is the simpler honest
    start (document; namespacing is a later decision if collisions annoy).
  - `ProjectSession::open` gains multi-file awareness: the session tracks the import closure for diagnostics/recompile
    (a library edit recompiles dependents — with one open piece this is just recompile-on-save of the library file; do
    not build a multi-document editor).
  - Directory projects (§16's `musa.toml` + `pieces/` + `library/`) get their minimal form: if a `musa.toml` exists
    beside the piece, it is read for shared metadata only; the piece still compiles standalone without it.
- Curves:
  - Language: `tempo q = 72;` may now appear multiple times with positions (`at 9:1 tempo q = 96;` — choose syntax
    consistent with prompt 35's `at`), and dynamic hairpins (`cresc.` / `dim.` spans in a voice, rendered as hairpins in
    notation).
  - Compiler: `TempoMap` becomes piecewise (prompt 15's structure pays off); `IntegratedTempoMap` integrates segments
    exactly (rationals until the frame boundary, then exact conversion). Dynamics curves interpolate the profile's
    amplitude levels across hairpin spans in `lower_performance`.
  - Notation: tempo marks and hairpins in MEI/LilyPond/MusicXML.
  - Playback: tempo changes are honored by scheduling (they already are, once the map is piecewise — the test proves
    it).
- Fixture: `examples/album/` mini directory project — a `library/patches.musa` + `library/motifs.musa` imported by one
  piece with a tempo change and one hairpin.

## Target

- `musa-language`/`musa-compiler`: import syntax, resolution, cycle diagnostics; piecewise tempo + hairpin dynamics.
- `musa-render`: tempo/hairpin notation in all three backends.
- `musa-project`: import-closure tracking, minimal `musa.toml` reading.
- Tests: import resolution (shared, cycles, missing files, score-in-library rejection); exact tempo-integration tests
  (frame boundaries across a tempo change); hairpin interpolation endpoints; backend snapshots; end-to-end WAV with an
  audible tempo change.

## Repairs made while implementing

- **A library is its own file root, not a piece with restrictions.** `library { ... }` is a second root production, so
  "a score in an imported file" is a parse error rather than a rule the compiler enforces after the fact. The prompt
  offered a choice between ignoring-with-warning and rejecting; the grammar makes rejecting free.
- **The compiler reads no files.** `CompileOptions::imports` carries an `ImportSources` map (resolved path → text) and
  `resolve_import` joins paths lexically, so compilation stays a pure function of its inputs and every import test runs
  without a filesystem. `musa-project` owns the I/O: it walks the closure on every recompile, which is what
  "recompile-on-save of the library" means with one document open.
- **Names are flat and collisions are errors**, as the prompt proposed. `register_motifs`/`merge_profiles` take the
  library a declaration came from so the message names the file rather than saying "duplicate motif" twice.
- **A library's `studio` declares patches and signals only.** A library does not know what parts a score has, so
  `assign`/`route`/`send`/`bus`/`modulate` inside one are reported rather than applied. This is why `lower_studio` now
  runs even when the piece writes no `studio` block.
- **Tempo changes are written where form markers are**: `tempo 1/4 = 108 at 3:1;`, resolved against the meter after the
  parts exist, so a tempo nobody reaches is an error. A second unpositioned `tempo` (two answers to "how fast does this
  start") and a change at `1:1` are both refused.
- **`IntegratedTempoMap` accumulates segment offsets as exact rationals** and rounds once, at the position asked about.
  Rounding each segment's start would drift at every change;
  `the_frame_at_a_tempo_change_is_the_sum_of_what_came_before` is the test that pins it.
- **MIDI export became piecewise too.** Frames → ticks with one factor would misplace every note after a change, so
  `IntegratedTempoMap::segments()` is exported and `midi.rs` writes one tempo meta-event per segment.
- **Hairpins interpolate over notes, not over frames.** `crescendo to f { ... }` gives each event under it an equal
  share of the distance from the prevailing amplitude to the target's, and the last one arrives exactly at the mark —
  which then stays in force. Interpolating over time would make the arrival depend on the rhythm.
- **Every export now carries the piece's starting tempo**, which none of them did before: the plan's `tempos()` begins
  with it and each backend writes it (`<tempo mm>`, `\tempo 4 = 72`, `<metronome>` + `<sound tempo>`). The example
  snapshots were regenerated for that reason.
- **The formatter learned two one-word constructs.** `3:1` and `fmaj7` were being written `3: 1` and `fmaj 7` — a
  prompt-35 gap that the album fixture surfaced, since `tempo ... at 3:1;` is the first positioned statement in a
  canonical example.
- **`musa.toml` is metadata and nothing else.** `ProjectSession::project()` finds it by walking up from the piece and
  reads a name and a composer; the piece compiles identically with or without it, which the test asserts by compiling
  the same source with no project around it.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-render -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/album/pieces/01-opening.musa
cargo run -p musa -- render examples/album/pieces/01-opening.musa --to wav -o /tmp/opening.wav
```

Commit as `Add relative imports and tempo/expression curves`.

## Stop

- No package registry, versioned dependencies, or remote imports (§16, rejected).
- No multi-piece album playback/export orchestration (each piece renders independently).
- No continuous tempo ramps (rit./accel. as interpolated segments) unless the piecewise structure makes it trivial — if
  so, keep it and snapshot it.
- No Phase 4 features (samples, plugins, MusicXML import).
