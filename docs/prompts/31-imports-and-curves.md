---
id: 31
slug: imports-and-curves
status: pending
depends_on: [29, 30]
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
- Prompt 05's resolution pass, prompt 15's `IntegratedTempoMap` (designed piecewise for this), prompt 23's profiles.

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
    consistent with prompt 30's `at`), and dynamic hairpins (`cresc.` / `dim.` spans in a voice, rendered as hairpins in
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

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-render -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/album/pieces/01-opening.musa
cargo run -p musa-cli -- render examples/album/pieces/01-opening.musa --to wav -o /tmp/opening.wav
```

Commit as `Add relative imports and tempo/expression curves`.

## Stop

- No package registry, versioned dependencies, or remote imports (§16, rejected).
- No multi-piece album playback/export orchestration (each piece renders independently).
- No continuous tempo ramps (rit./accel. as interpolated segments) unless the piecewise structure makes it trivial — if
  so, keep it and snapshot it.
- No Phase 4 features (samples, plugins, MusicXML import).
