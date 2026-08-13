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

Use two exact rational types tagged by a nominal coordinate `C`: `Position<C>` for *when* something happens and
`Duration<C>` for *how much time* it takes. They are the two structures `docs/rules/kernel/00-purpose.md` already names
— the abelian group `(ℚ, +, 0)` and the ordered monoid `(ℚ≥0, +, 0)` — and giving them one Rust type would let beat 3
and three beats be added. A position plus a duration is a position; two durations add; the difference of two positions
is a duration only when nonnegative, so it returns `Result`. Initial coordinate types are `WrittenTime`,
`PerformedTime`, and `PhysicalTime`; audio frame indices remain bounded runtime integers, not track positions. Meter,
part, voice, pitch, tuning, and scale are payload data, not type indices.

An event track contains a nonnegative duration and a finite multiset of `(start,end,payload)` within it. Positive spans
are half open; points have equal start and end. The basis is `empty`, `event`, `follow`, `together`, `map_payloads`, and
`duration`. Beyond the basis, retain exactly what has real callers: `scale`, `restrict`, `covering`, `prevailing`, the
existing observations, and the exact normalization/encoding (`docs/rules/kernel/00-purpose.md`). Expose nothing else.
`together` uses the longer duration, keeps multiplicity, and inserts no rests.

Rename rather than alias: remove `Timeline`, `extent`, `sequence`, `overlay`, `map_payload`, and their old interchange
spellings, and rename the law tests with them. `docs/plan/clean-break-ledger.md` §§2–5 is the checkable list this prompt
discharges; the `% musa-kernel-1` header and encoding versions 1 and 2 become refusals, not migrations. Update the
kernel text format and all goldens in the same prompt with an explicit format-version break. Hashes remain lookup
helpers; exact framed bytes decide equality.

Keep `musa-kernel` a leaf and keep payloads musically opaque. Do not place machine execution or audio types in it.

## Target

- Typed `EventTrack<C,A>` implementation, operations, observation, exact encoding, and source/kernel term forms.
- Repository-wide clean rename and regenerated fixtures with no compatibility alias or dual serialization.
- Property tests for bounds, unequal-duration `together`, multiplicity, mapping, half-open spans, coordinate mismatch,
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
