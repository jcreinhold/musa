# The style guide

Layout is the formatter's. The style guide owns everything layout cannot say: the choices that are *spelled* correctly
and still mislead the player who reads them. Its rules, in short:

1. **A name is a promise.** A `motif`, `fragment`, `part`, or `patch` that nothing uses is not an abstraction; it is a
   rumour of one. Delete it, or use it. (A named `bar` is the exception: it plays where it stands, so its name is an
   address, not a promise.)
2. **A marking changes something.** A tempo, meter, or key marking that states the value already in force reads as an
   event and is none. If it is there to reassure, write a comment, which says honestly that it changes nothing.
3. **A change arrives somewhere.** A gradual tempo change names its destination or its words; a number and a distance
   with no destination is a clock pretending the arrival is knowable, and the compiler rejects it.
4. **Say it once.** Two identical bars is an accident of phrasing; three is a motif that has not been named yet. The
   cost of the copy is not the typing but the edit: change one and the others are silently wrong.
5. **A waiver lives next to the sin.** A lint can be wrong about a spot. The waiver is written in the source, directly
   above the construct it waives, naming the codes it suppresses. There is no project-level switch and no configuration
   file, on purpose: a standard that can be switched off silently is a rumour of a standard.

```musa
// musa:allow(unused-material) — kept for the B section, which is not written yet
motif answer() {
    g4/4; a4/4;
}
```

The machine-checkable subset is enforced by lints; each rule names the diagnostic code that enforces it — see
[Lint codes](../reference/lints.md). When a rule and its lint disagree, the guide is the authority and the lint is too
coarse: repair the lint, do not silence it.

The full guide is `docs/style-guide.md` in the repository. It is prose first and machinery second.
