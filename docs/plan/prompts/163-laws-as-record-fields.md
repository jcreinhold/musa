---
id: 163
slug: laws-as-record-fields
status: pending
depends_on: [156, 157]
phase: 3
---

# Let a Structure Carry Its Laws

## Task

`stdlib/src/algebra.musa` says, in its own words, that *"a trait declaration in this language carries methods and never
obligations… an obligation needs evidence, and evidence needs a term witnessing an equality."* Prompt 156 made `Equal`
declarable and prompt 157 made a structure an ordinary record, so both halves of that sentence are now available. Give
`Group`, `Action` and `Torsor` their equations as fields.

## Read

- `stdlib/src/algebra.musa` — the three structures and the sentence above.
- `docs/rules/language/05-verification.md` §4 — the law suites that are currently the only place a law is written down.
- `stdlib/src/transformational.musa` — in particular the note that PLR chains do not close in the spelled domain, which
  is a law with a *precondition*, and the interesting case for this prompt.

## Design

**A law is a field, and its inhabitant is a proof term.**

```
record Group(G : Type) {
  unit    : G,
  compose : G -> G -> G,
  inverse : G -> G,
  left_unit : (x : G) -> Equal (compose unit x) x,
  ...
}
```

Constructing a `Group` now requires supplying the equations, which is the point: an instance that does not satisfy them
cannot be built.

**And here is the honest cost, stated up front.** Musa refuses tactics, proof search, and a hint database. So every law
field must be inhabited by a term the author *writes*, and for the concrete carriers in the stdlib those are `refl`
after both sides normalize — which works exactly when the operation is δ-reducible on canonical data, and does not when
the carrier is abstract. **Where it does not work, the field is not added.** This prompt's deliverable is the laws that
can be discharged by computation, not a demonstration that all of them can.

**Which laws those are, expected before the work starts.** The pitch-class group at a fixed modulus, the interval torsor
over written pitch, and transposition as an action all compute. The transformational laws with preconditions — PLR
closing only after spelling is forgotten — become laws over `triad_classes`, which is the type the precondition was
already carved into. Anything else is recorded as *not discharged*, with the reason, in the module.

**`05-verification.md` §4 does not go away.** A property test over generated values and a proof term are different
evidence: one covers the concrete carriers the corpus actually uses, the other covers all of them. Where a law is
proved, the suite keeps the test as a check on the *proof* being about the operation the code calls.

**This is where "not a proof assistant" gets its real test.** If discharging these laws turns out to want tactics, that
is evidence for the refusal, not against it: it means the boundary is in the right place and the answer is fewer law
fields, not more apparatus. Record which way it went.

## Target

- `stdlib/src/algebra.musa`: law fields on `Group`, `Action`, `Torsor`, with the undischarged ones named and explained.
- `stdlib/src/transformational.musa`: the PLR laws over `triad_classes`.
- `docs/rules/language/05-verification.md` §4: which laws are proved, which are tested, and why each is where it is.
- `crates/musa-compiler/tests/suite/`: the law suites kept, pointed at the proved operations.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa -- check stdlib/src/*.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Let a structure carry its laws`.

## Stop

- No tactic language, no `auto`, no proof search, no hint database. If a law needs one, it does not get a field.
- No new equality reasoning in the core. `Equal` and `rewrite` as 156 left them.
