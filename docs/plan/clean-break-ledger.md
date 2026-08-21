# Clean-break ledger

**Status: directive.** Written by prompt 127a as part of the amendment that made the event track and the machine the two
core values (`../rules/constitution.md`, `../rules/README.md`).

This page names every source spelling, Rust API, serialized form, fixture, and test name that prompts 127b–127d, 142,
143, and 150–153 **delete** rather than keep working through an alias, a deprecation shim, or a compatibility reader. It
exists so that "clean break" is a checkable list rather than an intention.

Prompt 143's row is the one that arrived by a different route. It is not a rename the event-track amendment forced; it
is the **count that admitted the index**, entered here so that admitting a feature on a workaround obliges removing the
workaround (`../rules/obligations.md` §10). Seventeen of the compiler's 121 `Builtin` variants are one modulus spelled
into the compiler because the language could not say it, and a prompt that lands the index without deleting them has not
finished.

## Why a ledger rather than aliases

An alias is cheap to add and expensive to remove, because every later reader has to learn both names and decide which
one is current. Musa has one governing vocabulary at a time; two spellings of one concept is exactly the drift that
`../AGENTS.md` forbids between code and governing documents. A version header, a refused format version, and a stable
error message do the job an alias would have done — and they say *no* at the boundary instead of silently accepting an
old meaning.

So the rule these prompts work under is:

> **Rename, delete, and version-break in the prompt that owns the concept. Never ship both names.**

A reader of an old artifact gets a refusal that names the version and the prompt that broke it. It does not get a
best-effort translation.

## 1. Source syntax

| Deleted spelling | Replacement | Deleted by |
| --- | --- | --- |
| the type name `Music` | `EventTrack<WrittenTime>`, written out | 142 |
| the type name `ContextualMusic` and the contextual-instantiation judgment | ordinary values; placement by the enclosing voice's left fold | 142 |
| `music { … }` as a contextual-value constructor | an ordinary expression of event-track type | 142 |
| `overlay(a, b)` | `together(a, b)` | 127c |
| `sequence(a, b)` / the `;` sequencing spelling in events documents | `follow(a, b)` | 127c |
| `timeline d { … }` in events documents | `track d { … }` | 127c |
| the `extent` keyword and the `extent` spelling in diagnostics | `duration` | 127c |
| partial application, default parameters, and named hole filling | complete calls, and `fn (…) -> τ { e }` where a specialization has to be written down | 127ad |
| a public `lift` from a source function into a machine | registered primitives only | 127d |

`instantiate`, `close`, and `EventsFragment` are private compiler concepts rather than source syntax; they are listed in
§2 because that is where they are spelled.

## 2. Rust APIs

| Deleted item | Replacement | Deleted by |
| --- | --- | --- |
| `musa_events::Timeline<A>` | the event-track type at `EventTrack<C, A>` | 127c |
| `musa_events::sequence` | `follow` | 127c |
| `musa_events::overlay` | `together` | 127c |
| `Timeline::extent`, `Term::extent` | `duration` | 127c |
| `Timeline::map_payload`, `Term::map_payloads` | `map_payloads` | 127c |
| `Beat` as an untagged position type | `Position<C>`; a track's extent is the separate `Duration<C>` | 127c |
| `Length<C>` — both the name and its use for an instant as well as an amount | `Position<C>` for *when*, `Duration<C>` for *how much* | 127c |
| `Timeline::length`, `Term::length`, `Measure::length`, `check_bar_length`, `ly.rs::measure_length`, `musicxml.rs::measure_length` | `duration` / `check_bar_duration` / `measure_duration` | 127c |
| `SecondTime` as a coordinate tag | `PhysicalTime` | 127c |
| `PrimitiveOwnership<Builtin>` and `primitive` naming a compiler-owned operation | `BuiltinOwnership<Builtin>`; compiler-owned operations are *builtins*, registered units are *primitives* | 127b, discharged by 127ca |
| `Scheduled<A>` | `Schedule<A>`, matching the existing `ScheduleError` | 151 |
| `musa_compiler::phase::Music`, `MusicOperation`, `MusicRole` | ordinary values of ordinary types | 142 |
| the private `close` and `instantiate_music` elaboration path, and the fragment type they close | building and closing over ordinary values | 142 |
| `musa_dsp::compile_graph` and public `StudioGraphSpec` as a semantic alternative | machine construction and `prepare_audio(format, machine)` | 152 |
| `RenderPlan` / `PreparedExecution` as the public prepared artifact | `PreparedMachine` | 150, 152 |
| a caller- or preparation-chosen "semantic step" option | one audio step is one sample frame; batching is a checked `batch(n)` contract | 152 |
| the seventeen `Builtin` variants `Pc12Of`, `Pc12Number`, `Pc12Forget`, `Pc12Transposed`, `Pc12Inverted`, `Pc12Spelled`, `Row12Of`, `Row12Pcs`, `Row12Head`, `Row12Transposed`, `Row12Inverted`, `Row12Retrograde`, `Row12Matrix`, `Row12Forms`, `Row12Symmetries`, `Row12Repeats`, `Row12Missing`, and the surface spellings `pc12_*` / `row12_*` they register | operations over `Pc(n)` and `Row(n)`, written in `.musa` against the index of `../rules/language/02-core-calculus.md` §1.5 | 143 |

The event track stays a leaf through all of this: no machine type, audio type, or frame index enters `musa-events`.

## 3. Serialized and interchange forms

| Deleted form | Behaviour after the break | Deleted by |
| --- | --- | --- |
| the `% musa-events-1` document header | refused with an error naming the version and this ledger; **not** upgraded | 127c |
| event-track encoding versions 1 and 2 (`../rules/events/05-normalization.md` N6, `12-payload-admission.md` A7) | refused; version 3 adds the coordinate tag and is the only accepted one | 127c |
| unframed `Display`-derived digests | already invalid; they remain invalid and are not read as track identity | (already broken) |
| any prepared-plan cache record keyed without the coordinate tag or the operation version | refused as a version error rather than treated as a miss | 127c, 152 |

A version header exists precisely so this can be a refusal rather than a guess. A reader that cannot reproduce a
document's version says so.

## 4. Fixtures

| Deleted or rewritten fixture set | Deleted by |
| --- | --- |
| all 24 files in `examples/events/*.musa.events` — regenerated at `% musa-events-3` with `track`, `follow`, `together`, and durations | 127c |
| every `.musa` example and stdlib source that spells the type `Music` or calls `overlay`/`sequence` | 142 |
| the insta snapshots under `crates/musa-events` and `crates/musa-compiler` that pin the old events text | 127c, 142 |
| studio fixtures whose expected output depends on host-block-defined feedback or modulation | 152 |

Goldens are rewritten in the same prompt that breaks them, never left failing across a prompt boundary
(`prompts/README.md`).

## 5. Test names

The event-track law suite is renamed with the operations it tests. The old names must not survive as aliases, because a
test named for a deleted operation is how a deleted operation comes back.

| Old test name | New name (`../rules/events/04-algebraic-laws.md`) |
| --- | --- |
| `seq_associativity` | `follow_associativity` |
| `seq_zero_identity` | `follow_zero_identity` |
| `sequence_translates_and_adds_extents` | `follow_duration_additivity` |
| `overlay_associativity` | `together_associativity` |
| `overlay_commutativity` | `together_commutativity` |
| `overlay_fixed_duration_identity` | `together_fixed_duration_identity` |
| `overlay_takes_max_extent_and_keeps_multiplicity` | folded into `together_fixed_duration_identity` and `together_not_idempotent` |
| `map_preserves_sequence` | `map_preserves_follow` |
| `map_preserves_overlay` | `map_preserves_together` |
| `scale_preserves_sequence` | `scale_preserves_follow` |
| `scale_preserves_overlay` | `scale_preserves_together` |
| `overlay_not_idempotent` | `together_not_idempotent` |
| `sequence_does_not_distribute_over_overlay` | `follow_does_not_distribute_over_together` |

Renamed by prompt 127c, except where the law itself moves.

## 6. What is *not* on this ledger

These survive the break unchanged, and a prompt that removes one is wrong:

- source authority — the `.musa` text is the master record;
- named conversions between representations, and the records they leave;
- exact rational time, and versioned exact identity;
- optional, plural musical theories;
- unequal-duration `together`, which takes the longer duration and inserts no rests;
- the real-time rules: the audio callback allocates, locks, does I/O, logs, and destroys nothing;
- `scale`, `restrict`, `covering`, `prevailing`, and normalization/encoding, which are beyond the six-operation basis
  but have real callers (`../rules/events/00-purpose.md`).

## 7. How a prompt discharges its rows

A prompt in 127b–127d, 142, and 150–153 is not done while a row it owns still resolves in the workspace. The check is
mechanical: after the prompt's commit, searching the workspace for the deleted spelling returns only this ledger and the
research record. If it returns code, a fixture, or a governing document, the break is incomplete.

Prompt 153 audits the whole ledger as one of its conformance rows.
