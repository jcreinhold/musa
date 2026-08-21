# Semantic staging and ownership

This document fixes the objects Musa computes and the boundaries between them. "Must" is normative for prompts 93–171;
candidate precedence is defined in `README.md`. Prompt 127a rewrote it against the event-track and machine core.

## 1. Representations

Compilation has one source stage that produces **two** core values, and each core value has its own downstream chain:

```text
lossless surface/CST
    │ resolve names, expand bounded syntax adapters, check bidirectionally into the core
    ▼
typed total source expression
    │ evaluate, under a versioned cost budget
    ▼
values ─────────────┬────────────────────────────────┐
                    │                                │
      EventTrack<C,A>                         Machine<K,A,B>
                    │                                │
    closed Term[ScoreFact], core evaluation          │
                    ▼                                │
    EventTrack<WrittenTime, ScoreFact>               │
                    │ project facts; realize a named performance profile
                    ▼                                │
    EventTrack<PerformedTime, Gesture>               │
                    │                                │
                    └── schedule(format, policy, time map, track) ──┐
                                                                    ▼
                                              Schedule<Gesture> = machine + decisions
                                                                    │ prepare_audio(format, machine)
                                                                    ▼
                                                             PreparedMachine
                                                                    │ step: one sample frame
                                                                    ▼
                                                              audio history
```

Four things this diagram is asserting:

- There is no implicit `EventTrack<C, EventTrack<C,A>> → EventTrack<C,A>` (`../kernel/03-denotational-semantics.md`
  D12). A `music` block chooses `follow` or `together` and elaborates to the corresponding core term.
- `EventTrack<WrittenTime, ScoreFact>` is the result of evaluating a closed term, not a universal intermediate type.
- **There is no contextual `Music` stage.** Earlier drafts had one between the typed expression and the closed term;
  prompt 127a deletes it. Reusable material is an ordinary value of an ordinary type (§3).
- **The machine is not outside this language.** The same expression language builds both columns; what is outside is the
  audio *history*, which is coinductive and is not a value (`../constitution.md` §4).

## 2. Judgments

The value stage uses the ordinary static and evaluation judgments:

```text
Σ; Γ ⊢ e ⇐ τ ⇝ t                 checking: τ is known; t is the elaborated core term
Σ; Γ ⊢ e ⇒ τ ⇝ t                 inference: τ is produced (`02-core-calculus.md` §2)
Σ ⊢ ⟨budget, e⟩ ⇓ done(v)        total call-by-value evaluation within a cost budget
Σ ⊢ ⟨budget, e⟩ ⇓ failed(r)      resource exhaustion; never a different accepted value
Σ; Γ ⊢ D : declaration κ         structural declaration checking
Σ ⊢ D ⇓decl Δ                    declaration-template expansion
```

`Σ` is the finite static environment of declarations and compiler builtins. `Γ` contains immutable value bindings. The
declaration kinds are `library`, `piece`, `part`, `voice`, `performance`, `instrument`, `mix`, and `structure`; they are
not value types.

**The expansion phase is these judgments in a second environment, not a second language.** Adapter expansion happens
before name resolution and elaboration, and an adapter module is checked and evaluated by the same checker and the same
evaluator, under a phase environment `Σφ` that adds the phase-local types `Syntax<Cat>`, `NodePath`, `BindingPath`, and
`SyntaxStep<C, A>` and a separate registry of compiler-owned phase operations. `Cat` is the two-case index — `Expr` and
`TokenTree` — that says how a syntax value parses (`11-quotation.md` §1); the untyped `Syntax` it replaces is the same
tree with nothing claimed about it, which is now spelled `Syntax<TokenTree>`. Ordinary source is read in a scope where
none of those names resolve, so nothing the phase owns can be written, named, or obtained outside it. The one thing that
crosses back is the answer, which is syntax that stands where the region stood; a sealed step is never part of it,
because a step is not storable data (`02-core-calculus.md` §1.2, §5.9).

`Σφ`'s registry is a second registry rather than a fifth builtin family, so §5.8's four families of the source core are
unchanged by it. Descent into a syntax value happens in exactly **two** places: `recurse_syntax` for syntax of unknown
shape — with `run_syntax_step` resuming a descent it did not start and `syntax_fold_from_leaves` derived from the first
at a context nothing reads — and a **quote pattern** for a shape the adapter can write down (`11-quotation.md` §4).
Prompt 127da's earlier law that a bottom-up fold is the only way into a syntax value was superseded by the traversal,
with its reason; prompt 131 admits the second entry, and the reason the rule existed is unchanged: the phase must not
grow a second *uncontrolled* traversal, because an uncontrolled traversal is where totality, path uniqueness, and
opacity would be lost together. A quote pattern is controlled because it is not a traversal — it destructures one level
of a shape written in the source grammar, supplies no path, reveals no `SourceInfo`, and binds only proper children, so
a definition that recurses through one is checked by the ordinary termination measure rather than by a special rule.
`11-quotation.md` §5 is where those four properties are stated as the obligations they are.

The budget can stop an evaluation but cannot change an accepted one: if two runs both reach `done`, they reach the same
value (`02-core-calculus.md`).

Compiler ownership is an information boundary, not a convenience category. An operation may be a builtin only when it
needs source-aware provenance, direct core construction, a registered primitive's private state, or the private finite
representation and work budget needed to preserve total evaluation. Everything expressible through public values and
those operations belongs in ordinary `.musa` source, including the bundled standard library. Adding a Rust
implementation merely because a source function is familiar or potentially faster is not semantics-preserving evidence;
an optimization requires measurement and an equivalence law.

The two staging judgments are therefore:

```text
Σ; ρ; p ⊢m m ⇓ K                 build a fragment at placement p
Σ ⊢piece Δ ⇓ t : Term[ScoreFact] expand declarations, build, close
```

where `K` is a private fragment holding a term, an acyclic binding environment, an exact duration, and an exact
eventual-occurrence count; `follow` adds durations and counts, `together` takes the maximum duration and adds counts.
Closing retains only reachable bindings, wraps them in dependency order, and checks the term.

The core alone evaluates `t ⇓k T : EventTrack[WrittenTime, ScoreFact]`. Source evaluation never evaluates a track term,
and core evaluation never invokes a source closure.

## 3. Reusable material is an ordinary value

Earlier drafts specified a **contextual `music`**: a value that read a supplied context — placement, voice scope, local
scale — and had one private semantic observation, `instantiate`. That type is deleted. What replaces it is smaller:

- a fragment is a value of type `EventTrack[WrittenTime, ScoreFact]`;
- a motif is a source function returning one;
- placement is applied by the enclosing voice's left fold, not read from an ambient context.

Composition is therefore the core's own operations, with no separate equation set to maintain:

```text
use m; use n;      ⇒  follow(m, n)          durations add
voices m and n     ⇒  together(m, n)        the longer duration wins
```

A block may still contain notes, sounded chords, rests, local annotations, and lexically scoped `in scale`. It may
**not** contain a key, meter, tempo, or clef change, a part/voice declaration, a profile, an instrument, a mix route, or
a package import. Those are structural declarations with scope or placement authority, and "from here onward" has no
unique meaning in a value usable at several places. `in scale` is resolved while pitches are resolved — before any track
value exists — so it emits no key signature and is not a claim of modulation or tonicization.

### Higher-order construction and the pitch traversal

Track values and function values may be passed and returned without revealing either representation. A **call must be
complete**: partial application is not a value (`02-core-calculus.md`), so `transpose(i)` is written as a function that
takes its track argument, not as a closure produced by an under-applied call. The function and block spellings of
transpose, stretch, retrograde, and inversion invoke the same semantic action, so their equality is an implementation
theorem rather than a duplicated convention.

The sole pitch traversal is

```text
map_note_pitches : (Pitch -> Pitch) -> EventTrack[WrittenTime, ScoreFact]
                                    -> EventTrack[WrittenTime, ScoreFact].
```

For an occurrence `o = ([a,b), f)` its action is `([a,b), mapPitch(g,f))`, where `mapPitch` changes the written pitch of
`Note` and `Grace` facts and is the identity on every other `FactKind`; sounded chord tones are represented by
simultaneous `Note` facts at this layer. In particular it does not map the tonic of `Key` or the root of `Harmony`. This
distinction is music-theoretic, not merely representational: a key signature and a Roman-numeral/chord analysis state a
tonal reading, while a written note states a sounded pitch. Rewriting the latter does not prove the former has changed.

The temporal-support law follows directly from `map_payloads` (`../kernel/03-denotational-semantics.md` D7, L9–L12): the
traversal changes no occurrence span and no term constructor, hence the multiset of pairs `(onset, span)` and the
enclosing track duration are identical before and after mapping. Identity and composition follow by cases on the
exhaustive `FactKind` table: on pitch-bearing facts they reduce to the corresponding function equations; on all other
facts both sides are the identity. Origin is deliberately finer: one `MapNotePitches` step is retained, so equality
holds under `≈facts`, not byte-for-byte payload equality.

Interval answers use Musa's signed pair of written diatonic steps and semitones. This follows the distinction between
generic interval size and specific quality in *Open Music Theory*, "Intervals" (`016-intervals.md`): `P8` therefore
means seven diatonic steps and twelve semitones, preserving spelling rather than reducing the answer to a MIDI offset.

## 4. Ownership

| Owner | Knows | Must not know |
| --- | --- | --- |
| `musa-language` | tokens, lossless CST, recovery, formatting | musical types, closures, core evaluation |
| private `musa-compiler` elaboration subsystem | `Type`, `Value`, `Closure`, modules, theory algorithms, quotation | public backend or DSP types |
| `musa-kernel` | exact time, coordinates, typed occurrences, term binding, `follow`, `together`, scaling | notes, scales, functions, profiles, samples, seconds, machines |
| compiler score projection | `ScoreFact`, context tracks, Origin, `ScoreSnapshot` | machine topology and audio buffers |
| compiler performance preparation | profiles, gestures, tempo, tuning, part lanes | a primitive's private state |
| `musa-dsp` | registered primitives, machine construction and validation, scheduling, prepared buffers | notation semantics and source CST |
| `musa-playback` | prepared machine, transport, real-time queues | parsing, allocation or machine construction in the callback |

The rejected alternative is a public `musa-elaboration` crate. Its only actual caller would be `musa-compiler`, while
its proposed public `Type`, `Value`, `Closure`, module environment, and theory APIs are volatile pass details. The
chosen design is one deep private compiler subsystem with the existing `musa_compiler::compile` facade. No public API
exists merely because this specification names a semantic object.

The sound side likewise rejects a combined mutable score-audio object. One deep preparation operation binds a compiled
score snapshot, profiles, selected instrument declarations, and mix declarations into an immutable prepared machine
while retaining their distinct editable source declarations.

## 5. Equality and provenance

Four relations serve different questions, and none of them is behavioural machine equality, which is not decidable
(`../across-stages/04-identity-and-realization.md` §4):

- `t ≡core u`: alpha-equivalent closed terms normalize to the same canonical core term, including payload bytes.
- `T ≈facts U`: their tracks are equal after the explicit projection that erases Origin and non-sounding stable
  identities, but preserves every musical fact, every exact span, and the coordinate.
- `m ≈material n`: for every placement where both are used, their results are `≈facts` and have equal durations.
- `M ≡struct N`: two machines have the same primitive ids, versions, and configurations at the leaves and the same
  wiring tree, within one registry. This is what a cache key may use, and nothing weaker.

Full `ScoreFact` equality is deliberately finer than `≈facts`: Origin paths and stable instance identities support
selection, diagnostics, realization choices, and caching. Transform, call, repeat, template-instance, quotation, and
context steps are attached by centralized syntax-directed elaboration. User code cannot inspect or forge Origin.

Caches key typed source, imported build closure, static environment, placement-relevant context, and compiler version.
Using only source text or only `≈facts` is unsound because the same value may be placed differently under different
scales or meter tracks.

## 6. Closure theorem required of implementation

If a project resolves, all declarations check, `Σ ⊢piece Δ ⇓ t : Term[ScoreFact]`, and resource checking accepts it,
then `t` is finite, closed, core-well-formed, and every payload decodes as `ScoreFact`. Core totality then gives a
unique finite `EventTrack[WrittenTime, ScoreFact]`. Prompts 93–119 establish the typing, normalization, closure,
quotation closure, and adapter lemmas; prompt 170 audits the end-to-end theorem against the implementation.

The corresponding claim on the sound side is stated where the machines are, not here: preparation determinism is R1 and
frame equality is R1-frames, both in `../across-stages/04-identity-and-realization.md`.
