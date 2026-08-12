# Semantic staging and ownership

This document fixes the objects Musa computes and the boundaries between them. “Must” is normative for prompts 93–143;
candidate precedence is defined in `README.md`.

## 1. Representations

Compilation has four score-side representations and three sound-side representations:

```text
lossless surface/CST
    │ resolve names, infer/check surface types
    ▼
typed total elaboration expression
    │ evaluate values; retain contextual music
    ▼
contextual Music
    │ instantiate in a score environment and close
    ▼
closed Term[ScoreFact]
    │ kernel evaluation
    ▼
Timeline[ScoreFact]
    │ project score facts; realize a named performance profile
    ▼
GestureTimeline[InstrumentSignature]
    │ groove, tempo integration, tuning, frame conversion
    ▼
ScheduledGestureLane
    │ prepared instrument implementation and mix graph
    ▼
stereo Signal
```

There is no implicit `Timeline[Timeline[A]] → Timeline[A]`. `music` composition chooses `sequence` or `overlay`, and
then elaborates to the corresponding kernel term. A `Timeline[ScoreFact]` is the result of evaluating a closed term, not
the universal intermediate type.

## 2. Judgments

The value stage uses the ordinary static and evaluation judgments:

```text
Σ; Γ ⊢ e : τ                     surface/core typing
Σ ⊢ e ⇓ v                        total call-by-value evaluation
Σ; Γ ⊢ D : declaration κ         structural declaration checking
Σ ⊢ D ⇓decl Δ                    declaration-template expansion
```

`Σ` is the finite static environment of declarations and compiler primitives. `Γ` contains immutable value bindings. The
declaration kinds are `library`, `piece`, `part`, `voice`, `performance`, `instrument`, `mix`, and `structure`; they are
not value types.

Compiler ownership is an information boundary, not a convenience category. An operation may be primitive only when it
needs the intentionally hidden contextual-`music` representation, source-aware provenance, direct kernel construction,
or the private finite representation/work budget needed to preserve total evaluation under rank-1 monomorphization.
Everything expressible through public values and those operations belongs in ordinary `.musa` source, including the
bundled standard library. Adding a Rust implementation merely because a source function is familiar or potentially
faster is not semantics-preserving evidence; prompt 127 requires measurement and an equivalence law before such an
optimization.

Contextual music has one private semantic observation:

```text
instantiate : Music × ElabEnv × Beat → Checked KernelFragment

KernelFragment = {
    term: Term[ScoreFact],
    bindings: private finite acyclic environment,
    extent: Duration
}

close : KernelFragment → Closed Term[ScoreFact]
```

`ElabEnv` contains score scope, the optional local scale, prevailing key and meter tracks, and compiler-owned Origin and
realization context. It has no mutable user state. `Beat` is the exact rational notated onset. `close` hoists the
fragment's shared bindings, rejects a free kernel variable or malformed `ScoreFact`, and returns one closed term.

The two staging judgments are therefore:

```text
Σ; ρ; p ⊢m m ⇓ K                instantiate contextual music
Σ ⊢piece Δ ⇓ t : Term[ScoreFact] expand declarations, instantiate, close
```

The kernel alone evaluates `t ⇓k T : Timeline[ScoreFact]`. Surface evaluation never evaluates a timeline, and kernel
evaluation never invokes a surface closure.

## 3. Context-neutral music

A `music` value may read a supplied context but may not mutate the caller's context. It may contain notes, sounded
chords, rests, local annotations, and lexically scoped `in scale`. It may not contain a key, meter, tempo, or clef
change, a part/voice declaration, a profile, an instrument, a mix route, or a package import. Those are structural
declarations with scope or placement authority.

This restriction makes `music` context-neutral. For compatible fragment binding environments:

```text
inst(sequence(m,n), ρ, p) = sequence(inst(m,ρ,p), inst(n,ρ,p+extent(m)))
inst(overlay(m,n),  ρ, p) = overlay(inst(m,ρ,p),  inst(n,ρ,p))
inst(in_scale(s,m), ρ, p) = inst(m, ρ[scale := s], p)
```

These are equations about the chosen constructors, not a monadic join. `in scale` emits no key signature and is not a
claim of modulation or tonicization.

### Higher-order construction and the pitch traversal

`music` and function values may be passed and returned without revealing either representation. An application with
unsupplied required parameters evaluates to a closure over the supplied arguments; consequently
`transpose(i) : music -> music`. The function and block spellings of transpose, stretch, retrograde, and inversion
invoke the same semantic action, so their equality is an implementation theorem rather than a duplicated convention.

The sole pitch traversal is

```text
map_note_pitches : (Pitch -> Pitch) -> Music -> Music.
```

For an occurrence `o = ([a,b), f)` its action is `([a,b), mapPitch(g,f))`, where `mapPitch` changes the written pitch of
`Note` and `Grace` facts and is the identity on every other `FactKind`; sounded chord tones are represented by
simultaneous `Note` facts at this layer. In particular it does not map the tonic of `Key` or the root of `Harmony`. This
distinction is music-theoretic, not merely representational: a key signature and a Roman-numeral/chord analysis state a
tonal reading, while a written note states a sounded pitch. Rewriting the latter does not prove the former has changed.

The temporal-support law follows directly: the traversal changes no occurrence span and no term constructor, hence the
set of pairs `(onset, extent)` and the enclosing timeline extent are identical before and after mapping. Identity and
composition follow by cases on the exhaustive `FactKind` table: on pitch-bearing facts they reduce to the corresponding
function equations; on all other facts both sides are the identity. Origin is deliberately finer: one `MapNotePitches`
step is retained, so equality holds under `≈facts`, not byte-for-byte payload equality.

Interval answers use Musa's signed pair of written diatonic steps and semitones. This follows the distinction between
generic interval size and specific quality in *Open Music Theory*, “Intervals” (`016-intervals.md`): `P8` therefore
means seven diatonic steps and twelve semitones, preserving spelling rather than reducing the answer to a MIDI offset.

## 4. Ownership

| Owner | Knows | Must not know |
| --- | --- | --- |
| `musa-language` | tokens, lossless CST, recovery, formatting | musical types, closures, kernel evaluation |
| private `musa-compiler` elaboration subsystem | `Type`, `Value`, `Closure`, `Music`, modules, theory algorithms, quotation | public backend or DSP types |
| `musa-kernel` | exact time, typed occurrences, term binding, sequence, overlay, scaling | notes, scales, functions, profiles, samples, seconds |
| compiler score projection | `ScoreFact`, context tracks, Origin, `ScoreSnapshot` | graph topology and audio buffers |
| compiler performance preparation | profiles, gestures, tempo, tuning, part lanes | private instrument topology |
| `musa-audio` | instrument implementations, graph validation, prepared buffers | notation semantics and source CST |
| `musa-engine` | prepared render plan, transport, real-time queues | parsing, allocation or graph construction in callback |

The rejected alternative is a public `musa-elaboration` crate. Its only actual caller would be `musa-compiler`, while
its proposed public `Type`, `Value`, `Closure`, `Music`, module environment, and theory APIs are volatile pass details.
The chosen design is one deep private compiler subsystem with the existing `musa_compiler::compile` facade. No public
API exists merely because this specification names a semantic object.

The sound side likewise rejects a combined mutable score-audio object. `PerformancePlan` and `StudioSpec` currently
preserve useful independence but meet too late and too weakly. One deep preparation operation will instead bind a
compiled score snapshot, profiles, selected instrument declarations, and mix declarations into an immutable prepared
audio plan while retaining their distinct editable source declarations.

## 5. Equality and provenance

Three relations serve different questions:

- `t ≡kernel u`: alpha-equivalent closed terms normalize to the same canonical kernel term, including payload bytes.
- `T ≈facts U`: their timelines are equal after the explicit projection that erases Origin and non-sounding stable
  identities, but preserves every musical fact and exact support.
- `m ≈music n`: for every well-formed environment and placement where both instantiate, their closed results are
  `≈facts` and have equal extents.

Full `ScoreFact` equality is deliberately finer than `≈facts`: Origin paths and stable instance identities support
selection, diagnostics, realization choices, and caching. Transform, call, repeat, template-instance, quotation, and
context steps are attached by centralized syntax-directed elaboration. User code cannot inspect or forge Origin.

Caches key typed source, imported build closure, static environment, placement-relevant context, and compiler version.
Using only source text or only `≈facts` is unsound because the same contextual value may instantiate differently under
different scales or meter tracks.

## 6. Closure theorem required of implementation

If a project resolves, all declarations check, `Σ ⊢piece Δ ⇓ t : Term[ScoreFact]`, and resource checking accepts it,
then `t` is finite, closed, kernel-well-formed, and every payload decodes as `ScoreFact`. Kernel totality then gives a
unique finite `Timeline[ScoreFact]`. Prompts 93–119 must establish the typing, normalization, contextual closure,
quotation closure, and adapter lemmas; prompt 144 audits the end-to-end theorem against the implementation.
