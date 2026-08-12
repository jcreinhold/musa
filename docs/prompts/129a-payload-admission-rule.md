---
id: 129a
slug: payload-admission-rule
status: done
depends_on: [126]
phase: 3
---

# Make Temporal Identity Exact

## Task

Write the payload-admission rule into `docs/kernel/` **before** prompt 130 admits the first payload that is not
`ScoreFact`, repair the verified unframed semantic-identity bug, and state exact `R1`. Prompt 126 decided that the core
is a calculus of occurrences of any canonical payload. The later proof review found that the current N5 display writer
is not an injective encoding and that “injective on values” contradicts `ScoreFact` deliberately dropping fields. This
prompt repairs the governing rule and implementation together; it adds no temporal operation or compiler feature.

## Read

- `docs/governance/01-constitution.md` §7 and §4 — the decision, its boundary, and its list of what is forbidden.
- `docs/kernel/00-purpose.md` §12 (payload opacity), `03-denotational-semantics.md` D1–D12, `04-algebraic-laws.md`
  L1–L24 and X1–X3, `05-normalization.md` N1–N7, `07-backend-contract.md`, `10-term-calculus.md` T1–T6 and the scope
  rule.
- `crates/musa-kernel/src/occurrence.rs` — `Canonical`, the whole contract as it stands.
- `crates/musa-kernel/tests/laws.rs` — the suite already proves L1–L24 at payload `u8`.
- `crates/musa-compiler/src/elaborate.rs` — `ScoreFact::canonical_key`, the one admission that exists, as the worked
  example the rule must describe rather than contradict.
- `docs/spec/04-identity-and-realization.md`, `docs/scratch/40-canonical-framing-bug.md`, and the accepted K₃.3 proof
  review in `docs/scratch/45-proof-review-k3.3.md`.

## Design

### The rule

A new document, `docs/kernel/12-payload-admission.md`, stating in this order:

1. **What a payload is.** An opaque value of the type parameter `A`. The kernel never looks inside one; every operation
   acts on the span and leaves the payload byte-identical (L24 is the strongest form of this, for `Progress`).
2. **What a payload owes.** `Canonical` supplies an exact stable owner type id, a quotient/schema version, and a
   deterministic total `canonical_key`. Key equality **defines the admitted payload equality**. The key is complete for
   those equality classes; it need not distinguish raw stored values which the declared quotient intentionally
   identifies. It emits no address, hash-ordered iteration, or float. Changing observed fields or their encodings
   changes the quotient version.
3. **What a payload may not do.** Add an operation to the kernel; require the kernel to inspect it; carry absolute time
   (seconds, frames, samples); carry a coinductive value. Each with the law or boundary that forbids it, citing
   `docs/governance/01-constitution.md` §7.
4. **Law transport.** L1–L23, X1–X3, and T1–T6 hold at every admitted payload **unchanged**, by genericity of the
   statements over `A` — not by a new proof per payload. L24 holds conditionally: at a payload carrying a `Progress`.
   N1–N7 hold given `Canonical`. Say which laws are transported and which are conditional, and say that the law suite's
   instantiation at a non-`ScoreFact` payload is the evidence.
5. **What `canonical_key` may quotient away, and what that costs.** N3 is the admitted equality projection and may drop
   presentation detail; it is not an interchange form. The admission record says what is dropped and why. Remove every
   contradictory “injective on stored values” statement from kernel docs and trait comments.
6. **The admission table.** One row per admitted payload: the payload, the crate that defines it, what its key includes,
   what it deliberately quotients away, and the falsifying example that would show the choice wrong. `ScoreFact` is the
   first row and must be written from the code as it stands, not from what it ought to be.

### Exact semantic framing

Separate human `Display`/N5 text from persisted semantic identity. Define one versioned private timeline semantic writer
containing:

- a domain tag and timeline encoding version;
- payload owner id and quotient version;
- exact reduced rational extent;
- occurrence count; and
- canonical occurrences with exact rational endpoints and **byte-length-framed** payload keys.

Every variable child is length/count framed. `Timeline::semantic_hash` hashes these bytes, not `Display`. A digest is an
index only; equal digests do not prove semantic equality. Update N3–N6, T3, and backend text accordingly. Retain display
format only for human/golden consumers.

Add the exact regression:

```text
M = extent 2, one String occurrence [1,2] with key
    "a from 0 to 1;\n  occurrence b"
N = extent 2, String "a" on [0,1] and "b" on [1,2].
```

Current `Display`/old N5 bytes are equal while `semantic_eq` is false. The new semantic hashes/encodings must differ.
Add arbitrary delimiter/newline payload-key properties, schema-header differentiation, multiplicity, and
`semantic_eq ⇒ equal hash` tests. Fixed digest goldens change under an explicitly documented encoding-version bump.

### The rendering law

Add exact `R1` from `docs/spec/04-identity-and-realization.md`: `prepare_execution` is a pure deterministic function of
`Sem_Gesture × Bindings × Seed × Options` returning the complete `Result`. Presentation-only lineage is separate. Frame
equality additionally requires equal external input histories, initial/allocation state, and conforming deterministic
processors. A cache stores and confirms exact complete framed arguments after digest lookup.

### The evidence

Law transport is a claim about genericity, and the cheapest honest proof is a second instantiation. `laws.rs` proves
L1–L24 at `u8` today. Add one payload type in the test suite that is *structurally* unlike `u8` — a small record with a
string field, an exact rational, and a `Progress` — implement `Canonical` for it, and run the existing law properties at
it. If a law needs a different generator but not a different statement, that is the transport claim demonstrated. If any
law needs a different *statement*, the admission rule is wrong and this prompt is repaired before 130 runs.

## Target

- `docs/kernel/12-payload-admission.md` — the rule, the transport statement, and the admission table with `ScoreFact` as
  its first row.
- Repaired `docs/kernel/{02-static-semantics,05-normalization,07-backend-contract,10-term-calculus}.md` identity claims.
- `R1` in `docs/kernel/07-backend-contract.md`.
- `docs/kernel/00-purpose.md`'s document list and any `docs/kernel/README`-equivalent index updated to name the new
  document.
- A second payload instantiation in `crates/musa-kernel/tests/laws.rs` exercising the existing law properties, named so
  the suite says what it protects.
- Versioned schema metadata on every current `Canonical` implementation and a framed semantic hash writer separate from
  `Display`, with the exact/adversarial regressions above.
- the payload-admission obligation marked satisfied, with the document it points at.

## Check

```sh
cargo nextest run -p musa-kernel
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
mdwright fmt-check docs/kernel/*.md docs/governance/*.md
python3 scripts/renumber-prompts.py audit
```

The law suite must pass at both payloads with the same property statements. Commit as `Make temporal identity exact`.

## Stop

- No new temporal operation, term form, or dependency. Public `Canonical` metadata may change only as required to make
  the existing equality/hash contract exact.
- No gesture payload. Prompt 130 defines it; this prompt defines what it will have to satisfy.
- No signal payload, and no re-opening of `docs/governance/01-constitution.md` §4.
- No change to which `ScoreFact` fields its key observes. Add its owner/version metadata and record the current
  quotient; a different quotient is a separate design decision.
