---
id: 141ha
slug: machine-core
status: done
depends_on: [136, 141e, 141g, 141h]
phase: 3
---

# Give the Machine a Core Shape

## Task

`crates/musa-compiler/src/registry/rules.rs`'s `UNREGISTERED` still holds `("machine builtins", 9)`. They are
`primitive`, `machine`, `identity`, `connect`, `beside`, `feedback`, `copy`, `drop`, and `swap` — the nine admissible
forms of [`../../rules/across-stages/03-machine-calculus.md`](../../rules/across-stages/03-machine-calculus.md) §2 — and
the reason they are unregistered is not the one `rules.rs` gives. Give them a core shape, and register them.

This prompt exists because prompt 141h's repair found the machine half to be a different problem from the track half,
with no lemma shared with it. §5.7 says as much itself: a machine's safety is M1–M8 of §7, "stated over a step relation
this section has no vocabulary for", and "an argument that covered both would have had to talk about a 'value' general
enough to be either, and that generality is the contextual-`Music` mistake in a new place".

## The three things that make this its own prompt

**A machine builtin can never fire, and must not.** `musa-calculus`'s `eval.rs` reduces a δ-builtin only when every
argument on its spine is canonical data, and `canonical` answers `None` at `Form::Universe`. Every machine form is
polymorphic in its step and its ports, so a type stands on every one of those spines and no rule of theirs would ever
run. That is not a defect to route around: §2 gives the nine forms **typing rules and no reductions**, so a machine's
application *is* its value, and two machines are the same machine exactly when they were built the same way. What the
core is missing is a way to say so — `Reduction` has `Delta` and `Structural` and no third variant, and `Builtin::new`
demands a `Rule`.

**A machine needs a unit and a product.** §2's rules are written over `(A, D)` and `Unit`: `beside` pairs both ports,
`copy` answers a pair, `drop` answers unit, `swap` exchanges a pair's halves, and `feedback` threads a stored value
through a pair on both sides. `crates/musa-compiler/src/prelude.rs` declares `Nat`, `Bool`, `Option`, `List`, `Result`,
and the musical families, and no product and no unit. The old checker had `Type::Product` and `Type::Unit` as checker
types with no source spelling; the core has records (prompt 136) and nothing anonymous.

**`primitive` is typed by a registry, not by a signature.** `MachineOp::instantiate` returns `None` for it, and the old
checker types it by a rule of its own: the literal name and version select a unit from the build-local registry, and its
ports come from that unit. Nothing in `musa-calculus` reads a build-local anything, so this is the one form whose
registration has a real decision in it rather than a transcription.

## Read

- [`../../rules/across-stages/03-machine-calculus.md`](../../rules/across-stages/03-machine-calculus.md) §2 in full —
  the nine forms, their typing rules, the storable-port premises, and the two sentences that decide the shape here:
  "there is no public `lift` from a source function into a machine", and "the step tag `K` prevents machines whose steps
  mean different things from being connected". §7's M1–M8 are the safety obligations; prompt 148 owes their proof and
  this prompt owes them a representation they can be stated about.
- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.8's fourth family and the
  §5.7 paragraph "What this obligation does not cover", which is the argument for this prompt being separate.
- `crates/musa-calculus/src/base.rs` — `Reduction`, `Builtin::new`, `Builtin::structural_with`, `Family::Machine`, and
  `Registry::new`'s two checks. `check_delta_signatures` already skips a non-δ family, so a machine signature is not
  held to D1; what is missing is the registration shape, not permission.
- `crates/musa-calculus/src/eval.rs`'s `delta` and `canonical`, which is where "can never fire" is a fact rather than a
  worry.
- `crates/musa-compiler/src/core.rs` — `MachineOp` and its `instantiate`, `Type::Machine`, `Type::Primitive`,
  `Type::Product`, `Type::Unit`, and `machine_type`. The rank-1 schemes there are the signatures being restated as
  ordinary Π types, and `instantiate`'s doc says which variables are `Kind::Data` and which are `Kind::Ordinary` — that
  distinction is §1.1's storable-data rule and has to survive the restatement.
- `crates/musa-compiler/src/machine.rs` and `studio.rs` — what a machine value becomes downstream, and therefore what
  prompt 142 will have to read back out of a normal form. A neutral spine is read the way `crate::registry::held` reads
  a literal; nothing here writes that reader.
- [`136`](136-records-and-enums.md) for what a record type is in the core, which is the candidate a product is chosen
  from or against, and [`141h`](141h-track-core.md) for the base-type inertness test and for the `Family::Track`
  precedent of a registered operation that is not a δ-builtin.
- [`141e`](141e-compiler-registry.md) for how a signature is read off the old table rather than retyped, which is what
  keeps nine restated types from being nine chances to write a different one.

## Design

**A machine is a base type indexed by its step and its ports, and its nine forms are constructors.** `Machine` is
registered at `Type 0 → Type 0 → Type 0 → Type 0` and `Primitive` at the same kind, so `connect`'s signature is the
ordinary Π that §2's rule already is:

```text
connect : {step in mid out : Type 0} → Machine step in mid → Machine step mid out → Machine step in out
```

The indices are types rather than literals because §2's ports are arbitrary storable data, and a base type is not held
to D1's literal-index rule — that rule is about what a `fn` **rule** can write, and a constructor writes nothing. This
is the whole reason the third reduction shape below is the right answer rather than a workaround.

**`musa-calculus` gains `Builtin::constructor`, and `Reduction` gains `None`.** A builtin registered that way has a
type, a family, and no computation: its saturated application stays a neutral spine, and conversion of two of them is
the structural conversion the core already does on spines. Three consequences are worth stating because they are what
makes this safe rather than convenient — a constructor contributes no ι-rule, so §5.8's inertness argument covers it
unchanged; `Malformed::BuiltinStuck` cannot fire for it, because there is no rule to answer `None`; and it cannot be
confused with a δ-builtin whose rule was forgotten, which is exactly what "a rule that always answers `None`" would have
been.

**The product is a decision, not a transcription.** Two candidates, and the prompt must pick one and say why in the
registration's doc comment: a **declared `Pair A B` family with `Unit`** in `crates/musa-compiler/src/prelude.rs`, which
is one more declared family and gives §2's `(A, D)` a name a program can match on; or **the core's record types**
(prompt 136), which are structural and would spell `(A, D)` as a two-field record with fixed field names. Prefer the
declared family unless the record answer removes a spelling the surface will need in 142: §2's pairs are *positional*
wiring, `copy` and `swap` say nothing about what the halves are called, and inventing field names for them would be the
surface reading a wiring diagram as a record. Whichever is chosen, `Unit` is declared beside it, because `drop`'s result
is the one place the language needs a type with exactly one value and nothing else.

**`primitive` is registered against what the build-local registry says.** Its signature cannot name the unit's ports,
because they depend on a name and a version it is applied to. Register it with the type its old rule enforces —
`primitive : (name : Text) → (version : Text) → (configuration : Text) → Option (Primitive ⟨step⟩ ⟨in⟩ ⟨out⟩)` is *not*
writable, because the indices are the answer rather than the argument — so this is the one form that stays out until 142
wires the registry, **or** is registered at a signature that takes the ports explicitly and leaves the build-local check
to the elaboration that has the registry. Decide it in the prompt's own doc comment, record which, and if it stays out,
add it to `UNREGISTERED` with the prompt that owns it named rather than leaving the count wrong.

**Nothing is wired and nothing is deleted.** As with 141e, 141f, 141ga, and 141h, what this prompt adds is a second,
unused spelling. `Type::Machine`, `Type::Primitive`, and the old checker's machine path survive until 142.

## Target

- `crates/musa-calculus/src/base.rs`: `Reduction::None`, `Builtin::constructor(name, ty, family)`, and the doc paragraph
  arguing why a constructor is not a rule that never fires. `Registry::new`'s existing checks unchanged — a constructor
  is not a δ-builtin and not a structural eliminator, so neither check applies to it, and the laws say so.
- `crates/musa-compiler/src/prelude.rs`: `Unit`, and the product the Design's decision selected, each doc-commented with
  the §2 rule that needs it.
- `crates/musa-compiler/src/registry.rs`: `Machine` and `Primitive` registered as type-indexed base types, and a
  `registry/machine.rs` holding the nine signatures, read off `MachineOp::instantiate` rather than retyped.
- `crates/musa-compiler/src/registry/rules.rs`: `REGISTERED` and `UNREGISTERED` updated, leaving
  `("structural eliminators", 8)` and `("phase projections", 1)` — plus `primitive` if the Design's decision left it
  out, with its owning prompt named.
- Laws: each of the nine checks at the type §2 gives it, on a sample of ports; two machines built the same way are
  convertible and two built differently are not; a machine with an open port is a well-typed value that is simply not
  yet *a* machine; `connect` at mismatched ports is refused by the core rather than by a hand-written arm; and a
  constructor's saturated application normalizes to itself.
- `docs/plan/code-map/` rows for `musa-calculus` and `musa-compiler`.
- Prompt 142's Read and `depends_on` repaired to name this prompt.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo clippy --all-targets -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Give the machine a core shape`.

## Stop

- No tenth form. §2's set is closed, and a tenth is a change to the calculus — stop condition 4, not a repair.
- No `lift`, and no machine built from a source function. §2 forbids it in one sentence, and a core that admitted one
  would make M1–M8 unprovable.
- No step semantics, no scheduling, no audio. §3's one step and §7's M1–M8 are prompts 148 and 150–153; this prompt owes
  them a representation, not a runtime.
- No deletion of `Type::Machine`, `Type::Primitive`, `MachineOp`, or the old checker's machine path. 142 owns them.
- No amendment to `03-machine-calculus.md` or to §5.8. A form that cannot be written in the core's terms is a finding.
- No change to `musa-audio` or `musa-engine`. The machine's core spelling is a description; what runs one is downstream
  and unchanged.
