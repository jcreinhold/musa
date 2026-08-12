# Domain obligations before another kernel

**Status: scratch; evidence and falsification obligations, not a proposed ontology. Governs nothing.** This note
supersedes the evidential scope of [18](18-minimal-recommendation.md). Open Music Theory remains a useful launch corpus,
but it is overwhelmingly a theory of Western written repertories. It cannot establish that a proposed Musa primitive is
musically general.

The immediate question is narrower than “what is music?”:

> What must a Musa kernel *not presuppose* if repertoire-specific theories are to be stated naturally and connected to
> performance and audio without first translating them into common-practice Western categories?

The answer below is deliberately a list of obligations, not a synthetic account of “non-Western music.” There is no
single non-Western domain. Each named practice is a separate counterexample to a tempting universal.

---

## 1. Evidence labels and limits

- **Verified locally** means the claim was checked in the repository or in the local Open Music Theory corpus.
- **Verified from a source** means the linked article was read for the stated claim.
- **Judged** means a language-design conclusion drawn from those observations.
- **Not yet checked** names evidence this notebook still owes. Absence from this note is not evidence that a tradition
  fits the candidate.

The external readings here are probes, not authorization to define other people's theories for them. A production
standard library for a practice should be reviewed by practitioners or scholars of that practice.

## 2. Western notation is one projection, not the carrier

**Verified locally.** Open Music Theory begins by describing Western notation as a socially situated, selective
translation of sound
([001-introduction-to-western-musical-notation.md](../../../papers/music-theory/open-music-theory/001-introduction-to-western-musical-notation.md)).
Its later chapters show that the translation remains incomplete even for its home repertories: rubato, ametric music,
feathered beaming, metric modulation, timbre, orchestration, and performer realization all require conventions or
information outside a note grid
([098-twentieth-century-rhythmic-techniques.md](../../../papers/music-theory/open-music-theory/098-twentieth-century-rhythmic-techniques.md),
[114-core-principles-of-orchestration.md](../../../papers/music-theory/open-music-theory/114-core-principles-of-orchestration.md)).

**Verified from a source.** In Karnatak music, svara notation is an approximate guide: detailed rhythmic placement and
gamaka realization are learned through oral transmission. Treating the notation as the complete work reverses that
relationship ([Schachter, paragraphs 12–13](https://mtosmt.org/issues/mto.15.21.4/mto.15.21.4.schachter.php)). Sami Abu
Shumays argues more strongly that comparative music theory must elevate oral transmission and melodic vocabulary rather
than merely apply note-centred Western analysis to a larger repertoire
([Abu Shumays, paragraphs 3.7–4.4](https://mtosmt.org/issues/mto.24.30.4/mto.24.30.4.shumays.html)).

**Judged — obligation N.** `Notation` is not a universal source type and `Note` is not a kernel atom. A Musa artifact
may contain notation, but it may instead begin from a performance protocol, a learned vocabulary, an audio process, a
set of constraints, or several partial descriptions. No domain must supply a fake notation value to enter the language.

## 3. Pitch is not a global quotient tower

The tower

```text
spelled pitch -> chromatic pitch -> pitch class -> set class -> row class -> scale degree
```

is useful in specific Western theories. It is not an ontology of pitch.

### 3.1 Karnatak rāga

**Verified from a source.** Rāga identity involves svaras, different ascending and descending behavior, characteristic
prayogas, and pervasive gamakas. The same scale source can support distinct rāgas, while a rāga may use different svaras
in ascent and descent. Gamaka and svara are analytically distinguishable but musically interdependent; fixed pitch
points alone do not reconstruct the foreground
([Schachter, paragraphs 21–31](https://mtosmt.org/issues/mto.15.21.4/mto.15.21.4.schachter.php)).

**Consequence.** A `Raga` cannot be defined as `FiniteSet PitchClass`. A useful theory needs at least contextual melodic
gesture, phrase vocabulary, constraints or judgments of admissibility, and a tonic/drone relation where that practice
requires one. `Gamaka` is not merely a decorative transformation of a prior discrete note.

### 3.2 Arabic maqām

**Verified from sources.** Arabic maqām combines intervallic material with characteristic phrases, melodic course,
modulation possibilities, ornamentation, and aesthetic convention. A practitioner critique warns specifically against
deriving melodic grammar from scale atoms: the perceived functions of tones emerge through culturally learned melodic
movement ([MaqamWorld overview](https://www.maqamworld.com/en/maqam.php),
[Abu Shumays, paragraphs 3.5–3.9](https://mtosmt.org/issues/mto.24.30.4/mto.24.30.4.shumays.html)).

**Consequence.** A `Maqam` cannot be a microtonal replacement for `Key`. A scale or collection may be one observation of
a maqām; it is not the structure itself.

### 3.3 Balinese gamelan

**Verified from sources.** Paired instruments in Balinese gamelan are deliberately tuned apart so their simultaneous
sounding produces *ombak*. Octaves may be stretched or compressed, and the analyzed tuning varies among ensembles
([Sethares et al., “Ombak and octave stretching in Balinese gamelan”](https://www.tandfonline.com/doi/full/10.1080/17459737.2020.1812128),
[Vitale and Sethares, “Balinese Gamelan Tuning: The Toth Archives”](https://iftawm.org/journal/oldsite/articles/2021b/Vitale_Sethares_AAWM_Vol_9_2.pdf)).

**Consequence.** There need not be a single abstract pitch followed by an incidental instrument tuning. Ensemble,
register, partner instrument, and intended beating can participate in pitch realization. Quotienting to pitch class can
erase the phenomenon the theory is meant to state.

### 3.4 Design obligation

**Judged — obligation P.** The kernel may provide numbers, ratios, curves, finite data, and abstract types. It must not
provide a distinguished global `Pitch`, `PitchClass`, octave, temperament, tonic, key, scale degree, chord, or harmonic
function. Those are types and functions defined by a named theory. Different theories may expose non-isomorphic pitch
spaces and different equalities.

The criterion is positive: common-practice harmony should still be concise because its package defines the familiar
frame, quotient, and chord constructions. It just does not receive kernel privilege.

## 4. Time is not meter, and meter is not one pulse lattice

**Verified locally.** The OMT corpus itself contains asymmetrical and changing meter, polymeter, metrical dissonance,
metric modulation, ametric notation, timeline/seconds notation, feathered beaming, swing, rubato, and fermatas
([098-twentieth-century-rhythmic-techniques.md](../../../papers/music-theory/open-music-theory/098-twentieth-century-rhythmic-techniques.md),
[117-hypermeter.md](../../../papers/music-theory/open-music-theory/117-hypermeter.md),
[118-metrical-dissonance.md](../../../papers/music-theory/open-music-theory/118-metrical-dissonance.md)).

**Verified from a source.** Hindustani rūpak tāl is more than “seven equal beats.” Its cycle has culturally known group
structure, gestures, drum pattern, competing hearings, tempo-dependent practice, and repertoire associations. The same
3+2+2 count does not identify another seven-cycle, and performance evidence includes long-form non-isochrony
([Clayton, “Theory and Practice of Long-form Non-isochronous Metres”](https://mtosmt.org/issues/mto.20.26.1/mto.20.26.1.clayton.html)).

**Verified from a source.** In Puerto Rican bomba, dance is constitutive and a lead drummer translates a dancer's
improvised movement into sound. The temporal organization is therefore partly an interaction protocol produced during
performance, not a completed metrical plan supplied in advance
([Smithsonian Folkways, “Los Pleneros de la 21”](https://folkways.si.edu/los-pleneros-de-la-21-afro-puerto-rican-traditions/latin/music/article/smithsonian)).

**Judged — obligation T.** The kernel needs ways to state order, placement, duration, simultaneity, and relationships
among clocks. It must not identify these with meter or require every artifact to have a finite pulse plan. Meter, tāl,
groove, timeline patterns, conducted time, and performer interaction are separate theories over temporal and behavioral
data.

The current finite rational timeline remains a good denotation for *finite placed facts in an exact local time
coordinate*. It is not the universal time of music. It should be parameterized or staged rather than enlarged until it
pretends to model every temporal practice.

## 5. Parallel material is not necessarily voices, parts, or a chord

**Verified locally.** OMT's texture categories do not even transfer unchanged between common-practice and popular music.
Pop analysis often uses functional layers, and instruments may enter, leave, split, merge, or change roles
([008-texture.md](../../../papers/music-theory/open-music-theory/008-texture.md),
[089-texture-in-pop-music.md](../../../papers/music-theory/open-music-theory/089-texture-in-pop-music.md)).
Orchestration may distribute, double, add, omit, dovetail, or transform material; it is not a deterministic payload map
([114-core-principles-of-orchestration.md](../../../papers/music-theory/open-music-theory/114-core-principles-of-orchestration.md),
[116-transcription-from-piano.md](../../../papers/music-theory/open-music-theory/116-transcription-from-piano.md)).

**Judged — obligation X.** `overlay` may combine occurrences without claiming what their relationship means. `Voice`,
`Part`, `Layer`, `Chord`, `Instrument`, and `Track` are interpretations or structures over material, not names for the
children of a generic parallel constructor. Voice identity, when needed, must be explicit evidence or provenance; it
does not fall out merely from a syntax tree.

## 6. A musical artifact may be interactive and socially situated

**Verified from a source.** Bomba's drummer–dancer dialogue makes a concrete counterexample to a closed “work →
performance → sound” pipeline. The dancer's action is not an imperfection in realization; it is an input which partly
determines the music while it occurs. The singing, drumming, dancing, community practice, and historical identity are
not interchangeable projections of a single fixed note sequence
([Smithsonian Folkways](https://folkways.si.edu/los-pleneros-de-la-21-afro-puerto-rican-traditions/latin/music/article/smithsonian)).

**Judged — obligation I.** Musa need not model all social meaning in its first release. It must avoid defining it out of
existence. A future live layer needs typed inputs, outputs, state, choice, and causal response. A finite score should be
one accepted artifact form, not the compulsory normal form of every musical process.

## 7. Analysis is a claim, not recovered ontology

**Verified locally.** Segmentation into motives or pitch-class sets depends on rhythm, timbre, articulation, register,
and context. A set-theoretic fact can be formally true and analytically vacuous
([104-analyzing-with-set-theory-or-not.md](../../../papers/music-theory/open-music-theory/104-analyzing-with-set-theory-or-not.md)).
Motive recognition allows transformations and depends on analytical judgment; a motive may be rhythmic, contoural, or
timbral rather than a literal repeated subterm
([052-foundational-concepts-for-phrase-level-forms.md](../../../papers/music-theory/open-music-theory/052-foundational-concepts-for-phrase-level-forms.md)).

**Judged — obligation A.** Analysis values should name a theory, target evidence or regions, and carry provenance.
`HarmonicFunction`, `Motif`, `Raga`, and `Form` are not normalization results of the generic kernel. “A motif is a split
idempotent” remains a category-theoretic pun unless a specific analytical theory defines retraction maps and shows that
musicians use the resulting invariant.

## 8. The five distinctions the kernel must preserve

The probes above repeatedly force five separations:

1. **Description from realization.** A notation, oral cue, constraint set, or protocol may underdetermine performance.
2. **Abstract intention from physical gesture.** A pitch label is not a trajectory of frequency and technique.
3. **Logical time from performed time.** Beat, cycle position, conducted time, seconds, and sample frames are different
   coordinates connected by explicit maps.
4. **Finite artifact from running behavior.** A processor graph or interaction protocol is finite; the signal or live
   run it denotes may be unbounded.
5. **Fact from interpretation.** An occurrence can be present without the kernel deciding its function, voice, phrase,
   or cultural meaning.

These distinctions do more work than the notebook's proposed `world` keyword. Ordinary abstract types and typed maps can
preserve them if the language lets domain packages own their equalities.

## 9. Candidate invariant: plural theories over a small metalanguage

**Judged.** The smallest credible universal is not a universal musical object. It is:

> a total typed language for defining finite data and transformations, plus a few deep abstract structures for finite
> temporal placement, causal process wiring, and explicit clock conversion.

Repertoire packages then define their own musical objects and judgments. For example:

```text
Western.Key          Karnatak.Raga       Arabic.Maqam
Balinese.Ensemble    Hindustani.Tal      Bomba.Exchange
```

None is a subtype of another. A package may provide comparisons or translations, but those are ordinary named maps and
may be partial, approximate, relational, or evidence-producing.

This is closer to a comparative linguistics stance than to a pitch ontology: the metalanguage supplies ways to define
vocabularies and compositional rules; it does not decree one vocabulary's phonemes universal.

## 10. Admission rules for the next candidate

A proposed primitive is rejected if any of the following is true:

1. a named practice must provide a meaningless `none`, fake tonic, fake meter, or fake note to use it;
2. the primitive identifies descriptions that a practice distinguishes;
3. the primitive makes an analytical classification definitional;
4. the primitive assumes a complete work exists before a performance begins;
5. the same symbol is being asked to mean temporal succession, function composition, and processor wiring;
6. its laws hold only after explicit musical information has been erased;
7. a package-defined type and function express it just as naturally.

A feature is admitted only when it makes at least two materially different examples simpler *and* its laws are stated
without repertoire-specific side conditions.

## 11. Known gaps in this obligation map

**Not yet checked:** Indigenous and land-based practices; liturgical recitation beyond the brief chant probe; tuning and
ensemble organization in multiple African traditions; Chinese, Japanese, Korean, Persian, Turkish, and Central Asian
theories considered in their own terms; sign languages and embodied music; DJ practice, turntablism, live coding, and
studio-born works; accessibility practices; music whose relevant identity is inseparable from ritual, ownership, or
restricted knowledge.

The gap is a stop sign against universality claims, not a reason to postpone every design decision. The next calculus is
a candidate for Musa's intended subset. Its abstractions must remain open enough that adding a new domain package does
not require pretending the new practice was Western notation in disguise.
