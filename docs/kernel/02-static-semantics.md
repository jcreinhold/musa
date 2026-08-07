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

## K3 — Ambient extension

`extend(d, E)` to a new extent `e` requires `d ≤ e`. Extension never adds, moves, or removes an occurrence (§9); a
request to shrink is an error, not an implicit crop. Cropping is `restrict` — an observation, not a mutation
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
- Every payload type used with normalization, semantic equality, or serialization must provide a **canonical
  serialization** (`05-normalization.md`, N3): a deterministic, total, injective-on-values text form.

## K6 — Multiset discipline

Occurrences form a finite **multiset**, not a set (§6): two occurrences identical in span and payload are two
occurrences. No operation in the kernel may deduplicate them. Two performers playing the same note must not collapse
merely because all visible values coincide.

## Error surface

All static violations are reported as `KernelError` values naming the rule and the offending data (extent, span, or
reference). The kernel has no warnings: a construct is either well-formed or rejected.
