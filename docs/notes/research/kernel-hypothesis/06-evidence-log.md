# Evidence log

What has actually been run, what it showed, and what it did to the hypothesis. Append-only: a claim that was corrected
stays here with its correction, because the point of the directory is to be refutable and a log that quietly edits its
own failures is not evidence.

---

## Gate 0 — do the deletions exist? (run)

`05-open-questions.md` §10 step 1. Three probes, run against the repository at commit `2fcaccf`. **One passed, two fired
against the hypothesis.**

### G0.1 — the kernel identifies parallel motion with a voice exchange ✅ confirmed

```rust
overlay(seq(note(60), note(62)), seq(note(57), note(59)))   // C4→D4 over A3→B3
overlay(seq(note(60), note(59)), seq(note(57), note(62)))   // C4→B3 over A3→D4
```

`Timeline::semantic_eq` returns `true` for these, with `u8` payloads. Proposition B of `02-denotational-semantics.md`
holds of `musa-kernel` in isolation, as stated.

### G0.2 — but the compiler carries voice identity in the payload, so the pipeline does not lose it ❌ fires

`crates/musa-compiler/src/elaborate.rs:885` assigns `VoiceId(index)` per declared `voice` block within a part, and
`crates/musa-compiler/src/score.rs:715` carries it on the fact. Because the voice tag travels *in the payload*, the two
readings above have unequal payloads in the real pipeline and are therefore distinguished by the multiset denotation
after all.

**Consequences, stated exactly:**

1. **"The kernel deletes voice identity" is too strong.** The kernel does not know about voices; the compiler does,
   through a payload convention driven by surface `voice` declarations. Q3's working stance is functioning, not failing.
2. **L18 is not falsified at the pipeline level.** `02-denotational-semantics.md` §5.2 argues that synchronized
   interchange erases a voice exchange. With voice-tagged payloads the tags move with the notes, so both sides of L18
   agree and the erasure does not occur. §5.2's claim is correct *about the kernel in isolation* and misleading about
   the system. The two tests named in §5.3 are not in fact asserting anything false today.
3. **What survives is narrower and is an APOSD argument, not a semantics argument.** Three specific things remain
   unrepresentable or leaky, and they are what Atom 2 now rests on:
   - *Partial ordering is inexpressible.* OMT `110-row-properties.md` §"Partially ordered sets" describes music whose
     segments are fixed but whose internal order is free. A per-note voice tag cannot say it; a partial order can.
   - *Divisi and convergence are inexpressible.* A line that splits into two and rejoins needs new tags and loses the
     relation between them.
   - *The convention is a leak.* Every consumer must agree on what the tag means, and the kernel's laws are sound over
     tagged payloads only by accident of the tags differing — nothing in the kernel states that invariant.

   These are real, but they are "the design leaks and under-expresses," not "the design loses the music." That is a
   materially weaker claim and the hypothesis must be argued at that strength.

### G0.3 — `docs/rules/events/11-realization.md` already refuted most of the `alt` constructor ❌ fires

Prompt 66 wrote, and prompt 67 implemented, a governing decision against exactly the constructor
`02-denotational-semantics.md` E4 proposes. It gives four reasons. Scored honestly:

| Prompt 66's reason | Verdict against `alt` |
| --- | --- |
| 1. A finite alternative set cannot express the aleatory repertoire (*In C*, Klavierstück XI at 19! orderings, Cage, Feldman, an open jazz chorus) | **Stands.** `alt` covers the narrowest subcase and misses every motivating piece. |
| 2. It breaks T2: in `let x = alt a b in over x x`, does sharing share the *decision*? Both readings are musically real and neither is canonical | **Stands, and E4 is worse than silent about it** — the disjoint-copy semantics of E3 silently picks "two performers choose independently," which is exactly the arbitrary choice prompt 66 refused to make. |
| 3. It breaks T3/T4/N6, so no normal form, so no semantic hash, so prompt 43's recompilation keying breaks | **Answered.** This objection is to a *nondeterministic operator*. `alt` is deterministic: it evaluates to a branching structure rather than choosing among branches. Normalization and hashing survive, on the canonical event structure. |
| 4. A `.musa.kernel` file containing the form cannot be normalized or hashed without a choice environment | **Answered**, by the same distinction. |

**Also, the falsifier in `01-atoms.md` §3 is simply wrong.** First and second endings are *deterministic*: a repeat with
two endings has exactly one hearing, in which both endings sound at different times. `crates/musa-compiler` already
implements them (`score.rs:919` `EndingRegion`, `keywords.rs:220` the `ending` block) by expanding passes, and expansion
is correct there because there is nothing to choose. A falsifier for conflict must be genuinely nondeterministic — an
ossia, an *ad lib.* passage, a cue-optional part, a mobile — and none of those are currently supported, so no existing
feature is evidence for the atom either.

**Net:** Atom 3 as proposed is refuted for the general case and unmotivated for the bounded case. What is left is a
narrow question — whether *notated, bounded* alternatives (ossia above all) want a kernel construct — and it is not
established that anyone wants them.

---

## Gate 2 — did the voice-leading consumer want lines? (run)

`07-adoption-plan.md` §2's kill criterion, answered by prompt 119 as landed in commit `90db0b1`. **Atom 2 is
withdrawn.**

The criterion was stated in advance and had two questions. Both are answered against the atom.

### G2.1 — did any profile need a line relation the tag could not give? ❌ no

Seven profiles — `satb_common_practice`, `species_1` through `species_5`, `jazz_voice_leading` — over twenty-four rules,
nine of them species rules that are entirely about what one line does. Every one of them is served by
`crates/musa-compiler/src/analysis/motion.rs`'s `strands`, and a `Strand` is three fields: a display label, the lane's
*existing* `(PartId, VoiceId)`, and the lane's tones sorted by `(onset, diatonic_height, note_id)`. Succession within a
line is "the next tone in that vector." Ordering *between* lines is the mean diatonic height of the strand, compared by
cross-multiplication so no division is needed.

That is tag plus time ordering, and nothing else. **No shadow line structure was built** — no relation, no threading
pass, no per-note predecessor field, no second identity. The species rules that most wanted a line
(`species_dissonance_passing` must say "this dissonance is quitted by leap," which is a claim about the note *after*
this one in the same line) read it off the sorted vector directly.

### G2.2 — did anything need a partial order or a splitting line? ❌ no

Neither appeared, in any of the twenty-four rules or the eleven example fixtures. The place that came closest is
instructive and is recorded as a limit in `docs/rules/language/07-analysis.md` §8: a jazz voicing written as a chord in
one lane puts every pitch of the chord into a *single* strand, so within that lane a "voice" is not a line at all but a
position from the bottom. The implementation does not paper over this — it reads vertical position and says so. That is
the honest shape of the answer: **where a line exists, the tag names it; where no line exists, the analysis does not
invent one.** A kernel partial order would have had nothing to do in either case.

### What this does and does not settle

It closes Q3 (`docs/rules/events/08-open-questions.md`), which is the outcome §2 named as good: an open question closed
by a consumer's evidence rather than left open forever.

It does *not* refute the two things G0.2 left standing — partial ordering (OMT `110`) and divisi are still
inexpressible, and the tag is still a convention no kernel law states. What it establishes is that **neither has a
consumer**, and §1's rule is that demand comes before design. Atom 2 is withdrawn, not disproved. Reviving it requires a
named consumer that wants one of those two things, which serial partial-order analysis or a divisi engraving feature
could someday supply.

---

## Revised atom status

| Atom | Before Gate 0 | After Gate 0 | After Gate 2 |
| --- | --- | --- | --- |
| 1. Ambient exact time | Retained | Retained, untouched | Untouched |
| 2. Occurrence + **succession** | Strong: the kernel loses voices | **Weakened.** Voices are carried by payload convention. The case is now leakage plus two specific inexpressible things (partial order, divisi). Needs a consumer to want it — see `07-adoption-plan.md` G2. | **Withdrawn.** The decisive consumer landed on tags with no shadow structure. Partial order and divisi remain inexpressible and remain without demand. |
| 3. **Conflict** | Strong: endings and aleatory | **Refuted as proposed.** Prompt 66's reasons 1 and 2 stand; the endings falsifier was wrong. Survives only as a narrow question about notated bounded alternatives. | Untouched; still parked (Track D). |
| 4. **Pulse layers** | Strong | **Unchanged and now the strongest structural claim.** `elaborate.rs:980` still says meter is a region; OMT `098` §Polymeter is still a counterexample; nothing in Gate 0 touched it. | Untouched, and now the only structural claim left standing. |
| 5. **Payload torsors and group actions** | Strong | **Unchanged.** Independent of the kernel-shape question; largely library work. | Untouched. |
| 6. **Orbit identification** | Strong | **Unchanged, and cheapest to test.** Independent of everything else here. | Untouched. |

The ranking has inverted. The two atoms the first write-up led with are the two that Gate 0 damaged, and the three that
survive intact are the three that need the least change to the kernel — 5 and 6 need none at all.

---

## What Gate 0 cost and what it bought

Under an hour of probing overturned the two headline claims of a 1,400-line design document. That ratio is the argument
for running every remaining gate before writing any implementation prompt, and it is also the argument for having
written the document with falsifiers attached — the claims were refutable, so they got refuted, cheaply, before anything
was built on them.
