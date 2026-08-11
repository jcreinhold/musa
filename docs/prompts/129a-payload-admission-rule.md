---
id: 129a
slug: payload-admission-rule
status: pending
depends_on: [126]
phase: 3
---

# What a Kernel Payload Owes

## Task

Write the payload-admission rule into `docs/kernel/` **before** prompt 130 admits the first payload that is not
`ScoreFact`, and write the rendering law `R1` into the backend contract. `docs/core-boundary.md` decided that the core
is a calculus of occurrences of any canonical payload; the kernel specification has never said what a payload owes,
because there has only ever been one. If prompt 130 writes both the rule and the payload, the rule is fitted to the
payload — the drift `AGENTS.md` forbids. This prompt produces specification text and a law-suite instantiation. It adds
no kernel operation and no compiler feature.

## Read

- `docs/core-boundary.md` — the decision, §4's obligations, §5's boundary, §6's list of what is forbidden.
- `docs/kernel/00-purpose.md` §12 (payload opacity), `03-denotational-semantics.md` D1–D12, `04-algebraic-laws.md`
  L1–L24 and X1–X3, `05-normalization.md` N1–N7, `07-backend-contract.md`, `10-term-calculus.md` T1–T6 and the scope
  rule.
- `crates/musa-kernel/src/occurrence.rs` — `Canonical`, the whole contract as it stands.
- `crates/musa-kernel/tests/laws.rs` — the suite already proves L1–L24 at payload `u8`.
- `crates/musa-compiler/src/elaborate.rs` — `ScoreFact::canonical_key`, the one admission that exists, as the worked
  example the rule must describe rather than contradict.

## Design

### The rule

A new document, `docs/kernel/12-payload-admission.md`, stating in this order:

1. **What a payload is.** An opaque value of the type parameter `A`. The kernel never looks inside one; every operation
   acts on the span and leaves the payload byte-identical (L24 is the strongest form of this, for `Progress`).
2. **What a payload owes.** `Canonical::canonical_key` must be **deterministic** (same value, same key, same process or
   not), **total** (no panic, no failure case), and **injective** (distinct values, distinct keys) — with injectivity
   stated as an obligation on the *implementor*, since the kernel cannot check it. It must emit no address, no
   hash-ordered iteration, and no float. State why each: determinism and hash-order for reproducible builds and semantic
   hashing, floats because N4 equality would inherit float equality's failures.
3. **What a payload may not do.** Add an operation to the kernel; require the kernel to inspect it; carry absolute time
   (seconds, frames, samples); carry a coinductive value. Each with the law or boundary that forbids it, citing
   `docs/core-boundary.md` §6.
4. **Law transport.** L1–L23, X1–X3, and T1–T6 hold at every admitted payload **unchanged**, by genericity of the
   statements over `A` — not by a new proof per payload. L24 holds conditionally: at a payload carrying a `Progress`.
   N1–N7 hold given `Canonical`. Say which laws are transported and which are conditional, and say that the law suite's
   instantiation at a non-`ScoreFact` payload is the evidence.
5. **What `canonical_key` may quotient away, and what that costs.** N3 is the *equality* serialization and may drop
   detail; it is not an interchange form (`05-normalization.md` N3). A payload that quotients away detail is choosing
   what two values of it mean by "the same"; the admission record must say what was dropped and why.
6. **The admission table.** One row per admitted payload: the payload, the crate that defines it, what its key includes,
   what it deliberately quotients away, and the falsifying example that would show the choice wrong. `ScoreFact` is the
   first row and must be written from the code as it stands, not from what it ought to be.

### The rendering law

Add `R1` to `docs/kernel/07-backend-contract.md`, in the words `docs/core-boundary.md` §5 fixes: semantic equality of
gesture timelines implies identical preparation and identical rendered frames under the same bindings and seed, and
rendering may observe nothing normalization forgets (N7). State that prompt 144 measures it and that a measured failure
reopens `docs/core-boundary.md` §5 rather than being patched downstream.

### The evidence

Law transport is a claim about genericity, and the cheapest honest proof is a second instantiation. `laws.rs` proves
L1–L24 at `u8` today. Add one payload type in the test suite that is *structurally* unlike `u8` — a small record with a
string field, an exact rational, and a `Progress` — implement `Canonical` for it, and run the existing law properties at
it. If a law needs a different generator but not a different statement, that is the transport claim demonstrated. If any
law needs a different *statement*, the admission rule is wrong and this prompt is repaired before 130 runs.

## Target

- `docs/kernel/12-payload-admission.md` — the rule, the transport statement, and the admission table with `ScoreFact` as
  its first row.
- `R1` in `docs/kernel/07-backend-contract.md`.
- `docs/kernel/00-purpose.md`'s document list and any `docs/kernel/README`-equivalent index updated to name the new
  document.
- A second payload instantiation in `crates/musa-kernel/tests/laws.rs` exercising the existing law properties, named so
  the suite says what it protects.
- `docs/core-boundary.md` §4's obligation 1 marked satisfied, with the document it points at.

## Check

```sh
cargo nextest run -p musa-kernel
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
mdwright fmt-check docs/kernel/*.md docs/core-boundary.md
python3 scripts/renumber-prompts.py audit
```

The law suite must pass at both payloads with the same property statements. Commit as `State what a kernel payload
owes`.

## Stop

- No new kernel operation, no new term form, no change to `musa-kernel`'s public surface beyond what the test suite
  needs, and no new dependency.
- No gesture payload. Prompt 130 defines it; this prompt defines what it will have to satisfy.
- No signal payload, and no re-opening of `docs/core-boundary.md` §5.
- No change to `ScoreFact::canonical_key`. The admission table records what it does; if what it does looks wrong, that
  is a finding to report, not a repair to make here.
