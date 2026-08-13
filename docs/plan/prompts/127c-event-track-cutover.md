---
id: 127c
slug: event-track-cutover
status: pending
depends_on: [127b]
phase: 3
---

# Replace Timelines with Typed Event Tracks

## Task

Make `EventTrack<C,A>` the one finite temporal value throughout Musa. Rename the public Rust API, kernel interchange,
compiler stages, tests, and documentation in one clean break. Add a coordinate tag so written, performed, and physical
positions cannot be mixed by accident.

## Read

- The prompt-127a kernel specification and research `05-selected-calculus.md` §§4 and 11.
- `crates/musa-kernel`, all `Timeline`, `Beat`, `extent`, `sequence`, and `overlay` callers, kernel goldens, renderers,
  compiler facts, and project IPC.
- The current law suites and every serialized `.musa.kernel` fixture.

## Design

Use exact rational `Length<C>` and positions tagged by a nominal coordinate `C`. Initial coordinate types are
`WrittenTime`, `PerformedTime`, and `SecondTime`; audio frame indices remain bounded runtime integers, not track
positions. Meter, part, voice, pitch, tuning, and scale are payload data, not type indices.

An event track contains a nonnegative length and a finite multiset of `(start,end,payload)` within it. Positive spans
are half open; points have equal start and end. Expose only `empty`, `event`, `follow`, `together`, `map_events`,
`length`, the existing observations, and exact normalization/encoding needed by real callers. `together` uses the longer
length, keeps multiplicity, and inserts no rests.

Rename rather than alias: remove `Timeline`, `extent`, `sequence`, `overlay`, and their old interchange spellings.
Update the kernel text format and all goldens in the same prompt with an explicit format-version break. Hashes remain
lookup helpers; exact framed bytes decide equality.

Keep `musa-kernel` a leaf and keep payloads musically opaque. Do not place machine execution or audio types in it.

## Target

- Typed `EventTrack<C,A>` implementation, operations, observation, exact encoding, and source/kernel term forms.
- Repository-wide clean rename and regenerated fixtures with no compatibility alias or dual serialization.
- Property tests for bounds, unequal-length `together`, multiplicity, mapping, half-open spans, coordinate mismatch,
  exact round trip, and every retained law/non-law.
- Updated renderer, compiler, project, CLI, and web callers.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
! rg -n '\bTimeline\b|\.extent[(]|\bsequence[(]|\boverlay[(]' crates examples stdlib docs/rules docs/book
```

Commit as `Replace timelines with typed event tracks`.

## Stop

- No compatibility alias, old interchange reader, automatic migration layer, meter index, musical payload type, machine
  form, audio frame, or float time in `musa-kernel`.
- No change to note spelling or musician-facing score syntax beyond names that directly expose the renamed core.
