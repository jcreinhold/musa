# Exact equality and stored data

This page explains how the implementation stores values that may be compared, cached, or loaded after a restart. The
Rust identifiers still carry their pre-127a spellings; the pairs are in
[`../clean-break-ledger.md`](../clean-break-ledger.md).

## 1. Do not use one equality for every job

| Question | Example | Fields it may ignore |
| --- | --- | --- |
| Do these values have the same meaning in this representation? | `EventTrack<WrittenTime, ScoreFact>` | construction order and fields excluded by the payload equality rule |
| Did these results come through the same recorded conversions? | an origin path | insertion order and exact duplicate paths |
| Are these the same prepared instructions for the audio engine? | `PreparedMachine` | source locations used only by the editor |
| Did these two runs produce the same observable output? | audio-history or conformance comparison | only differences allowed by the named comparison rule |
| Are these the same machine? | `≡struct`, within one registry | nothing; behavioural equality is not decidable and no cache key may use it |

A shared hash does not connect these rows. Any theorem that moves from one equality to another must name both equality
rules and state its extra assumptions.

## 2. Encoding values for exact comparison

Every saved value that takes part in equality needs three private operations:

```text
schema version
encode(value) -> unambiguous bytes
compare(left, right) -> ordering
```

Byte equality and value equality must agree. The encoding begins with a type tag and version. It records the duration of
every variable-size child before the child bytes. Human display and error messages use separate formatting.

This rule applies to event-track payloads, future nominal type ids, stored-representation references, anchors, origin
paths, instrument bindings, seeds, audio options, and prepared results.

The code does not need one public `CanonicalData` trait. Private writer functions are better until several real callers
need exactly the same API.

## 3. Event-track equality is now framed and versioned

`Canonical::canonical_key()` defines equality for one track payload type. The trait also records the payload owner and
equality version. A key may ignore stored fields if its documentation says so.

The track's semantic hash no longer hashes human `Display` output. It hashes a versioned byte record containing the
payload schema, the coordinate tag, the exact rational duration, occurrence count, endpoints, and duration-framed
payload keys. Prompt 127a's addition is the coordinate tag: a written-time track and a performed-time track with the
same occurrences are different values (`../../rules/kernel/12-payload-admission.md` A7).

The implementation includes regression tests for:

- the old newline and delimiter collision;
- arbitrary string keys;
- payload schema changes;
- duplicate occurrences;
- equal tracks built in different orders; and
- the same track laws at a structured payload containing text, a rational, and a progress curve.

Old unframed digests belong to an old format version and must not be read as new track identity; so do the two refused
encoding versions listed in [`../clean-break-ledger.md`](../clean-break-ledger.md).

## 4. Cache records

A cache whose false hit could change a result stores:

```text
(hash, exact encoded arguments, exact result)
```

The hash selects a bucket. Exact argument comparison confirms the hit. The argument record includes the operation
version, every data-schema version, gesture meaning, instrument and studio bindings, seed, sample rate, channel layout,
render bounds, and all other execution options. It does not record a "semantic step": the step is one sample frame by
constitution §4 and is not a caller option.

Origin data may need a separate cache because editor navigation can distinguish inputs that audio execution deliberately
ignores. A prepared-machine cache hit does not imply equal origin paths, and an origin-cache hit cannot stand in for a
prepared machine.

## 5. Stored representations and origin paths

One stored representation contains or resolves:

- its id and version;
- its kind and schema;
- its root, generation sites, and anchor table;
- its exact manifest;
- the descriptors of conversions used to make it; and
- complete origin paths and loss records.

Hashes may help find a manifest. Loading or merging compares the exact descriptors and rejects one id paired with two
different records. The registry does not change while a conversion is being checked. A source edit creates a new version
instead of changing the meaning of an old id.

## 6. Reading old data

When a schema changes, a reader has three honest choices:

1. read the old version and run a named checked migration;
2. rebuild the result from source and locked packages; or
3. reject the value with a version error.

It must not reuse an old version number, ignore a new field that affects equality, or accept a record solely because its
hash or byte length looks plausible.
