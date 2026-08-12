# Review of the selected calculus

**Reviewed files:** `05-selected-calculus.md` at SHA-256
`1bd502ff100326ed3e24b12e923303878201bea92d224365db8d2fd1eb6b1eb0` and `06-proof-outline.md` at SHA-256
`91719842d386a96a073481cd2e9169d054c95dec43c6b4d7871da9949d0aef61`.

## Findings

### High

1. **A source primitive can hide a looping function inside data.**
   - **Location**: Proof assumption A1 and Theorem 2.5.
   - **Type**: false termination theorem under the stated assumption.
   - **Problem**: A1 says that a primitive does not return a source function. It does not forbid a function inside its
     result. Declare:

     ```text
     Box = Box(Unit -> Unit)
     p : Unit -> Box
     ```

     Let `p(())` take one primitive step to

     ```text
     Box(fn x => let Box(f) = p(()) in f(x))
     ```

     The result is finite, well typed, and not itself a function, so it meets A1 literally. But this closed term never
     terminates:

     ```text
     let Box(f) = p(()) in f(())
     ```

     After opening the box and applying the function, it returns to the same computation.
   - **Why it matters**: Theorem 2.5 is the boundary between finite source evaluation and a machine that runs later. The
     theorem is false as written.
   - **Suggested repair**: make foreign source primitives first order. Their argument and result types must contain no
     source function at any depth. Machine descriptions returned by a primitive must also contain no source closure.
     Keep higher-order finite operations such as `map_events` inside the proved source calculus.

2. **Primitive batching does not automatically preserve feedback.**
   - **Location**: Theorem 5.5 and its proof.
   - **Type**: false composition argument.
   - **Problem**: in `feedback(initial,m)`, the feedback input at frame `j + 1` is an output of `m` at frame `j`. A
     batch call on `m` cannot be given all `n` feedback inputs before those earlier outputs have been computed.
     Replacing each primitive's repeated calls by a primitive batch therefore does not justify a batch call for the
     enclosing feedback machine.
   - **Why it matters**: the proposed optimization can change the state and sound of an accepted feedback machine. This
     is the same class of callback-partition bug the one-frame semantics was meant to remove.
   - **Suggested repair**: state the batch contract for the whole machine being batched. A feedback-free machine can
     inherit valid primitive batches by induction through `connect` and `beside`. A feedback machine must keep the
     frame-by-frame loop unless a specialized whole-machine batch is separately proved equal to those repeated steps.

### Medium

1. **Scheduling need not preserve `together` under the stated scheduler assumption.**
   - **Location**: Selected calculus §7.2, A5, and Theorem 5.4.
   - **Type**: missing hypothesis.
   - **Problem**: A5 requires a finite assignment table but does not say that an occurrence's assignment is independent
     of other occurrences. A legal policy could shift one of two same-frame attacks to avoid a collision. Scheduling
     `together(x,y)` would then differ from scheduling each track alone and merging the results.
   - **Why it matters**: this theorem is the claimed algebraic connection from finite tracks to running sources.
   - **Suggested repair**: add the exact premise that boundary assignment is occurrence-local and same-frame merging
     only orders messages. For a policy that drops or shifts events based on neighbors, withdraw the law and record the
     whole-track decision instead.

2. **Resource charging is named but not defined.**
   - **Location**: Selected calculus §2.1 and Theorems 2.3–2.5.
   - **Type**: missing operational rule.
   - **Problem**: the source evaluator may return a resource error, but no fixed charge function or meter state appears
     in the transition rules. A wall-clock or allocation-dependent charge could make evaluation nondeterministic.
   - **Why it matters**: progress and determinism include the error boundary.
   - **Suggested repair**: define a versioned integer cost for every reduction and constructed value. Evaluation state
     carries the remaining integer budget. No cost may read elapsed time or allocator behavior.

### Low

1. **The feedback state equation uses the wrong name.**
   - **Location**: Selected calculus §5.3.
   - **Problem**: `State(feedback(f,m))` should read `State(feedback(initial,m))`.

## Verdict

- **Decision**: Incorrect.
- **Basis**: the boxed-function counterexample refutes source termination, and primitive batch replacement is not sound
  through stored feedback. Both are exact counterexamples to displayed claims.
- **Limits**: this review checks the paper calculus, not compiler conformance, real-time performance, or the musical
  adequacy of a future theory library.

## Clean passes

- `EventTrack` closure, unequal-length `together`, multiset handling, and the stated track laws pass.
- Machine state and step are now defined structurally; the K2 port-scheduling counterexample does not apply.
- Explicit stored feedback has a first output and is causal.
- Explicit seeds remain fixed under regrouping, so connection associativity is sound.
- One-frame audio meaning is coherent.
- Dynamic sample-rate checking avoids value-dependent types without hiding the rate.
- Opaque event handles and equality up to consistent renaming repair the duplicate-event problem, provided instrument
  primitives obey the stated handle rule.

