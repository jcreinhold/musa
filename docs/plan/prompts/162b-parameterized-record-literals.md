---
id: 162b
slug: parameterized-record-literals
status: done
depends_on: [157, 161]
phase: 3
---

# Let a Parameterized Record Be Constructed

## Task

`record Cell<A> { at: Nat; value: A; }` declares. Nothing constructs it. `Cell { at = 0, value = 1 }` is refused with
"this stands where a type is needed, but it is not one", because the lowering annotates a record literal with the bare
family name and `Cell : Type 0 → Type 0` is not a type. Every parameterized record in the library is therefore a
declaration nobody can make a value of — including all three of `stdlib/src/algebra.musa`'s, which
[`03-musical-domains.md`](../../rules/language/03-musical-domains.md) §5 and prompt [164](164-builtin-collapse.md) both
require as *values*. Make the literal find its family and solve the family's parameters.

## Read

- `crates/musa-compiler/src/lower/values.rs`'s `record` — five lines, and the defect is the last two:
  `Raw::annot(origin, built, Raw::var(at, name))`. The annotation is elaborated by `check_type`, which wants a universe
  and gets a Π.
- `crates/musa-compiler/src/lower.rs`'s `Lowering` doc — "**No expected type.** … a lowering that took one would be a
  second checker." That is why the fix is not "look up the arity here", and it is the right rule to keep.
- `crates/musa-calculus/src/elaboration/elab/check.rs`'s `RawShape::Record` arm — `self.product(ty)` already checks a
  literal against a known goal, and `crates/musa-calculus/src/elaboration/elab/infer.rs:101` is the matching
  `Refusal::Uninferable`. So the checking half works and the *inferring* half is what the annotation exists for.
- `crates/musa-calculus/src/elaboration/raw.rs:376` — `RawShape::Record(Arc<[RawField]>)`, which carries fields and no
  head.
- `crates/musa-compiler/src/lower/laws.rs`'s `a_parameterized_record_puts_its_parameter_on_the_family` — the law that
  fixes what `Cell<Nat>` means: `Cell Nat`, the parameter explicit, "because a written `Cell<Nat>` elaborates to an
  application and not to an implicit solve". That law is about the *type*; this prompt is about the term, and must not
  change it.
- `docs/rules/language/01-surface.md` §1.2 — "**Parameters are allowed and are ordinary**", and "a literal [elaborates]
  to that constructor applied to its fields". The spec already says this works; the compiler does not. That makes this a
  defect and not a feature, which is why the Stop below forbids widening it into one.
- Prompt [157](157-records-leave-the-core.md), which made a record a one-constructor family, and
  [161](161-one-declaration-form.md), which made `record` sugar over `data`. The literal's elaboration is theirs; this
  prompt finishes it.

## Design

**The head belongs in the raw term, not in an annotation.** `RawShape::Record` gains an optional head name. `Cell { … }`
lowers to a record with head `Cell`; a literal written with no head — which the parser does not accept today and this
prompt does not add — would carry `None` and keep the current inferring refusal. The annotation goes away, and with it
the accidental demand that a family constant be a type.

**Checking is unchanged and inferring is what gains.** With a goal, the head is not needed: `check.rs`'s `product`
already takes the goal apart. Where there is no goal, the head names the family, and the family's parameters become
metavariables the fields solve — `Cell { at = 0, value = 1 }` infers `Cell ?A` and the second field solves `?A := Nat`.
An unsolved parameter is the ordinary unsolved-metavariable refusal, pointing at the literal, and not a new diagnostic.

**The head is checked against the goal where both exist.** `let c: Cell<Nat> = Other { … };` names two families and must
say so. Today the annotation catches this as a conversion mismatch; keep a refusal that names both, because a literal
whose head disagrees with its goal is a real mistake and silence about it would be worse than the bug this prompt fixes.

**One defect was found under this one and is fixed here.** `check.rs`'s `literal` walked the family's parameters with
`apply` — which applies a λ — where a constructor's *type* is a Π and the walk belongs to `record::instantiated`, whose
doc comment already said "every rule here walks an accessor's or a constructor's telescope one argument at a time, so
the one walk lives here". It stood because it was unreachable: no parameterized record could be written at all. Fixing
it is not scope creep; without it the head this prompt adds reaches a malformed-core report instead of a value.

**A record pattern is the same question and already has an answer.** `Pending { read = r }` matches through
`02-core-calculus.md` §6.2's case tree, where the scrutinee's type supplies the family. Nothing here changes it, and the
Check below asserts that.

**`Duration` keeps its refusal.** The tempting fix — make `check_type` insert metas until a Π reaches a universe — would
turn `let d: Duration = …` from "`Duration` takes an argument; write `Duration<WrittenTime>`" into a silent solve for
the coordinate. That diagnostic is deliberate and a law names it. The parameters solved by this prompt are solved at a
*term*, from its fields, and nowhere else.

## Target

- `RawShape::Record` carries an optional head; the lowering fills it; the annotation is gone.
- Inferring a headed literal solves the family's parameters from the fields.
- A headed literal whose head disagrees with its goal is refused, naming both families.
- `stdlib/src/pitch.musa` gains the three constructions `stdlib/src/algebra.musa` declares and has never been able to
  make: a `Group<Interval>`, an `Action<Pitch, Interval>`, and a `Torsor<Pitch, Interval>`, each written out and each
  named by a law.

  **In `pitch.musa` and not in `algebra.musa`.** `pitch.musa` imports `algebra`, so `algebra` cannot import `pitch` —
  the bundled-module probe answers "these files import each other" — and a construction written in `algebra.musa`
  would have to reach past `Interval.compose` and `Pitch.act` to the builtins under them. That is the third spelling of
  one operation `algebra.musa`'s own head comment exists to prevent. A structure is declared where the vocabulary is
  and constructed where the carrier is, which is also the ordinary direction: `algebra` says what a group is, `pitch`
  says that written intervals are one.
- A law suite entry per rule above, in `crates/musa-compiler/tests/suite/`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test -p musa-compiler --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

And the one-line proof, which must go from a refusal to a value:

```sh
printf 'library {\n  record Cell<A> { at: Nat; value: A; }\n  let one: Cell<Nat> = Cell { at = 0, value = 1 };\n}\n' > /tmp/cell.musa
cargo run -q -p musa -- check /tmp/cell.musa
```

`cargo nextest run --run-ignored all` and `cargo insta test --workspace` carry the reds prompt
[164](164-builtin-collapse.md)'s Check enumerates and [166b](166b-per-context-memo-stamp.md) owns; a *new* red under
either is this prompt's.

Commit as `Let a parameterized record be constructed`.

## Stop

- No headless record literal. `{ at = 0 }` with the head inferred from the goal is a surface addition, `01-surface.md`
  §1.2 does not write one, and this prompt is a defect fix.
- No change to what `Cell<Nat>` means as a *type*. The parameter stays explicit there, and
  `a_parameterized_record_puts_its_parameter_on_the_family` stays exactly as it is.
- No meta insertion in `check_type`, for the reason the Design gives.
- No new record feature: no positional construction, no field punning beyond the `Pending { read }` shorthand that §1.2
  already writes, no `..`.
- No use of the new construction outside `stdlib/src/pitch.musa` and the laws. Prompt 164 is where the algebra reaches
  the musical domains.
