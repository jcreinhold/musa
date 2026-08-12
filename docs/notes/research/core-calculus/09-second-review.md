# Second review of the selected calculus

**Reviewed files:** `05-selected-calculus.md` at SHA-256
`6d118e2f27cf981bc7c031aa47ca44ce3f094a2096a7dd0e687697563e0f7090` and `06-proof-outline.md` at SHA-256
`3e6ed9ee9fbe13ed53f3dd1ecec6ee53b8e5bde3f18fffeead420d97cdb744db`.

## Findings

### High

1. **A polymorphic payload can still become a source function.**
   - **Location**: selected calculus §2 and §5.1; proof assumption A1 and Theorem 2.1.
   - **Type**: unenforced premise / type-system gap.
   - **Problem**: A1 forbids functions inside a primitive argument or result, but the type system has only one class of
     type variable. The displayed generic scheduler has an argument `EventTrack<C,A>`. Ordinary Hindley–Milner inference
     permits `A = Unit -> Unit`. Nothing in the formation rules rejects

     ```text
     EventTrack<PerformedTime, Unit -> Unit>
     ```

     or the resulting `EventBatch<Unit -> Unit>` machine port. With an empty track, the payload variable may even remain
     generalized and be instantiated later. The prose restriction on foreign calls therefore has no checking rule.
   - **Why it matters**: the boxed-loop repair and the boundary between compile-time closures and running data can both
     be bypassed through a type variable.
   - **Suggested repair**: use two simple type classes. Every source function is a value. Only transitively
     function-free values are **storable data**. Give payload variables a declared class. Require storable data in event
     tracks, primitive configurations, machine ports, feedback values, and foreign source primitive arguments and
     results. Unification must preserve the class. This is ordinary kinded Hindley–Milner inference, not a dependent
     type system or public type class mechanism.

### Medium

1. **Resource failure has no type-preserving configuration.**
   - **Location**: selected calculus §2.1; Theorems 2.2 and 2.3.
   - **Type**: incomplete operational rule.
   - **Problem**: a term `e : A` may reduce to `ResourceError`, but the grammar gives no error term of type `A` and no
     typed evaluation configuration. The preservation proof says only that the error “preserves the declared result
     boundary.”
   - **Why it matters**: preservation and progress are not literal theorems for the displayed syntax.
   - **Suggested repair**: type evaluation configurations by the source result type. A terminal configuration is either
     `done(v)` with `v : A` or `failed(ResourceError)` while the configuration remains indexed by `A`. Ordinary
     expression preservation then concerns only expression-to-expression steps.

## Verdict

- **Decision**: Incomplete.
- **Basis**: the main semantic repairs pass, but the first-order primitive rule lacks a static enforcement mechanism and
  the error boundary lacks an exact typed object.
- **Limits**: this review did not repeat the current-code audit or test musical libraries, because neither finding
  depends on them.

## Clean passes

- The first review's boxed-loop, feedback batching, occurrence-locality, fixed-charge, and naming findings are repaired.
- The event-track and structural machine proofs remain sound under their assumptions.
- No dependent type or effect system is needed for either new repair.

