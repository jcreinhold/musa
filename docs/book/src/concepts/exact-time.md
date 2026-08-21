# Exact time

Musical time in musa is rational. Positions form the abelian group `(ℚ, +, 0)`; durations form the ordered commutative
monoid `(ℚ≥0, +, 0)`. Floats never represent symbolic musical time.

This is why the language writes durations as fractions of a whole note — `1/4`, `3/8`, `1/12` — rather than as decimal
seconds or ticks. A triplet eighth is exactly `1/12`, not a rounding of `0.0833…`. Tuplets, meter changes, and nested
repeats compose without drift because addition of rationals is exact.

## Two clocks, kept apart

Physical seconds are a separate domain, introduced by performance realization. Tempo is the map between them: it
converts a position in musical time to a position in seconds. This is what makes tempo different from key or meter —
those are facts the score states; tempo is a reading of the score, and a piece can carry more than one reading.

The consequences run one way. A beat-fitted audio clip follows tempo, because its duration is musical. A fixed-media cue
keeps its recorded seconds, because a recording knows nothing of the beat; musa never manufactures a musical extent for
it.

## Where floats are allowed

Floats appear only at the performance and DSP edge: sample positions, control voltages, audio buffers. They are the
realization of exact values, computed once at the boundary, and they never flow back into the score. The event track,
the snapshots, and the exports stay rational end to end.
