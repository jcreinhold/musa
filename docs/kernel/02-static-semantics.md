# 02 — Static Semantics

Well-formedness rules for kernel compositions and kernel files. Everything here is checkable without evaluating anything
musical — the kernel never inspects payload *meaning* (course correction §12); it checks shapes and bounds.

## K1 — Occurrence bounds

Every occurrence `(s, e, a)` in a timeline of extent `d` must satisfy:

```text
0 ≤ s ≤ e ≤ d
```

- `s`, `e`, `d` are exact rationals (`ℚ`), with `d ∈ ℚ≥0` by construction.
- Zero-length occurrences (`s = e`) are **well-formed**. Their meaning is a payload-type question (a percussive hit
  might be modeled as a point), not a kernel question.
- Violations are construction-time errors (`KernelError`), never silently clamped: clamping would rewrite where an
  occurrence began, which is exactly the lie restriction is designed to avoid (§17).

## K2 — Time domain

- Positions and durations are exact rationals. No floating-point value may enter the kernel (§4).
- Timeline extents are non-negative. The empty timeline at extent `d` is `(d, ∅)` — a perfectly good value, and the
  identity of `overlay` at fixed duration (§8).
- Scaling factors are **positive** rationals (`ℚ>0`); zero or negative scaling is a construction error
  (`03-denotational-semantics.md`, D5).

## K3 — Ambient extension *(struck: prompt 37)*

There was a rule here about `extend(d, E)` refusing to shrink. The operation was removed at prompt 37 (nothing called
it), and with it the error. Cropping was never extension anyway: it is `restrict` — an observation, not a mutation
(`03-denotational-semantics.md`, D6).

## K4 — Reference resolution and acyclicity

In a kernel file (or any HIR that names compositions):

- Every `composition-name` referenced in a `composition-expression` must be declared in the same file/scope.
- The reference graph must be **acyclic**. There is no recursion in the kernel (§32); a cycle is rejected, not lazily
  tolerated.
- All composition expressions in one `sequence` or `overlay` must share the same payload type. The kernel is parametric
  in `A`, not polymorphic per composition.

## K5 — Payload schemas

Payload declarations are checked by their own modules (the score payload lives with the score adapter), but the
kernel-side contract is:

- Field names are unique within a payload declaration.
- Field types resolve (`text`, `rational`, `integer`, `bool`, or a previously-declared payload type — no forward
  references, hence no recursion).
- A payload **value** must match its declared schema exactly: every field present, no extra fields.
- Every payload type used with normalization, semantic equality, or semantic identity must satisfy the admission rule
  (`12-payload-admission.md`): a stable owner id, a quotient version, and a deterministic total key complete for the
  admitted equality classes. It need not distinguish raw fields which the declared quotient intentionally omits.

## K6 — Multiset discipline

Occurrences form a finite **multiset**, not a set (§6): two occurrences identical in span and payload are two
occurrences. No operation in the kernel may deduplicate them. Two performers playing the same note must not collapse
merely because all visible values coincide.

## K7 — Term well-formedness *(specified prompt 46, implemented prompt 47)*

Rules for the term calculus of `10-term-calculus.md`. A reference's mark is not checked — it is an opaque string the
kernel does not interpret (T6), so there is nothing here that could be wrong about it. They are checkable on the term
alone, without evaluating it.

- **Scoping.** Every name occurring in a term is bound by an enclosing `let`. A term with a free name is rejected, not
  resolved against an ambient environment: a term means one thing on its own or it does not mean anything.
- **No shadowing.** `let x = t in (let x = u in v)` is rejected. Nothing needs it, alpha-renaming is not a burden a file
  format should impose on its readers, and forbidding it makes substitution textual, which is what lets T2 be stated
  without a capture-avoidance apparatus.
- **Payload uniformity.** All arguments of one `seq` or `over` share a payload type, as K4 already requires of
  composition expressions. `let` binds a term of one payload type; a name's type is its bound term's.
- **Arity.** `seq` and `over` take at least one argument. Zero arguments would need a unit, and the two units differ
  (`(0, ∅)` for `seq`, `(d, ∅)` at a fixed `d` for `over`, L2/L6) — so the empty case is written as the literal it is,
  not inferred.
- **Windows and factors.** `restrict [i, j)` requires `i ≤ j`; `scale r` requires `r ∈ ℚ>0` (K2). A window is *not*
  required to lie inside the extent: restriction is total (D6, L17), and a window past the end observes nothing.
- **Literals.** Every `timeline` literal satisfies K1.
- **Acyclicity comes free.** `let` scopes over its body only, so a name cannot refer to itself and the reference graph
  is a tree by construction. K4's acyclicity rule is what this replaces for terms.

A term satisfying these rules and containing no free names is **closed and well-formed**, which is the precondition of
every theorem in `10-term-calculus.md`.

**Only two of these rules are ever checked.** Prompt 47's implementation makes the rest unrepresentable: `Term` is
opaque and built through constructors, so a non-positive `scale` factor, a negative `shift`, an empty `seq` or `over`,
and a disordered window are rejected where they are written and never become terms. Payload uniformity is the type
parameter. Acyclicity is free, as above. What is left is the two rules that are **not local to one node** — a free name
and a shadowed one — because a reference is built before the binder that encloses it. Those are what `Term::check`
answers, and a term that passes it evaluates (T4).

## Error surface

All static violations are reported as `KernelError` values naming the rule and the offending data (extent, span, or
reference). The kernel has no warnings: a construct is either well-formed or rejected.
