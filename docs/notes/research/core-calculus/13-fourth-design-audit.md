# Fourth design audit

**Status: findings found after the third repair. This document does not set Musa's rules.**

## Findings

### Medium

1. **A “time map” could reverse two different boundaries.**
   - **Location**: selected calculus §7.1.
   - **Problem**: the draft checked each event's start against its own end, but did not compare boundaries from
     different events. A supposed map could send beat 2 before beat 1 while still leaving each note with a nonnegative
     duration.
   - **Repair**: scheduling sorts the finite set of queried source boundaries and requires their exact physical times
     and assigned frames to be nondecreasing. Local anticipation belongs in the performed gesture track, not in a map
     that reverses time.

2. **The end-frame convention was unstated.**
   - **Location**: event-track definition and boundary-message definition.
   - **Problem**: if an event begins at frame 10 and ends at frame 20, the draft did not say whether frame 20 belongs to
     the event. Two instruments could follow different rules while both claiming conformance.
   - **Repair**: a positive event occupies the half-open span `[start,end)`. The batch for frame `j` is read before the
     instrument emits frame `j`. Thus `Begin` affects the start frame and `End` prevents the event from sounding on the
     end frame. A point event is a single message at its assigned frame.

3. **The overlay theorem did not require all compared schedules to succeed.**
   - A scheduler returns `Result`. One separate track or the combined overlay may fail a checked conversion. The repair
     compares machines only when the combined call and both separate calls succeed.

## Verdict

- **Decision**: Incomplete before repair.
- **Basis**: these gaps permit two different audio histories from what appears to be one scheduled track.
- **Clean passes**: neither repair changes event-track algebra, machine typing, feedback, or source normalization.
