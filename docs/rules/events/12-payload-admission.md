# 12 — Payload Admission

**Status: governing.** Which types may be an occurrence payload, and which coordinates a track may carry.

The event-track core is parametric in a coordinate `C` and a payload `A`. This document fixes the evidence a payload
type must supply before it is used with normalization, equality, or semantic identity. Admission does not add a temporal
operation and does not teach the core the payload's musical meaning.

## A1 — A payload is opaque

A payload is **storable data** (`../constitution.md` §9): an ordinary finite value at the type parameter `A` that
contains no source function at any depth and has a versioned finite exact encoding. The core may move, copy, sort, and
compare its canonical key; it never branches on the value or rewrites one of its fields. `track`, `follow`, `together`,
`scale`, and `restrict` act on occurrence support. L24 is the strongest instance: a locally normalized `Progress` keeps
identical payload bytes while its containing span moves or stretches.

A type carrying a function is refused here rather than diagnosed later: it has no bytes, so it has no key, so it has no
equality the core could use.

## A2 — The `Canonical` obligation

An admitted payload implements:

```text
Canonical A = {
  OWNER_TYPE_ID : stable text,
  QUOTIENT_VERSION : u32,
  canonical_key : A → text,
}.
```

The owner id names the schema which defines the value. The quotient version names its equality projection and key
encoding. `canonical_key` is deterministic and total. It emits no address, hash-iteration order, platform-dependent
value, or float.

Key equality **defines** admitted payload equality:

```text
a ≡_A b  iff  canonical_key(a) = canonical_key(b).
```

The key is complete for these equality classes. It need not distinguish two raw implementation values when the declared
quotient intentionally identifies them. Changing an observed field, a deliberately omitted field, or the field encoding
is an equality design decision and requires reviewing and normally incrementing `QUOTIENT_VERSION`.

## A3 — What a payload may not demand

An admission may not:

- add a core operation or term form—`10-term-calculus.md`'s scope rule and the constitution's §8 apply;
- require the core to inspect musical content—payload opacity and the constitution's §8 apply;
- carry an absolute position, in any coordinate—K2 and the constitution's §3 apply, because time lives in the
  occurrence's span and in the track's coordinate, never in the payload;
- carry a frame index or a sample—a frame is a machine's index, not a track position (`../constitution.md` §4); or
- carry a coinductive stream, an audio history, or a machine's private state—a finite track is not a running source,
  under the constitution's §3–§5.

An exact rational control shape indexed by normalized local occurrence time is finite payload data. A sample stream is
not. An opaque handle naming a scheduled source is finite payload data; the source it names is not.

## A4 — Law transport

L1–L23, X1–X3, and T1–T6 quantify generically over `A` and transport unchanged to every admitted payload. N1–N7 hold
given the `Canonical` contract. L24 is conditional: it applies to a payload which carries a `Progress` indexed by local
normalized time.

This is parametric transport, not a new theory-specific proof. `crates/musa-events/tests/suite/laws.rs` instantiates the
same statements both at scalar `u8` and at `AdmissionProbe`, a record containing delimiter-bearing `String`, exact
`Ratio`, and `Progress` fields. If a new payload needs a changed temporal-law statement, it has failed admission.

## A5 — Equality projection is not interchange

N3's key may drop presentation detail and therefore need not reconstruct the stored payload. An interchange adapter owes
the stronger round-trip law and uses `TextPayload`; it is a different function. Dropped fields and their cost are
recorded below. A consumer needing a finer relation defines a separately named projection rather than silently
strengthening core equality.

## A6 — Admission table

| Payload | Owner | Key includes | Deliberately quotients away | Falsifying example |
| --- | --- | --- | --- | --- |
| `u8` | `musa-events` | decimal scalar | nothing | two bytes receive one decimal key |
| `String` | `musa-events` adapter for Rust `String` | the exact UTF-8 string | nothing | two strings receive equal bytes |
| `Progress` | `musa-events` | every exact ordered `(u,v)` breakpoint | nothing | distinct curves receive one breakpoint list |
| `ScoreFact` | `musa-compiler` | scope; full fact kind; source span; expansion path | elaboration-only `tied`; `Origin.definition_span`; `Origin.declaration` | a cache, projection, or lineage consumer requires one omitted field to distinguish execution results |
| `Gesture` | `std::performance`, projected at the compiler/runtime boundary | constructor; stable gesture identity; written pitch; exact controls and normalized-local curves; techniques; group/member identity | presentation `Origin`; separately keyed written-support lineage | a scheduler or instrument distinguishes two payloads whose keys compare equal, or moving an occurrence changes its payload bytes |

The `ScoreFact` row records the implementation as it exists. Its omitted origin fields remain available in the stored
fact and its interchange form. Core semantic equality does not observe them. A future identity-sensitive preparation
must use a complete presentation or a named semantic projection whose fields match its actual decisions; it may not
pretend the coarser `ScoreFact` equality contains those fields.

## A7 — Exact temporal framing

Human N5 display is not an identity grammar: keys may contain its newlines and delimiters. N6 therefore hashes a private
versioned record with this field order:

```text
length(domain tag), domain tag,
track encoding version,
length(coordinate tag), coordinate tag,
length(payload owner id), payload owner id,
payload quotient version,
duration numerator, duration denominator,
occurrence count,
for each canonical occurrence:
  start numerator, start denominator,
  end numerator, end denominator,
  length(payload-key bytes), payload-key bytes.
```

Lengths/counts are unsigned 64-bit big-endian; versions are unsigned 32-bit big-endian; rational components are signed
64-bit big-endian and denominators are positive. Rationals are reduced by the exact-time representation. The current
track encoding version is **3**, which adds the coordinate tag. Version 1 was the unframed N5 display stream and version
2 the framed bytes that predated the coordinate; both are **refused**, not reinterpreted
(`../../plan/clean-break-ledger.md`).

The coordinate tag is framed rather than positional so that a reader refusing an unknown coordinate refuses it by name.

This grammar is uniquely decodable. At a fixed schema its complete framed-byte equality is exactly N4 semantic equality.
`SemanticHash` is FNV-1a-128 over these bytes, but the digest is only an index: an equal digest does not prove byte
equality. Correctness-sensitive caches retain and confirm the complete framed arguments after lookup.
