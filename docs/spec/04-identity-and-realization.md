# Identity, realization, and caches

## 1. Payload admission

An admitted temporal payload supplies:

```text
PayloadSchema A = {
  owner_type_id,
  quotient_version,
  key : A → ByteString,
}.
```

`key` is total and deterministic and **defines** admitted payload equality:

```text
a ≡_A b iff key(a)=key(b).
```

It may quotient stored presentation detail. It is complete on equality classes, not necessarily injective on the raw
implementation struct. Changing observed fields or their canonical encodings changes `quotient_version`.

## 2. Structured temporal semantics

For `M=(d,E)`, let

```text
Sem_A(M) = (d, sort_multiset { (s,e,key(a)) | (s,e,a)∈E }).
```

Then `M≡_A N` exactly when `Sem_A(M)=Sem_A(N)`. Multiplicity is retained. Construction history and display formatting
are absent.

## 3. Exact canonical encoding

The persisted identity encoding is parameterized by a timeline encoding version and a payload schema:

```text
encode_sem[TimelineVersion,PayloadSchema A](M).
```

It contains a domain tag, both versions, owner type id, exact reduced rational extent, occurrence count, and every
canonically ordered occurrence's exact rational endpoints and payload-key bytes. Every variable-size field is
length-framed. Display text is a separate operation and may contain arbitrary delimiters or newlines without changing
the framing grammar.

### I1 — exact framing

At one fixed well-formed schema, equal semantic encodings are equivalent to equality of `Sem_A`. Encodings with
different domain/schema headers are unequal.

*Proof.* The byte grammar is uniquely decodable. Induct over the fixed record and counted occurrence list. Each exact
rational has a unique reduced representation and each variable child has one length. Conversely, equal semantic
components emit equal bytes by determinism. ∎

## 4. Digests

`semantic_hash(M)=H(encode_sem(M))` for one versioned hash operation. It is an index, not semantic equality. An unequal
digest proves unequal bytes for deterministic `H`; an equal digest requires exact encoded-byte confirmation whenever a
false hit could change a result.

Display output may have a stable golden format, but the semantic hash is not defined as “whatever `Display` writes.”

## 5. Execution factorization

Let `Sem_Gesture` be the complete structured semantic form under one admitted gesture schema. Define:

```text
prepare_execution :
  Sem_Gesture × Bindings × Seed × Options
  → Result PreparedExecution PrepareError.

prepare_lineage :
  Presentation_Gesture × PreparedExecution
  → Lineage Gesture Process × List RealizationLoss.
```

`Options` includes sample rate, channel contract, semantic tick/block policy, render bounds, deterministic quality
policy, and every other acceptance/execution choice. Bindings, seed, options, and the complete result each have owned
versioned equality. `prepare_lineage` cannot modify the execution result.

### R1 — semantic execution factorization

If semantic gesture values, bindings, seeds, and options are equal pairwise, the two `prepare_execution` calls return
equal complete results.

*Proof.* The operation is a pure deterministic function whose type exposes exactly those inputs. Substitute equal
arguments. ∎

### R1-frames — conditional observation equality

If the common result succeeds, allocation semantics agree, external input histories agree, initial state and parameters
agree, and every processor satisfies the deterministic process contract, executions produce equal output ticks.

*Proof.* Equal prepared definitions allocate equal accepted process graphs/state under the premise. Apply process
Theorem P1 tick by tick. Floating-point/device equality follows only to the extent processor conformance promises it. ∎

## 6. Collision-checked caches

For one named operation version define one canonical record:

```text
ExecArgs = {
  operation_version,
  timeline_encoding_version,
  gesture_payload_schema,
  Sem_Gesture,
  bindings_schema, Bindings,
  Seed,
  options_schema, Options,
}.
```

The cache stores `(digest(encode(ExecArgs)),encode(ExecArgs),Result)` only after computing the named operation on those
exact arguments. Lookup uses the digest to locate candidates and returns a hit only after exact complete
encoded-argument equality.

### C1 — cache correctness

A returned hit equals recomputation of the named preparation operation on the requested arguments.

*Proof.* Exact canonical equality gives equality of every argument and operation version. The insertion invariant
identifies the stored result with the operation on stored arguments. Substitute. Digest collisions only add rejected
candidates. ∎
