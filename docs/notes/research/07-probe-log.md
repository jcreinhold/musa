# Probe log

Append-only, on the model of `docs/notes/research/kernel-hypothesis/06-evidence-log.md`. A probe that damages a claim
stays here next to the claim it damaged.

---

## P-1 — does the analysis code re-derive quotients by hand? (run)

[05-candidate-torsor.md](05-candidate-torsor.md) D-2 predicted: if the payload is a torsor with a tower of quotients but
the kernel offers one `canonical_key`, consumers needing coarser levels will re-derive them at the point of use. Run
against `crates/musa-compiler/src/analysis/` at commit `1b55fe5`.

**Verdict: partially confirmed** — *and then superseded by P-2 below, which refutes the rest of it. Read P-2 before
using anything here.* The searched surface was `analysis/` only; the abstraction P-1 reports as missing lives one
directory up.

### What is already named

`crates/musa-compiler/src/pitch.rs` exposes three levels of the tower as methods on `WrittenPitch`:

| Method | Tower level |
| --- | --- |
| `diatonic_height()` (`pitch.rs:168`) | position on the line of letters |
| `chromatic_height()` (`pitch.rs:175`) | chromatic pitch — the enharmonic quotient |
| `pitch_class()` (`pitch.rs:232`) | pitch class — the octave quotient |

Eight call sites across `cadence.rs`, `segment.rs`, and `voice_leading.rs` use `pitch_class()`. So the two most-used
quotients **are** named and shared, and D-2's "every coarser consumer must re-derive its quotient" is too strong. That
part is refuted.

### What is hand-rolled, and it is the interesting part

Two sites compute a quotient inline that has no name anywhere:

1. **`analysis/motion.rs:194` — `simple()`**, reducing an interval into one octave:

   ```rust
   let steps = interval.diatonic_steps.rem_euclid(7);
   let semitones = interval.semitones.rem_euclid(12);
   ```

   This is the octave quotient applied to the *interval group* rather than to the pitch torsor. The tower is named on
   `Pitch` and unnamed on `Interval` — but OMT `103-interval-class-vectors.md` treats interval class as a first-class
   object, so a consumer wants it.

2. **`analysis/voice_leading.rs:445` — `is_leading_tone()`**:

   ```rust
   pitch.diatonic_height().saturating_add(1).rem_euclid(7) == letter.rem_euclid(7)
       && pitch.chromatic_height().saturating_add(1).rem_euclid(12) == semitone.rem_euclid(12)
   ```

   This computes "is this pitch one step below the tonic" by reducing both coordinates against the key's tonic
   inline. **That is the scale-degree quotient — the orbit under translation-by-the-tonic — computed by hand.**

### Why the second site matters more than the first

[05-candidate-torsor.md](05-candidate-torsor.md) §5 ran the hard cases and recorded harmonic function as the one
"expected to fail and did not": scale degree is the orbit under translation-by-the-tonic, so tonal function sits in the
same tower as set class. P-1 finds that this is precisely the level the code computes inline, in two coordinates at
once, without naming it.

So the prediction and the finding meet at one point rather than everywhere: **the levels of the tower that the theory
names, the code names; the level the theory had to derive, the code open-codes.** That is weaker evidence than D-2
claimed and better-aimed evidence than D-2 asked for.

### Consequences

1. **D-2 is amended, not withdrawn.** Its claim is now: the tower is named where OMT names it, and unnamed where OMT
   states the equivalence without naming the construction — which is `docs/notes/research/kernel-hypothesis/01-atoms.md`
   §6's own observation ("the theory has been describing quotients by group actions in four chapters without a single
   name") showing up in the code.
2. **This does not yet justify a refactor.** Two sites is not a pattern, and course-correction §34 wants a consumer. The
   honest next step is not to introduce a quotient abstraction but to check whether a *third* consumer wants the
   scale-degree quotient — `analysis/tonal.rs` and `analysis/cadence.rs` are where to look.
3. **It does support T over the alternatives**, weakly: the code independently grew the shape T predicts, at the level T
   identified as the surprising one.

### Next probe

The one this suggests: does `analysis/tonal.rs` compute scale degree or harmonic function by hand as well? Three sites
would make it a pattern and would be the consumer §34 asks for. One site would leave it as two accidents.

---

## P-2 — the follow-up, which refutes P-1 (run)

Same commit. P-1's next probe, run immediately: does `analysis/tonal.rs` open-code the scale-degree quotient too?

**Verdict: D-2 is refuted outright. The quotient tower is already in the codebase, named in both directions, with a law
test asserting that a square in it commutes. P-1 concluded "the level the theory had to derive, the code open-codes",
and that is false — P-1 searched `analysis/` and the construction lives in `crates/musa-compiler/src/scale.rs`.**

### What is actually there

`scale.rs` carries the scale-degree level of the tower as real structure:

| Item | Role in the tower |
| --- | --- |
| `Degree` (`scale.rs:444`) | the quotient's index — an ordinal plus an alteration |
| `Scale::class(Degree)` (`scale.rs:321`) | section, into pitch classes |
| `Frame::pitch(Degree)` (`scale.rs:570`) | section, into spelled pitches with register |
| **`Frame::locate(WrittenPitch) -> Option<Degree>`** (`scale.rs:596`) | **the quotient map — the one P-1 said was missing** |

`locate`'s own doc comment reaches for canonicalization vocabulary without any prompting from this directory: the answer
"is always unaltered, which is what makes it the canonical representative the round trip returns." That is
[05](05-candidate-torsor.md) Proposition 6's *canonicalize-then-quotient* stated in a doc comment written before any of
these notes existed.

### The test that settles it

`scale.rs:685`, `a_classs_degree_is_its_framed_degree_with_the_register_forgotten`, asserts for every collection and 24
ordinals:

```rust
frame.scale().class(degree) == frame.pitch(degree).map(WrittenPitch::pitch_class)
```

Read structurally, that is: **the section into pitch classes equals the section into spelled pitches followed by the
octave quotient.** A commuting square between two levels of [05](05-candidate-torsor.md)'s tower, already a test, with
its name spelling out the commutation in English — "with the register forgotten."

### What this does to the candidates

1. **D-2 is withdrawn, not amended.** Both of its forms are now false. Its prediction was that the tower's absence would
   show up as duplicated normalization; the tower is present.
2. **This is better evidence for T than D-2 would have been.** D-2 was a prediction that the code would be *deficient*
   in a way T explains. What P-2 found is that the code independently grew the *structure* T describes — sections, a
   quotient map, a canonical representative, and a commuting square — with no category theory anywhere in its
   vocabulary. Convergent description by an unrelated author is corroboration that the description fits; it is not
   evidence that the description is the *motive*, and it must not be read as such.
3. **`analysis/tonal.rs` is exonerated.** `chord_at` (`tonal.rs:174`) searches ordinals rather than calling `locate`,
   but it is fitting a *chord* to a degree under a best-fit rule, not locating a pitch. Different problem; not a
   duplicated quotient.
4. **One real defect survives, and it is small.** `voice_leading.rs:445`'s `is_leading_tone` open-codes in two
   coordinates what `Frame::locate` already computes. That is ordinary duplication of an existing helper — a cleanup
   item, not a structural finding, and it should be a prompt rather than a note here.

### Method note

P-1 searched the directory where it expected the answer and reported a finding from that directory's silence. The
correct search surface was the crate. Recorded because this directory's standard of evidence is worth more than its
conclusions: **a probe scoped to where the defect was expected will find the defect.**

---

## P-3 — is the structured editor a consumer of the studio's core structure? (run)

[08](08-candidate-enriched.md) §6 case 5 asked the sharpest open question in that document: `docs/rules/desktop/` and
`AGENTS.md` make the studio UI a structured editor of `.musa` text, so if the editor consumes the studio's *structure*,
Proposition 7 pushes the erasure point far past the render plan. Run against `crates/` and `apps/`.

**Verdict: the premise is false. The editor is not a consumer of the studio's core structure, and case 5 is closed.
Separately, this probe found an error in [08](08-candidate-enriched.md)'s own D-1, corrected below.**

### What the editor actually consumes

`crates/musa-project/src/studio.rs` states the split in its module doc, and the code matches:

- **Display:** `StudioFacts` — "what the interface *displays*: patches, their stages, every parameter's value and unit …
  all display-ready, so a fader renders a number it was handed rather than converting one."
- **Edit:** `StudioEdit` — "musical intent in, `TextEdit`s out." A slider "reads the compiled value and writes back the
  smallest text change that produces the new one", rewriting the literal the composer wrote and nothing else.

So the UI consumes a *projection* and emits *source edits*. That is roadmap §11 and `AGENTS.md`'s "the source is
canonical" working exactly as specified. The editor is a consumer of facts and of text, not of the term structure.

**`StudioGraphSpec` appears nowhere outside `crates/musa-audio/`** — grep across `crates/` and `apps/` returns no
occurrence. It crosses no crate boundary at all.

### The correction to [08](08-candidate-enriched.md) D-1

D-1 claimed that R1's cache key `semantic_hash(M) ⊕ B ⊕ s` is uncomputable because `StudioGraphSpec` lacks `PartialEq`
and `Hash`. **That conflates two different objects.** `docs/core-boundary.md` states R1 over "all instrument bindings
`B`", and its own ledger row for prompt 158 says "`PartId` on the gesture payload is what B already implies". `B` is the
part→instrument binding, not the studio patch graph.

Worse for the claim: **`B` does not exist yet.** Prompts 156, 157, and 158 are all `pending`. There is no type in the
workspace for instrument bindings; `crates/musa-compiler/src/core/mod.rs:351`'s `Binding` is a let-binding in the value
calculus and unrelated.

So D-1 as written is wrong twice over — wrong object, and a defect asserted against code not yet written. What survives
is smaller and forward-looking, and it belongs to the prompt stack rather than to this directory:

> **D-1, corrected.** R1 presupposes that `B` has a decidable equality, because a cache keyed on `semantic_hash(M) ⊕ B ⊕
> s` is only well-defined if `B` can be compared and hashed. `B` is introduced by prompts 157 and 158. Therefore *that*
> is a design constraint on those prompts, not a repair to existing code. The observation is still worth having — it is
> cheaper to satisfy at introduction than to retrofit — but it is not evidence of a present defect and must not be
> reported as one.

### Where Proposition 7 does still bite

Not at the editor, and not at `B`. At the seam this directory noticed at the outset: there are two independent studio
graph representations, `musa_compiler::StudioSpec` and `musa_audio::StudioGraphSpec`, and the compiler-side one is
projected a third time into `StudioFacts` for display. The facts projection is legitimate and mirrors the score's own
facts layer. The first two are the open question, and Prop 7's test applies to them directly: *name the pass that needs
the second representation's structure, or let the first survive to it.* That question is untouched by P-3 and is now the
live one.

### What closes and what opens

- **Closes:** case 5. The erasure point does not move to the editor.
- **Closes:** the worry that a UI consumer would force studio structure across every boundary. It does not; the facts
  pattern already absorbs it.
- **Opens:** whether `StudioSpec` → `StudioGraphSpec` is a justified staged erasure or an unjustified duplication. That
  is answerable by reading the two types against Prop 7 and does not need the motive.
