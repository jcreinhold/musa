# The musa Style Guide

Layout is the formatter's (roadmap §11): indentation, statement-per-line, blank lines, and comment attachment are
decided by `musa format`, and a hand that adjusts them is wasting itself. This guide owns everything layout cannot say —
the choices that are *spelled* correctly and still mislead the player who reads them.

It is prose first and machinery second. The lint pass enforces the machine-checkable subset, and each rule below names
the diagnostic code that enforces it; a rule that cannot be checked without guessing stays advice. When the two
disagree, the guide is the authority and the rule is too coarse — say so by repairing the rule, not by silencing it.

## 1. A name is a promise

Every declaration that takes a name — `motif`, `fragment`, `part`, `voice`, `patch` — spends the reader's attention on
the promise that the name will be spoken again. A motif nobody uses is not an abstraction; it is a rumour of one, and
the reader hunts the score for a use that does not exist. Delete it, or use it.

The named `bar` is the exception that proves the reading: it plays where it stands, so its name is an *address* — for
edit sites and provenance — not a promise of reuse. An unused one costs nothing.

The same holds in the studio one layer down: a `patch` is wiring, and wiring no `assign` connects to a part is a cable
that ends in the air. It costs DSP to build and silence to hear.

Enforced: `unused-material` (a `motif` or `fragment` declared and never used), `unassigned-patch`.

## 2. A marking changes something

A tempo, meter, or key marking is a *change*, written where it happens. A marking that states the value already in force
reads as an event and is none: the player marks their part, the conductor glances, the engraver spends a system — all
for a statement that was already true. If the marking is there to reassure, that reassurance belongs in a comment, which
says honestly that it changes nothing.

Enforced: `redundant-marking`.

## 3. A change arrives somewhere

A gradual tempo change has three honest spellings and one dishonest one:

- `tempo 1/4 = 72 to 48 over 2/1 "rit.";` — from here to there, over this far. Everything named.
- `tempo "poco rit." over 1/1;` — a word and a distance, the speed left to the player, which is what a worded marking
  has always meant.
- `tempo 1/4 = 72 over 2/1;` — a number and a distance but no destination. The player is told to move and not where to
  arrive, and unlike the worded form there is a clock on the stand pretending the arrival is knowable. Name the arrival,
  or write the word.

Enforced by the compiler itself: the third spelling is an error (`this gradual tempo change goes nowhere`), not a lint —
lints warn on what compiles, and this does not.

## 4. Say it once

Two identical bars is an accident of phrasing; three is a motif that has not been named yet. The cost of the copy is not
the typing but the edit: change one and the others are now wrong in a way nothing will flag, because each still spells
fine. A `motif` makes the repetition a fact the compiler can check and the editor can rename, fold, and jump through.

This is not an argument against repetition — music is repetition — but for writing it as repetition: `use`, `repeat`,
and the transformer ladder are the spellings that keep the repetition true under edit.

Enforced: `copied-bars` (three or more identical bars in one voice).

## 5. A waiver lives next to the sin

A lint can be wrong about a particular spot — a sketch that keeps an unused motif on purpose, a study in repeated bars.
The waiver is written where the reader will look for it, in the source, directly above the construct it waives:

```musa
// musa:allow(unused-material) — kept for the B section, which is not written yet
motif answer() {
    g4 q; a4 q;
}
```

`musa:allow` takes one or more diagnostic codes, comma-separated. It suppresses exactly those codes on the construct
whose leading comment it is — nothing else, and nothing further away. A waiver with no code is no waiver, and a waived
construct that would not have fired is left for the reader to notice and remove.

There is no project-level switch and no configuration file, on purpose: the source is canonical (roadmap §3), and a
standard that can be switched off silently is a rumour of a standard.

## 6. Candidate vocabulary says which layer it means

**Candidate rule for prompts 93–168; it becomes governing only with prompt 169.** The additions in
`docs/rules/language/` keep the musician-facing word when it names a musical intention and the technical word when the
author has deliberately entered an implementation block.

Write `expression`, `emphasis`, `separation`, `brightness`, `sustain`, and `phrase` in profiles. Do not spell those as
gain, velocity, gate, cutoff, release time, or envelope: the latter are possible instrument realizations, not meanings
of score marks. A profile never names a graph path. Within `implementation graph`, `oscillator`, `envelope`, `lowpass`,
`resonance`, typed ports, and physical units are honest and documented terms; hiding them behind vague musical words
would make the advanced surface less comprehensible, not more.

Likewise, `Scale`, `Key`, `ChordClass`, `Voicing`, `NoteName`, and `Pc12` are separate names because they preserve
separate choices — and a spelled thing is a *name*, which is why the last two are not one type. Prefer the readable
block form `in scale ... { ... }` to an unexplained context operator. Use `template`/`make ... as ...` only for
identity-bearing declarations; use `fn` for values and `motif` only for a music-producing function that a musician would
recognize as reusable material.

A sound binding should read aloud: `sound solo_strings using lyrical;` inside the violin part, then
`send violin -> concert_hall at -12 dB;` when a shared room is wanted. Hover and the handbook must define `sound`,
`assign`, `send`, `room`, `bus`, `route`, every built-in processor, unit, control, and unsupported-feature policy where
the user encounters it. Abbreviation that saves characters but hides the concept is rejected: the filter parameter is
`resonance`, not `q`.
