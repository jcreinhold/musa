# 07 — Analysis

Status: **candidate**, with `docs/rules/language/05-verification.md` §3 governing over it.

`05-verification.md` divides theory work into three strengths: constructor invariants, explicit assertions, and
interpretive analyses. This document is the third one, elaborated. It fixes what an analysis may say, what a caller
receives, and — the part that matters most — what a new analysis kind must prove before it may ship.

## 1. What an analysis is

An analysis **observes a compiled score and reports what it saw**. It never constructs music, never rewrites source, and
never raises a diagnostic. Those are not conventions; they are the shape of the operation:

```rust
pub fn analyze(snapshot: &ScoreSnapshot, request: &AnalysisRequest) -> Result<AnalysisReport, AnalysisError>
```

The snapshot is borrowed and the report is a value. There is no path from an analysis back into the resolver, which is
what keeps an analysis from becoming a lint with extra steps — prompt 83 drew that line and this does not move it.

The exit code of `musa analyze` says whether the *request* could be answered, never what the report contains. A piece
with nothing in the requested window is a piece with nothing in the requested window; it is not a failure.

Consequently:

- **A bad request is an error.** A misspelled part, a voice that is not in the part named, a window that ends before it
  starts. Each is something the caller can fix by asking differently, which is the test for belonging in `AnalysisError`
  at all.
- **An empty answer is a report.** Zero findings is a well-formed observation.

## 2. The admission rule

> **No analysis kind ships without an abstract domain, an abstraction map, and a soundness claim.**

The discipline is Peyton Jones (1987) Chapter 22, *Strictness Analysis*, §22.1: a program analysis is an abstract
interpretation, which is an abstract domain, an abstract version of each operation, and a stated relationship to the
concrete semantics. Musa needs no strictness analysis. It needs that shape.

Each kind states, in the doc comment of the module that implements it:

1. **The abstract domain.** What its findings *are*, as a set. Not "chord labels" — the actual objects, with the
   coordinates they carry.
2. **The abstraction map α.** How the concrete score projection (`ScoreSnapshot`, itself a projection of a
   `Timeline<ScoreFact>`) maps into that domain. Every finding must be the image of something under α.
3. **The soundness claim.** What γ, the concretization, admits: the set of concrete scores a report is consistent with.
   This is what a finding licenses a reader to conclude — and, equally, what it does not.

This is not documentation for its own sake. It is what makes §3's classification mean anything. Without a stated α, a
"candidate" is a severity label chosen by feel, and a finding with no stated relationship to the score is exactly the
false claim `03-musical-domains.md` §5 forbids. **An analysis whose soundness claim cannot be written must not ship.**

§22.3 of the same chapter is the standing warning about the other half: the obvious treatments of recursion are wrong.
An analysis that walks a structure with self-reference — a form graph, a motivic derivation, a reduction tree — owes a
fixed-point argument, not a recursion that happens to terminate on the corpus.

## 3. Findings: fact, candidate, conflict

A finding carries a stable code, a typed observation, a standing, and evidence. The standing is a position relative to
the kind's own abstraction, never a severity:

| Standing | What it means |
| --- | --- |
| `fact` | α determines it: every concrete score the report is consistent with agrees about it. |
| `candidate` | Several concrete readings survive α and this is one of them. |
| `conflict` | Two readings α says cannot both hold, reported together. |

A candidate is never reported alone: the readings it competes with are in the same report. Their order is the kind's
**stated deterministic ranking**, and it is not a probability. Musa does not emit a confidence number it cannot derive —
"73% likely a half cadence" is a claim about a corpus nobody named.

There is no privileged "the analysis". Two kinds may return different well-typed readings of one passage; that is data,
not malformed music. Analysis results enter kernel payloads only when an author explicitly writes an annotation derived
from one, and the provenance records that it was their choice.

## 4. Evidence

Evidence answers "where can I see this", and it has three shapes because there are three genuinely different answers:

| Evidence | When | What a caller can do with it |
| --- | --- | --- |
| `Event` | the finding is about a note, chord, or rest | select it on the page; the span is the statement that *spells* it — the note inside the motif, not the `use` that placed it |
| `Annotation` | the finding is about something written above the staff | reveal it in the source |
| `InForce` | the finding is about a value in force from an instant | nothing to reveal: a context track records where a value takes force, which is a place in the piece rather than a place in a file |

Collapsing these into one optional span would make every consumer rediscover which case it was holding.

Reports are **totally ordered and deduplicated by `analyze` itself**, not by whoever prints them: by instant, then by a
fixed rank per observation kind, then by part, voice, and event. Two runs of one kind over one score produce identical
bytes — otherwise nobody can diff a report against yesterday's and act on the difference.

## 5. The boundary

The public surface is one request, one report, one function, and the types they are made of. Segmenters, indexes,
candidate graphs, and theory values stay private to `crates/musa-compiler/src/analysis.rs`.

A public `musa-analysis` crate was considered and rejected. Its only caller would be `musa-compiler` — every consumer
reaches analysis through `musa-project` — while its public surface would be exactly the pass details that later kinds
change. A new crate is justified only by a consumer that genuinely needs to *own* the algorithms, and there is none.

Callers reach it through `ProjectSession::analyze`, which resolves the report into display-ready facts: names instead of
ids, bars and beats instead of rationals, lines instead of byte offsets, and one sentence per finding. That is not a
pass-through — `docs/rules/desktop/03-interaction.md` §7 lists what a frontend may compute, and nothing musical is on
it, so the resolution has to happen somewhere below the frontend and this is where.

## 6. Adding a kind

1. Write the abstract domain, the abstraction map, and the soundness claim, in the module doc, before the code.
2. Add the variant to `AnalysisKind` and give it a `method` line and a non-empty `assumptions` list. An empty
   assumptions list is itself a claim — that the reading depends on nothing — so no kind may leave it empty.
3. Implement it as a private submodule taking the resolved lanes and window.
4. Prove the soundness claim in tests: what the kind reports, what it refuses to report, and — for a kind that emits
   candidates — that competing readings arrive together and in the stated order.
5. Name it honestly. `05-verification.md` §2's rule holds here too: `species.first_above(cantus)` is honest,
   `valid_counterpoint(cantus)` is not.

## 7. The kinds

### `facts`

**Abstract domain.** The finite set of *pointed statements* of a score: a sounding written pitch with its exact span, a
silence with its exact span, a chord symbol at an instant, and a key or meter in force from an instant — each paired
with where in the score it can be seen.

**Abstraction map.** α restricts the snapshot to the requested lanes and window and reads every surviving score event,
harmony annotation, and key and meter stretch into exactly one element of that set. A chord event becomes one `Sounding`
per tone: a chord at this layer is simultaneous notes and nothing more (`00-semantics.md` §3).

**Soundness.** α is *exact on what it reports*. For every finding there is a statement of the score with precisely those
coordinates, and every statement of the score inside the scope and window has a finding. Therefore every finding is a
`fact`: α separates every pair of concrete scores that differ on anything it reports, so no two readings survive it and
there is nothing to be a candidate about.

A finding licenses exactly "the score states this here". It licenses nothing about material the request excluded: γ of a
report is every score agreeing with it inside the scope and window, and that set is not a singleton. A reader who
concludes "the piece is in C major" from one `key-in-force` finding over one bar has read something α does not say.

**What it does not do.** It does not interpret. A chord symbol is reported as written and no notes are derived from it,
because `crates/musa-compiler/src/harmony.rs` opens by saying a symbol is recorded and never interpreted. Spelling
survives: `d#4` is reported as D-sharp and never as E-flat.

### Segmentation, which the other three kinds all rest on

Which notes count as sounding together is the first interpretive act of any harmonic reading, and it is a **policy the
request states**, never a discovery. `AnalysisRequest::segmenting` chooses one and the report carries it in its
assumptions:

| Policy | The slice | What it is good for, and what it loses |
| --- | --- | --- |
| `attacks` | begins at every attack and holds to the next | keeps every note; a passing tone is inside the sonority |
| `beats` | one per notated beat, holding what sounds when the beat arrives | hides offbeat passing motion; loses a chord change between beats |
| `harmony-lane` | one per written chord symbol, running to the next | the source's own reading; absent where the source wrote no harmony lane |

A note belongs to a slice when it has begun by the slice's start and has not stopped by then. Slices with nothing
sounding are dropped — a rest is a `facts` observation, not a chord with no notes in it.

### `chords`

**Abstract domain.** Per slice: its set of spelled classes and its bass, paired with every *(chord class, relation)*
where the relation is one of `exact`, `incomplete`, `with extra tones`, `partial`.

**Abstraction map.** α reduces each slice to its spelled classes — losing register, doubling, and voicing, which is what
"chord" means at all (OMT `019-inversion.md`) — and pairs it with every chord in the analysis vocabulary, rooted on
every sounding class, standing in one of the four relations. Only the strongest relation any chord achieves is reported,
with every chord that achieves it. The bounds are stated policy: at most one member absent, at most one class extra.

**Soundness.** α is exact on the slice, so the `sonority` finding is a fact. It is *not* exact on the naming: a class
set is generally the content of several chords. A fit is a fact exactly when one chord stands in the strongest achieved
relation **and** that relation is exact; everything else is a candidate.

**The vocabulary** is triads and sevenths (OMT `017-triads.md`, `018-seventh-chords.md`), the added-sixth chords, the
suspensions, and the three augmented sixths. It is the analysis's policy about what it is willing to name, not the
language's list of chords — `crate::chord` knows more, and offering all of them for every slice would bury the two or
three a musician would argue about.

**A symbol reading** compares a written symbol against the notes under it. Agreement is a `fact`, disagreement a
`conflict`, and neither is a diagnostic: the compiler still derives nothing from a symbol, and this is a reader holding
two things the source already said next to each other.

### `tonal`

**Abstract domain.** Key regions (a stretch of slices with the set of keys accounting for every class in it), numerals
(a key, a slice, a Roman numeral, a fit), and changes of tonic (a boundary between two regions, read both ways).

**Abstraction map.** α scans the slice sequence left to right accumulating classes and closes a region when no key
accounts for the accumulation. Inside a region it stacks the collection's own thirds on each degree — so a numeral
carries no quality of its own, the collection supplies it (`crates/musa-compiler/src/roman.rs`) — and adds the applied
dominant and applied leading-tone chords of each tonicizable degree (OMT `050-tonicization.md`). Minor keys are read
through the natural *and* harmonic collections, because a minor key sounds its raised seventh at every cadence (OMT
`014`).

At each region boundary the two adjacent chords are read in **both** keys: the last chord of the old region under the
new key, which is what makes it a pivot at all, and the first chord of the new region under the old key, which is where
an applied chord shows up as `V/V` rather than as `V`.

**Soundness.** α does not determine a key. A key region is a fact only when one key survives and the passage is one
region. A numeral is a fact only when its region's key is a fact, its fit is exact, and it is the only numeral the key
offers — three conditions that fail constantly, which is the honest result.

**Tonicization versus modulation.** Every boundary produces *both* candidates, with the OMT `051` criteria attached as
grounds: a pivot chord prepares the change; the new key holds to the end of the passage; a cadence confirms it. **No
criterion is about duration**, and nothing in the implementation compares a region's length to a threshold. Where the
criteria underdetermine the reading — which is the normal case — both readings stay in the report and the musician
decides.

**Recursion.** There is none, deliberately. The region scan is a single left-to-right pass over a finite slice sequence
with a monotonically shrinking key set, so it needs no fixed point. §2's warning from Peyton Jones §22.3 applies to any
future reading that iterates — keys informing segmentation informing keys — and such a reading would owe a fixed-point
argument before it could ship.

### `cadences`

**Abstract domain.** Potential cadence points — the instants the source marks as phrase endings, the instants a rest
begins, and the end of the piece — each paired with the two slices arriving there, a key, a cadence name, and a verdict
on each criterion in OMT `036-introduction-to-harmony-cadences-and-phrase-endings.md`.

**Abstraction map.** α locates the points, takes the last slice ending at or before each and the slice before that,
reads both as numerals in each key `tonal` proposes, and checks the harmonic criterion (which numerals the two chords
are), the melodic criterion (the tonic in the top voice), and the positional criterion (both chords in root position).
V–I is perfect authentic only when the melodic and positional criteria both hold; otherwise imperfect. A phrase ending
on V is a half cadence; V–vi is deceptive.

**Soundness.** **No cadence finding is ever a fact.** A cadence is a formal event, and a phrase ending is not
recoverable from pitch and rhythm — a subverted cadence has every harmonic and melodic feature of a real one and does
not end the phrase. So γ of a cadence finding contains scores where the place is a cadence and scores where it is an
evaded gesture, and the report says which criteria held so a reader can tell them apart.

A progression that is none of the four is **not reported at all**. There is no weak cadence and no confidence number:
the absence is the answer.

### `voice-leading` and `counterpoint`

These two kinds share a domain and differ in what α reads it from, so they are stated together. Both take a **profile**
— a named style — and neither has a default. There is no unnamed "good voice leading" to fall back on: OMT `022` and OMT
`076` disagree about parallel fifths because they are describing different music, and a reading that averaged them would
describe none.

**Abstract domain.** For the profile the request named: the set of pairs *(rule, place)* where the music departs from
that rule, together with the list of rules the profile looked at. A place is a span, the voices involved, the interval
at issue where the rule is about one, and — where a rule has several criteria — a verdict on each. The list of rules
looked at is part of the domain and not a courtesy: a report that named only departures could not distinguish "this
passage has no parallel fifths" from "nobody looked".

**Abstraction map.** For `voice-leading`, α cuts the requested lanes at every attack, reads each instant as a column of
sounding pitches ordered from the bottom, and evaluates each rule over the columns and over the motion between adjacent
ones. For `counterpoint`, α designates one voice as the cantus firmus — the request says which, because a first-species
cantus and a first-species counterpoint are both one note per measure and nothing in the notation tells them apart —
takes the cantus's notes as the measures, and reads the other voice against them. Every interval is measured from the
lower sounding pitch upward, and asked about with a bass present, because in a two-voice texture the lower voice *is*
the bass and OMT `023` makes the perfect fourth's consonance depend on exactly that.

**Soundness.** Every departure is decidable from the notated score: two voices either move in parallel fifths or they do
not. So a departure is a `fact` **about the motion**, and it is not a fact about the music being wrong. That separation
is the whole design: the finding states the motion, and the rule's *strength* states what one historical pedagogy makes
of it.

| Strength | What the style makes of a departure | Standing of the finding |
| --- | --- | --- |
| `definitional` | the passage is not the thing the profile is about | `conflict` — the request and the music disagree |
| `hard in this exercise` | the exercise forbids it | `fact` |
| `guideline` | the tradition advises against it | `fact` |

A definitional departure is a conflict rather than a fact because it is not an observation about the music at all: a
three-voice passage read as SATB, or a two-note-against-one exercise read as first species, is a request that does not
fit its subject. The music is intact; the lens is the wrong one.

The two rules that read a key are the other exception. Under a key the source wrote, their departures are facts; under a
key the request assumed with `--key`, they are candidates, because the reading that produced them is one the request
chose.

γ of a report is every score that departs from those rules in those places. It is **not** "every score in this style".
Nothing here decides whether a passage is in a style, and a profile is a lens the request picked up.

**The profiles** are `satb_common_practice` (OMT `022`), `species_1` through `species_5` (OMT `023`–`028`), and
`jazz_voice_leading` (OMT `076`). The fifth species' rule list is computed as the union of the other four rather than
written out, because "florid counterpoint mixes the species" is a statement about those four lists and a fifth list
written by hand would drift away from what it claims to combine.

**The rules.** The registry in `crates/musa-compiler/src/analysis/rules.rs` is the table below, and a rule that is not
in it cannot be checked by anything — an assertion resolves its rule word there, and a report prints from the same rows.
Every row has a citation, because a rule whose citation is "everyone knows" is what the registry exists to refuse.

| Rule id | Profile | States | Strength | Cites |
| --- | --- | --- | --- | --- |
| `satb_four_voices` | SATB | four voices sound, each sounding one note at a time | definitional | OMT 022 §Voices |
| `satb_ranges` | SATB | each voice stays within the range its part is written for | exercise | OMT 022 §Ranges |
| `satb_spacing` | SATB | adjacent upper voices lie within an octave of each other | exercise | OMT 022 §Spacing |
| `satb_crossing` | SATB | a voice does not sound below the voice beneath it | exercise | OMT 022 §Crossing |
| `satb_overlap` | SATB | a voice does not move past the note its neighbour has just left | exercise | OMT 022 §Overlap |
| `satb_parallel_perfects` | SATB | no two voices move in parallel fifths, octaves, or unisons | exercise | OMT 022 §Parallels |
| `satb_direct_perfects` | SATB | the outer voices do not arrive at a perfect fifth or octave in similar motion with a leap on top | guideline | OMT 022 §Direct fifths and octaves |
| `satb_doubling` | SATB | the leading tone of the key in force is not doubled | guideline | OMT 022 §Doubling |
| `satb_tendency_resolution` | SATB | the leading tone of the key in force rises to the tonic when it moves | guideline | OMT 022 §Tendency tones |
| `species_rhythm` | species 1–5 | the counterpoint stands in this species' rhythmic relation to the cantus firmus | definitional | OMT 023 §The species |
| `species_begin` | species 1–5 | the exercise begins on a perfect consonance | exercise | OMT 024 §Beginning |
| `species_end` | species 1–5 | the exercise ends on a unison or octave, approached by step | exercise | OMT 024 §Ending |
| `species_consonance` | species 1–5 | the interval on each strong beat is a consonance | exercise | OMT 023 §Consonance and dissonance |
| `species_dissonance_passing` | species 2, 3, 5 | a dissonance on a weak beat is approached and left by step in one direction | exercise | OMT 025 §Dissonance |
| `species_suspension` | species 4, 5 | a dissonance on a strong beat is prepared as a consonance, held over, and resolved down by step | exercise | OMT 027 §Suspensions |
| `species_parallel_perfects` | species 1–5 | consecutive strong beats do not repeat a perfect consonance of the same size | exercise | OMT 023 §Motion |
| `species_direct_perfects` | species 1–5 | a perfect consonance is not approached by similar motion | guideline | OMT 023 §Motion |
| `species_leap_recovery` | species 1–5 | a leap of a fourth or more is followed by motion in the other direction | guideline | OMT 023 §Melodic shape |
| `jazz_guide_tones` | jazz | the third and the seventh of the written symbol sound in the voicing | guideline | OMT 076 §Guide tones |
| `jazz_guide_tone_motion` | jazz | a guide tone is held or moves by step into the next voicing | guideline | OMT 076 §Guide tones |
| `jazz_common_tone` | jazz | a pitch class both voicings contain is kept in one voice | guideline | OMT 076 §Voice leading |
| `jazz_small_motion` | jazz | no upper voice moves by more than a third between voicings | guideline | OMT 076 §Voice leading |
| `jazz_spacing` | jazz | no interval within the voicing exceeds an octave, and no second sounds below the tenor register | guideline | OMT 076 §Spacing |
| `jazz_omission` | jazz | the voicing sounds the root or the fifth of the written symbol | guideline | OMT 076 §Rootless voicings |

**Assumptions each profile carries.** SATB reads a key for its two tendency rules and says whether it read the source's
or the request's. The jazz profile reads the harmony lane, so a passage with no written symbols is a passage it can say
nothing about. Species reads the designated cantus and nothing else about the request.

**What an author may assert.** Five of the twenty-four are assertable with `assert follows(…)`:
`satb_parallel_perfects`, `satb_spacing`, `satb_overlap`, `jazz_small_motion`, and `jazz_spacing`. The criterion is not
importance — it is locality. An assertion sees the passage inside its own braces and nothing else, so the rules it can
name are the ones decidable from that passage's own sonorities: no key in force, no cantus designated, no harmony lane,
no neighbouring music. The rest are readings of a score, and a score is not what an assertion is looking at.
`examples/analysis/asserted-voicings.musa` asserts all five, and `examples/broken/claim-*.musa` fails all five.

Both sides check the same rule through the same predicates in `analysis::motion`, so an assertion and an analysis cannot
come to different answers about the same two chords. That is a structural guarantee rather than a discipline: there is
one statement of each rule and two traversals over it.

## 8. Known limits, and the false positives they produce

Stated here rather than discovered by a user. Each is a consequence of a decision made elsewhere in this document, not a
defect to be patched quietly.

| Limit | What it produces | Why it stands |
| --- | --- | --- |
| Chord membership is by **spelling** | A fully diminished seventh spelled B–D–F–A♭ reports *one* rooting, not the four a listener hears. `examples/analysis/equivocal-sonority.musa` bar 4. | Re-spelling the score to find more readings would be constructing music, which §1 forbids. |
| Embellishing tones are not identified | A passing tone inside a slice makes the fit `with extra tones` and adds candidates that are not chords anyone hears. | Deciding a note is embellishing is a reading of a *line* (OMT `039`); a slice-wise abstraction cannot make it, and pretending otherwise would be a false claim. |
| Key candidates come from **collection membership** | A short diatonic passage returns three or four key candidates, most of which no listener entertains. `examples/analysis/pivot-ambiguity.musa` bar 1 returns four. | The ranking is stated and deterministic (written key, then nearest on the circle of fifths); narrowing it further would need a corpus prior nobody named — exactly what §3 refuses. |
| Regions are cut where **no key survives** | A single chromatic chord in an otherwise stable passage can open a spurious region. | The alternative is a threshold on how much chromaticism a key tolerates, which is a number with no source. |
| `beats` reads the meter, not the harmonic rhythm | A piece whose chords change every half-note gets one slice per quarter and reports the same chord twice. | Harmonic rhythm is what the analysis is *for*; deriving the segmentation from it would be circular. |
| A cadence needs the two slices adjacent to the point | A cadential ⁶₄ before the dominant is invisible: the reading sees V–I and not I⁶₄–V–I. | Widening the window is a design change with its own soundness claim, not a parameter. |
| A jazz voicing is read as a **chord in one lane** | A four-note voicing written as four lanes and one written as one chord are the same music, and only the second has "voices" the jazz rules can count. | A jazz voicing's voices are positions in a sonority, not lines the source named; asking which line a note belongs to would be a reading of a *part* nobody wrote. |
| In an asserted passage a "voice" is a **position from the bottom** | A crossing inside an asserted chord is invisible: the sonority re-sorts and the assertion sees the same set of pitches. | An assertion sees sonorities and not lanes (§7). The rules that need lanes are the ones §7 does not admit into `assert follows(…)`, which is the same boundary stated from the other side. |
| A style profile is **named, never inferred** | A passage in one style read under another profile reports departures throughout, and the report is correct. | Deciding a passage's style is a claim about a corpus nobody named — §3's refusal again. The profile is on the request so the reader can see which lens produced the reading. |

The fixtures in `examples/analysis/` are the corpus these are measured against, and each one's header states the
candidate set the reading is expected to produce. A change that alters those sets is a change to the analysis, and the
header is where it must be argued.
