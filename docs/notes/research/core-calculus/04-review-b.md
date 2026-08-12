# Review of Draft B

## Findings

### High

1. **`loop` may need an output before the first output exists.**
   - **Location**: Draft B §3.
   - **Type**: undefined execution.
   - **Problem**: let the feedback type be `Bool`, and let the machine copy its feedback input to its feedback output.
     The first step of `loop(m)` asks for the feedback input, but no earlier step supplied one. A negating machine is
     worse: it asks for a Boolean equal to its own negation.
   - **Why it matters**: a closed well-typed machine can fail before producing one frame.
   - **Suggested repair**: feedback must take an initial value and must store the returned feedback value for the next
     step. It must never connect a current output to a current input.

2. **`lift` moves arbitrary source work into the audio callback.**
   - **Location**: Draft B §3.
   - **Type**: wrong execution boundary.
   - **Problem**: the source language is total, but a total function can still traverse a million-item list, allocate a
     large result, or build text. `lift(large_sort)` would be a well-typed audio machine even though it cannot meet the
     callback rules. Total does not mean fast or allocation-free.
   - **Why it matters**: the type promises a runnable machine without establishing the property that makes it safe to
     run.
   - **Suggested repair**: remove public `lift`. Machines should be built from registered step units plus fixed wiring.
     User-defined real-time code can be added later only with a bounded step language and a proved resource check.

3. **`AudioStep` is not defined precisely enough.**
   - **Location**: Draft B §4.
   - **Type**: missing contract.
   - **Problem**: if one step means “one callback block,” the result may change when the host requests one block of 512
     frames instead of four blocks of 128. Feedback delay, event timing, and envelopes can all change. The current Musa
     audio implementation has this caller-chunk sensitivity in its feedback behavior.
   - **Why it matters**: the same prepared machine can denote different audio on different hosts.
   - **Suggested repair**: give audio a one-frame reference semantics. The engine may batch frames only when the batched
     implementation is proved to match repeated frame steps.

4. **The machine description has no exact operational meaning.**
   - **Location**: Draft B §3.
   - **Type**: missing definition.
   - **Problem**: “may have private state” does not say what the state is for `connect`, `beside`, or feedback. The laws
     are therefore wishes, not theorems.
   - **Why it matters**: the earlier K2 process draft failed for this exact reason: its accepted wiring could not be
     scheduled through the primitive interface it actually supplied.
   - **Suggested repair**: define every machine as an initial state and a total step function. Define the state and step
     of each constructor by structural recursion on the finite machine description.

### Medium

1. **The scheduler can silently round time.**
   - **Location**: Draft B §4.
   - **Problem**: performed positions are exact rationals, while audio frames are integers. `schedule` must return its
     rounding and collision decisions or an error. The conversion is not definitional equality.

2. **The tag `K` prevents only named step mismatches.**
   - Two machines can share `AudioStep` yet expect different channel layouts or event shapes. Those must appear in `A`
     and `B`, and connection must require exact type equality.

3. **Machine equality is unstated.**
   - Structural equality of two descriptions is decidable. Equality of all their future outputs is generally not. The
     design must not use the latter as a compiler conversion rule or cache key.

## Verdict

- **Decision**: Incomplete and unsafe as written.
- **Basis**: feedback lacks an initial stored value, public `lift` admits unsuitable callback work, and the state and
  step of composed machines are undefined.
- **Limits**: the finite `EventTrack` algebra and the score-to-scheduler-to-instrument shape survive.

## Clean passes

- `EventTrack` handles unequal lengths without padding.
- A rest remains payload data; `empty(d)` inserts no fact.
- Meter is not part of exact placement.
- A scheduler can connect a finite performed track to an audio machine.
- `connect` and `beside` are the right two wiring operations.
- An explicit mixer, rather than `beside`, performs audio combination.

