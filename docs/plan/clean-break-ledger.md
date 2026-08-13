# Clean-break ledger

**Status: directive.** Written by prompt 127a as part of the amendment that made the event track and the machine the two
core values (`../rules/constitution.md`, `../rules/README.md`).

This page names every source spelling, Rust API, serialized form, fixture, and test name that prompts 127b–127i
**delete** rather than keep working through an alias, a deprecation shim, or a compatibility reader. It exists so that
"clean break" is a checkable list rather than an intention.

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
| the type name `Music` | `EventTrack[WrittenTime, ScoreFact]`, written out | 127e |
| the type name `ContextualMusic` and the contextual-instantiation judgment | ordinary values; placement by the enclosing voice's left fold | 127e |
| `music { … }` as a contextual-value constructor | an ordinary expression of event-track type | 127e |
| `overlay(a, b)` | `together(a, b)` | 127c |
| `sequence(a, b)` / the `;` sequencing spelling in kernel documents | `follow(a, b)` | 127c |
| `timeline d { … }` in kernel documents | `track d { … }` | 127c |
| the `extent` keyword and the `extent` spelling in diagnostics | `length` | 127c |
| partial application, default parameters, and named hole filling | complete calls | 127b |
| a public `lift` from a source function into a machine | registered primitives only | 127d |

`instantiate`, `close`, and `KernelFragment` are private compiler concepts rather than source syntax; they are listed in
§2 because that is where they are spelled.

## 2. Rust APIs

| Deleted item | Replacement | Deleted by |
| --- | --- | --- |
| `musa_kernel::Timeline<A>` | the event-track type at `EventTrack<C, A>` | 127c |
| `musa_kernel::sequence` | `follow` | 127c |
| `musa_kernel::overlay` | `together` | 127c |
| `Timeline::extent`, `Term::extent` | `length` | 127c |
| `Timeline::map_payload`, `Term::map_payloads` | `map_events` | 127c |
| `Beat` as an untagged position type | `Length<C>` and positions tagged by coordinate `C` | 127c |
| `musa_compiler::core::Music`, `MusicOperation`, `MusicRole` | ordinary values of ordinary types | 127e |
| the private `close` and `instantiate_music` elaboration path, and the fragment type they close | building and closing over ordinary values | 127e |
| `musa_audio::compile_graph` and public `StudioGraphSpec` as a semantic alternative | machine construction and `prepare_audio(format, machine)` | 127h |
| `RenderPlan` / `PreparedExecution` as the public prepared artifact | `PreparedMachine` | 127f, 127h |
| a caller- or preparation-chosen "semantic step" option | one audio step is one sample frame; batching is a checked `batch(n)` contract | 127h |

The kernel stays a leaf through all of this: no machine type, audio type, or frame index enters `musa-kernel`.

## 3. Serialized and interchange forms

| Deleted form | Behaviour after the break | Deleted by |
| --- | --- | --- |
| the `% musa-kernel-1` document header | refused with an error naming the version and this ledger; **not** upgraded | 127c |
| event-track encoding versions 1 and 2 (`../rules/kernel/05-normalization.md` N6, `12-payload-admission.md` A7) | refused; version 3 adds the coordinate tag and is the only accepted one | 127c |
| unframed `Display`-derived digests | already invalid; they remain invalid and are not read as track identity | (already broken) |
| any prepared-plan cache record keyed without the coordinate tag or the operation version | refused as a version error rather than treated as a miss | 127c, 127h |

A version header exists precisely so this can be a refusal rather than a guess. A reader that cannot reproduce a
document's version says so.

## 4. Fixtures

| Deleted or rewritten fixture set | Deleted by |
| --- | --- |
| all 24 files in `examples/kernel/*.musa.kernel` — regenerated at `% musa-kernel-2` with `track`, `follow`, `together`, and lengths | 127c |
| every `.musa` example and stdlib source that spells the type `Music` or calls `overlay`/`sequence` | 127e |
| the insta snapshots under `crates/musa-kernel` and `crates/musa-compiler` that pin the old kernel text | 127c, 127e |
| studio fixtures whose expected output depends on host-block-defined feedback or modulation | 127h |

Goldens are rewritten in the same prompt that breaks them, never left failing across a prompt boundary
(`prompts/README.md`).

## 5. Test names

The kernel law suite is renamed with the operations it tests. The old names must not survive as aliases, because a test
named for a deleted operation is how a deleted operation comes back.

| Old test name | New name (`../rules/kernel/04-algebraic-laws.md`) |
| --- | --- |
| `seq_associativity` | `follow_associativity` |
| `seq_zero_identity` | `follow_zero_identity` |
| `sequence_translates_and_adds_extents` | `follow_length_additivity` |
| `overlay_associativity` | `together_associativity` |
| `overlay_commutativity` | `together_commutativity` |
| `overlay_fixed_duration_identity` | `together_fixed_length_identity` |
| `overlay_takes_max_extent_and_keeps_multiplicity` | folded into `together_fixed_length_identity` and `together_not_idempotent` |
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
- unequal-length `together`, which takes the longer length and inserts no rests;
- the real-time rules: the audio callback allocates, locks, does I/O, logs, and destroys nothing;
- `scale`, `restrict`, `covering`, `prevailing`, and normalization/encoding, which are beyond the six-operation basis
  but have real callers (`../rules/kernel/00-purpose.md`).

## 7. How a prompt discharges its rows

A prompt in 127b–127i is not done while a row it owns still resolves in the workspace. The check is mechanical: after
the prompt's commit, searching the workspace for the deleted spelling returns only this ledger and the research record.
If it returns code, a fixture, or a governing document, the break is incomplete.

Prompt 127i audits the whole ledger as one of its conformance rows.
