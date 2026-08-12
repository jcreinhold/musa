# Review of Draft A

## Findings

### High

1. **A written note is not a value at every rational instant.**
   - **Location**: Draft A §3.
   - **Type**: wrong object.
   - **Problem**: written musical time is dense: there are infinitely many rational positions between two beats. A
     sparse score has finitely many notes and markings, not an `Option<Note>` stored at every rational. Replacing the
     function with a finite event set repairs notation but no longer represents dense audio in the same way.
   - **Why it matters**: the one `Flow` representation is already two representations hidden behind one name.
   - **Suggested repair**: keep sparse placed events and dense sampled audio distinct.

2. **`after` has no honest meaning for an unbounded audio flow.**
   - **Location**: Draft A §1 and §3.
   - **Type**: undefined operation.
   - **Problem**: shifting `y` by the length of `x` requires `x` to have a finite length. A live input, oscillator, or
     room response need not end. The term `after(live_microphone, chorus)` therefore has no start time for `chorus`.
   - **Why it matters**: the operator was meant to cover both musical succession and audio. It covers only bounded
     values.
   - **Suggested repair**: use temporal succession only on finite event collections. Use typed wiring for running audio.

3. **`together` hides the musical operation.**
   - **Location**: Draft A §1.
   - **Type**: false unification.
   - **Problem**: putting two note collections together preserves both occurrences. Putting two audio samples together
     might add, average, choose channels, saturate, or route them separately. No one pointwise operation is right. A
     mixer must say which audio operation it performs.
   - **Why it matters**: the core would make an audio choice while pretending to provide neutral structure.
   - **Suggested repair**: side-by-side wiring should preserve two outputs. Mixing should be an explicit audio unit.

4. **The feedback rule admits a value that depends on itself now.**
   - **Location**: Draft A §1 and §3.
   - **Type**: undefined fixed point.
   - **Problem**: take `f(x) = not(x)` at one Boolean step. `feedback(f)` would require `x = not(x)`, which has no
     value. A continuous audio loop can have the same algebraic loop.
   - **Why it matters**: a well-typed flow can fail to produce its first output.
   - **Suggested repair**: feedback must read a stored value from the previous step and must receive an initial value.

### Medium

1. **`connect` treats time-varying functions as ordinary data.**
   - A first-class function at every audio sample gives the source language the power to replace the running program at
     every step. Musa has no need for this, and it makes memory and time use hard to bound. A finite machine description
     built from fixed units is smaller and easier to check.

2. **One time parameter does not solve clock conversion.**
   - Written beats, performed time, interaction turns, and audio frames still need explicit maps. Changing the name to
     `C` does not supply them.

## Verdict

- **Decision**: Incorrect.
- **Basis**: the live-audio counterexample makes `after` undefined, Boolean negation makes feedback fail at the first
  step, and sparse notation and dense audio require different finite representations.
- **Limits**: causal stream functions remain a sound model for running systems. The review rejects their use as the one
  representation of every musical object.

## Clean passes

- A finite description may denote an unbounded stream.
- Series and side-by-side connection are the right basic operations for running units.
- Taking a bounded observation of a run is useful.
- Written and audio time must be related explicitly, not silently identified.

