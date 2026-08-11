# Claims and readings

Two things a composer might want the software to do about theory look similar and are not:

- **A claim.** "This passage is one 3/4 measure." "These four notes are a C major triad and nothing else." Either it is
  true of what you wrote or it is not, and if it is not, you want to be told which note refuses.
- **A reading.** "This looks like a modulation to G, with the A minor chord as the pivot." A competent musician might
  read it another way and not be wrong.

Musa keeps these apart because conflating them is how software starts telling people their music is invalid.

## 1. Assertions: things you claim

An `assert` wraps a passage and states something about it. From `examples/theory-assertions.musa`:

```musa
// The claim `bar { ... }` has been making since it was
// introduced, written out in full. `fills_meter()` reads the
// meter in force where it stands — 4/4 here, because this
// piece writes no `meter` — and proves the passage is one
// measure of it.
assert fills_meter() {
    c4/4
    d4/4
    e4/4
    f4/4
}
```

An assertion adds no note, no time, and no barline. Delete it and the piece is unchanged. What it adds is a proof
obligation, checked after the passage has been instantiated in its real context, and one line of provenance under the
notes it covers.

The family is fixed — there are five kinds, and they are not an extension point:

```musa
// Membership is about spelling, not about sound. `f#4` would
// fail this claim as F-sharp: the compiler reports it, and
// never reads it as G-flat or as a raised fourth degree.
assert pitches_in(scale c major) {
```

```musa
// Three policies, three different claims. `exactly` is set
// equality on pitch classes: every member sounds, and nothing
// else does. Doubling is invisible to all three, because
// which member is doubled is a fact about the voicing.
assert realizes(chord c major, exactly) {
    [c3 e3 g3]/1
}
```

```musa
// `may_omit` lets a member go missing. The fifth of a
// dominant seventh is the one that usually does.
assert realizes(chord g dominant7, may_omit) {
    [g3 b3 f4]/1
}
```

```musa
// Four notes wherever anything sounds. Silence is not a
// violation: a moment where nothing sounds is no voices, not
// a wrong number of them.
assert voices(4) {
    [c3 g3 c4 e4]/1
}
```

```musa
// The ranges are given lowest voice first, and the passage's
// own notes say which voice is which — the compiler has no
// notion of "the alto". These four are the SATB ranges, bass
// to soprano.
assert within_ranges([(f2, d4), (c3, g4), (g3, d5), (c4, g5)]) {
    [c3 g3 c4 e4]/1
}
```

Style rules are asserted the same way, and are style-indexed rather than universal — the name of the rule set is part of
the claim, so nothing becomes a global law by being locally checkable. From
`examples/broken/claim-parallel-fifths.musa`:

```musa
assert follows(satb_parallel_perfects) {
    [c3 g3 c4 e4]/1
    [d3 a3 d4 f4]/1
}
```

When a claim fails, the diagnostic names the passage and the note. `examples/broken/claim-not-a-measure.musa` is a
quarter short:

```musa
assert fills_meter() {
    c4/4 d4/4
}
```

```sh
musa check examples/broken/claim-not-a-measure.musa
```

Every file under `examples/broken/` is a rendered-diagnostic golden: what you see in the terminal is compared, whole,
against a checked-in snapshot, so a help line cannot quietly stop matching its message.

## 2. Analyses: things software reads

An analysis observes a compiled score and reports what it saw. It never constructs music, never rewrites source, and
never becomes a diagnostic. It is asked for explicitly:

```sh
musa analyze examples/analysis/pivot-ambiguity.musa --kind tonal
```

A report carries the method it used, the assumptions it made, the findings, and the evidence for each one. A finding is
a **fact**, a **candidate**, or a **conflict**, and most interesting ones are candidates.
`examples/analysis/pivot-ambiguity.musa` is built to make that unavoidable: an A minor triad stands between a phrase in
C and a phrase in G, and it is `vi` in one and `ii` in the other.

```musa
voice upper {
    phrase "antecedent" {
        | [e4 g4 c5]/1
        | [f4 a4 c5]/1
        | [e4 a4 c5]/1
    }
    phrase "consequent" {
        | [f#4 a4 d5]/1
        | [g4 b4 d5]/1
        | [e4 a4 c5]/1
        | [f#4 a4 c5]/1
        | [b4 d5 g5]/1
    }
}
```

The report says both readings, with the OMT criteria attached to each, and resolves neither. In the workbench the same
report appears under the source as findings with their evidence — never as red marks on the score, because a debatable
theoretical reading is not a syntax error (`../../interface/08-elaboration.md`).

The exit code of `musa analyze` says whether the *request* could be answered, never what the report contains. A window
with nothing in it is a window with nothing in it.

The kinds that ship, what each one's abstract domain is, and the admission rule a new kind must pass are in
[`../07-analysis.md`](../07-analysis.md).

## 3. The kernel quote

Under the surface language is a small temporal kernel: exact rational time, typed occurrences, and three combinators.
Musa normally writes kernel terms for you. `kernel { ... }` is the one place you write them yourself, for placements no
surface constructor spells. From `examples/kernel-splice.musa`:

```musa
let assembled: Music = kernel Timeline[ScoreFact] {
    let subj = ${subject} in
    overlay {
        subj;
```

`${subject}` is a **hole**: ordinary Musa music, instantiated once, under this piece's context, and bound to a fresh
name the compiler chooses, so nothing written inside the quote can capture it.

What a quote keeps is exact time and payload typing. What it gives up is every theoretical guarantee that depends on
*where* facts land — bar alignment, scale membership, chord realization — because raw placement is precisely the freedom
to land them elsewhere. The way to get those back is to assert around the finished quote:

```musa
assert pitches_in(scale c major) {
    use assembled;
}
```

Every fact that leaves a quote records a `splice` step in its expansion path, so the Origin view can say a note came
through a quotation and point at the quote — while refusing to synthesize an edit inside it, because the facts there
were assembled rather than written.

Use it rarely. If a surface constructor spells what you want, that one is better: it keeps the theory checks, and it
reads like music.
