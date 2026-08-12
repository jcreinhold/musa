# The Core Boundary — What Musa's Core Is a Calculus Of

**Status: governing.** This document sits under `docs/course-correction.md` and governs over `docs/language/` wherever
they differ. The amendment it makes to the course correction is written into `docs/course-correction.md` §36 in the same
commit; if that section is absent, this decision has not been made. Prompt 126 produced it, and prompts 127–146 are
costed against it below.

Peyton Jones (1987) §3 states the criterion this document applies: the translation into the core *is* the language's
semantics. A surface construct that translates into nothing has no semantics beyond whatever its compiler pass happens
to do that week. §3.1's strategy is the shape of the answer: an *enriched* language chosen so that the first step down
is a change of syntax rather than a change of meaning, and then transformations inside one calculus. Musa's enriched
language is the surface; its core is the temporal kernel. This document decides what the kernel is a calculus *of*.

## 1. The census

### Method

The row list is driven from `crates/musa-language/src/keywords.rs`, the table prompt 84 made exhaustive by construction:
`keyword_doc` is a wildcard-free match over `SyntaxKind`, and the workspace forbids wildcard arms, so a keyword the
lexer accepts and the table omits is a compile error. Exhaustiveness of *this* table against *that* one is checked by:

```sh
diff <(grep -A1 'doc!($' crates/musa-language/src/keywords.rs | grep -oE '"[A-Za-z_0-9]+",' \
        | tr -d '",' | LC_ALL=C sort -u) \
     <(grep -oE '^\| `[A-Za-z_0-9]+` \|' docs/core-boundary.md | tr -d '|` ' | LC_ALL=C sort -u)
```

Empty output is the pass condition. As of this commit both sides hold **88** spellings.

"Elaborates to" names the kernel `Term` form (`crates/musa-kernel/src/term.rs`) or the `ScoreFact` variant (`FactKind`,
`crates/musa-compiler/src/elaborate.rs`) the construct produces. **none** means the construct produces no kernel term
and no score fact; the fourth column says what happens to it instead. Line numbers are as of this commit and are
orientation, not a contract.

### The table

| Construct | Elaborates to | Site | Note |
| --- | --- | --- | --- |
| `piece` | none (its `score` does) | `elaborate.rs:815` `elaborate_score` | The compilation unit. Its header's meter/key/tempo become piece-scoped facts in `context_facts`. |
| `library` | none | `elaborate.rs:1232` `elaborate_libraries` | Declarations only. Material elaborates at the `use` site that names it, or not at all. |
| `tempo` | `FactKind::Tempo` | `elaborate.rs:3090` `elaborate_tempo` | Notation. The `Beat → Second` map that *reads* it lives in `performance.rs` and is not a kernel operation. |
| `meter` | `FactKind::Meter` | `elaborate.rs:2996`, `3069` | Occupies its own extent: a fact about the passage, not a property of a note. |
| `key` | `FactKind::Key` | `elaborate.rs:3115` `elaborate_key` | Same shape as meter, with `Override` inheritance per scope. |
| `subtitle` | none | `resolve.rs:719` | Front matter, into `ScoreSnapshot::front_matter`. Not temporal. |
| `composer` | none | `resolve.rs:720` | Front matter. |
| `arranger` | none | `resolve.rs:721` | Front matter. |
| `copyright` | none | `resolve.rs:722` | Front matter. |
| `motif` | `Let` + `Var` at each use | `elaborate.rs:3345`, `3374` | The declaration itself elaborates nothing; sharing is the kernel's `let`, and each reference carries its `@` mark. |
| `fragment` | `Literal` (a labelled segment) | `elaborate.rs:3743` `elaborate_fragment` | Material a `mobile` arranges. |
| `score` | `Over` of its parts | `elaborate.rs:815` `elaborate_score` | The composition `main`. |
| `part` | member of the score's `Over` | `elaborate.rs:762`, `815` | Part identity is `Scope::Part` in the payload, not a kernel index. |
| `voice` | `Seq` of its items | `elaborate.rs:1558` `elaborate_voice` | Voice identity is payload metadata (kernel Q3, resolved at prompt 119). |
| `clef` | `FactKind::Clef` | `elaborate.rs:3150` `elaborate_clef` | Part-scoped context fact. |
| `profile` | **none** | `resolve.rs:976` `parse_profiles` | Rides in the snapshot as a declaration; interpreted once, in `performance.rs`. **No calculus.** |
| `performance` | **none** | `resolve.rs:976` | The block that holds profiles. **No calculus.** |
| `use` | `Let`/`Var` with a mark, or an inline `Literal` | `elaborate.rs:3345` `elaborate_use` | The one place sharing is introduced. |
| `import` | none | `imports.rs`; `elaborate.rs:1232` | Name resolution. |
| `transpose` | payload map over the segment | `elaborate.rs:2260`, `4115` | Written intervals, applied to payloads; no kernel operation and no new term. |
| `up` | none | `elaborate.rs:2260` | Direction modifier inside `transpose`. |
| `down` | none | `elaborate.rs:2260` | Direction modifier inside `transpose`. |
| `rest` | `FactKind::Rest` occurrence | `elaborate.rs:2193` | Notation intent. Rest *glyphs* are the notation layer's decision (kernel 07). |
| `repeat` | `Seq` of copies; or a `List` literal | `elaborate.rs:2621`; `core.rs` | Two constructs sharing a spelling: music repetition and the value language's finite list. |
| `assert` | body's segment, plus an `Assertion` expansion step | `elaborate.rs:2924` `elaborate_assert` | Proof obligation. Adds provenance, never an occurrence. |
| `bar` | body's segment, named | `elaborate.rs:2854` `elaborate_bar` | A named bar is material a `use` can answer, so it also participates in sharing. |
| `ending` | `FactKind::Ending` + per-pass `Seq` | `elaborate.rs:2276`, `2782` |  |
| `slur` | `FactKind::Slur` interval | `elaborate.rs:2289` |  |
| `phrase` | `FactKind::Phrase` interval | `elaborate.rs:2294` |  |
| `mark` | `FactKind::Mark` | `elaborate.rs:1809` `elaborate_mark` | Notation. What a mark *does* to sound is the profile's decision, downstream. |
| `dynamic` | `FactKind::Dynamic` | `elaborate.rs:2384` | A marking, not a velocity and not a decibel (roadmap §2). |
| `groove` | **none** | `resolve.rs:1105` `groove_of`; `groove.rs` | A named timing feel, applied in `performance.rs`. **No calculus.** |
| `grace` | `FactKind::Grace` | `elaborate.rs:1752` `elaborate_grace` | Written, not read (kernel 07, prompt 71). |
| `tuplet` | `FactKind::Tuplet` + `Scale` | `elaborate.rs:2325` | The one notation fact that is also a time action. |
| `stretch` | `Scale` | `elaborate.rs:2343`, `3950` | A ratio on notated time. Not tempo. |
| `retrograde` | occurrence reversal within the extent | `elaborate.rs:2361`, `4049` | No term: the timeline is rebuilt. |
| `invert` | payload map | `elaborate.rs:2367`, `3964` |  |
| `shift` | `Shift` (sugar for `Seq(zero d, body)`) | `core.rs` `MusicOperation::Shift`; `term.rs:44` | Stays sugar by kernel 10's scope rule. |
| `overlay` | `Over` | `core.rs` `MusicOperation::Overlay` |  |
| `map_note_pitches` | payload map | `elaborate.rs:3994` |  |
| `around` | none | `elaborate.rs:2367` | Axis modifier inside `invert`. |
| `with` | positional payload edit | `elaborate.rs:3374` | Respelling on one occurrence of a `use`, recorded as an extra provenance step. |
| `note` | none | `elaborate.rs:3374` | Literal inside a `with` override. |
| `section` | `FactKind::Section` | `elaborate.rs:1129` | Form marker, piece-scoped. |
| `harmony` | `FactKind::Harmony` | `elaborate.rs:1173` | The chord-symbol lane. |
| `crescendo` | `FactKind::Hairpin` | `elaborate.rs:2301` | The *notation*. Its realization as a curve is the performance layer's, and today has no calculus. |
| `diminuendo` | `FactKind::Hairpin` | `elaborate.rs:2301` | As above. |
| `to` | none | `elaborate.rs:2301`, `3090` | Modifier on a hairpin or a gradual tempo. |
| `over` | none | `elaborate.rs:2301`, `3090` | Modifier: how far a gradual change reaches. |
| `senza` | `Scale` over an unmeasured region | `elaborate.rs:3026` `elaborate_senza` |  |
| `mobile` | `Seq` in a chosen order | `elaborate.rs:2476` `elaborate_mobile` | The choice is a compile parameter; the freedom is recorded on the payload (kernel Q2). |
| `improvise` | `FactKind::Improvise` | `elaborate.rs:2548` `elaborate_improvise` |  |
| `studio` | **none** | `studio.rs:591` | Compiles to `StudioSpec`, then a private finite typed process IR. No temporal/source term. |
| `patch` | **none** | `studio.rs:591` `declare_patch` | A process-description declaration, not a temporal term. |
| `bus` | **none** | `studio.rs:592` | As above. |
| `assign` | **none** | `studio.rs:623` | Which instrument realizes a part; resolved during process preparation. |
| `route` | **none** | `studio.rs:655` | A typed process connection. |
| `send` | **none** | `studio.rs:682` `resolve_send` | A typed process connection. |
| `modulate` | **none** | `studio.rs:683` | A process-control connection. |
| `master` | **none** | `studio.rs` | The final process boundary output. |
| `at` | **none** | `studio.rs:682` | The level of a send. |
| `output` | **none** | `studio.rs:591` | Where a patch's signal leaves. |
| `pitch` | none | `stdlib/src/pitch.musa` | A standard-library module name, not a construct. |
| `let` | none (binds a value) | `core.rs` | The value calculus. A `let` whose value is `Music` reaches the kernel through its use. |
| `fn` | none | `core.rs` | Total, monomorphized, strongly normalizing. Nothing about a kernel term remembers a function was involved. |
| `music` | whatever its body elaborates to | `elaborate.rs:3593` `elaborate_music_value` | The staged boundary: a music value elaborates at its *use* site, under that site's context. |
| `kernel` | the quoted term, spliced | `elaborate.rs:3686` `kernel_quote_segment` | Hygienic quotation of a term written directly (prompts 120–121). |
| `Option` | none | `core.rs` | Value calculus. |
| `List` | none | `core.rs` | Value calculus. |
| `match` | none | `core.rs` | Value calculus; total over a finite value. |
| `Some` | none | `core.rs` | Value calculus. |
| `None` | none | `core.rs` | Value calculus. |
| `true` | none | `core.rs` | Value calculus. |
| `false` | none | `core.rs` | Value calculus. |
| `scale` | none (a value) | `scale.rs`; `elaborate.rs:1924` | Reaches the kernel only as context that other constructs read. |
| `degree` | none (a value) | `scale.rs` | An ordinal with no octave. |
| `frame` | none (a value) | `scale.rs` | A collection that knows its register. |
| `in` | body's segment under a scale | `elaborate.rs:1989` `elaborate_in_scale` | Context, plus an expansion step on every fact inside. |
| `step` | none | `scale.rs`; `elaborate.rs:1943` | Resolves a pitch against the collection in force. |
| `chord` | none (a value) | `chord.rs` | Rooted spelled content, with no register. |
| `stack` | occurrences in close position | `elaborate.rs:2021` `elaborate_stack` |  |
| `template` | none | `template.rs` | Parameterizes a piece or a voice; expansion is binding, not rewriting. |
| `make` | none | `template.rs` | Instantiation is generative: identity comes from the site. |
| `signature` | none | `module.rs` | Seals a structure's surface. |
| `mod` | none | `package.rs` | One child of a package's module tree. |
| `structure` | none | `module.rs` | Groups declarations behind a signature. |
| `module` | none | `module.rs` | The old spelling of `structure`. |
| `as` | none | `module.rs` | Names what an instance makes. |

### What the census shows

Three groups, and the boundary runs between the second and the third.

1. **Notation and the file's structure** — 53 spellings. Forty produce a kernel term or a `ScoreFact`; six are modifiers
   written inside one (`up`, `down`, `around`, `note`, `to`, `over`); seven are the compilation unit and its metadata
   (`piece`, `library`, `import`, `subtitle`, `composer`, `arranger`, `copyright`). The kernel is total over all of
   them.
2. **The value and module layers** — 22 spellings elaborate to nothing *because they are not temporal*. They are the
   enriched language above the core, in exactly Peyton Jones's sense: `let`, `fn`, `match`, `Option`, `List`, templates,
   structures, signatures. Their semantics is the value calculus of `docs/language/02-core-calculus.md`, which is itself
   a core. Their `none` is correct and permanent.
3. **Performance and sound** — 13 spellings (`profile`, `performance`, `groove`, `studio`, `patch`, `bus`, `assign`,
   `route`, `send`, `modulate`, `master`, `at`, `output`) elaborate to no *temporal term*. Performance belongs to a
   second temporal payload; studio declarations prepare a private finite process definition with its own formation and
   tick semantics. Neither becomes a source value merely because its implementation has a calculus.

Group 3 splits again, and the split is the whole answer:

- `profile`, `performance`, `groove`, and the realization of `dynamic`/`crescendo`/`diminuendo`/`mark`/`slur`/`senza`
  produce **an object with musical extent**: gestures and control curves positioned on exact rational time. Prompt 130's
  Design already specifies `E(b) = d0 + (d1-d0) * p((b-s)/(e-s))` over `[s,e]` with a kernel `Progress p`. That is a
  timeline. It is a timeline whose payload is a gesture rather than a notation fact.
- `studio`, `patch`, `bus`, `send`, `route`, `modulate`, `master`, `at`, `output` produce **a dataflow graph**: typed
  ports, delay-gated cycles, no extent (roadmap §13.3). That is not a timeline and never will be.

### The two premises, tested

Prompt 126 named two claims as premises to confirm or refute rather than assume. Both are **confirmed**.

**Premise 1 — `Timeline<A>` and `Occurrence<A>` are payload-polymorphic and instantiated only at `ScoreFact` outside the
kernel.** Confirmed, and the stronger fact matters more:

```sh
for t in Timeline Occurrence Term; do
  grep -rn "$t<" crates --include='*.rs' | grep -v '^crates/musa-kernel/' | grep -oE "$t<[A-Za-z_:]+>" | sort | uniq -c
done
```

Outside `musa-kernel`: `Timeline<ScoreFact>` 25 sites, `Occurrence<ScoreFact>` 32 sites plus one `Occurrence<_>`
inference placeholder, `Term<ScoreFact>` 17 sites. No other instantiation exists anywhere in the workspace.

The stronger fact: **`musa-kernel` is already a calculus of occurrences of anything canonical, and its laws are already
proved at a payload that is not `ScoreFact`.** `Term<A>`, `Timeline<A>`, and `Occurrence<A>` are generic in `A`; the
whole payload contract is one method, `Canonical::canonical_key(&self) -> String` (`occurrence.rs:10`). The law suite
`crates/musa-kernel/tests/laws.rs` generates `Occurrence<u8>` and proves L1–L24 there. The kernel has never been a
calculus of score facts. `ScoreFact` is the only payload the *compiler* has ever instantiated it at.

**Premise 2 — `GestureTimeline` is named in design documents and in no line of code.** Confirmed.
`grep -rn GestureTimeline crates --include='*.rs'` returns zero lines. Before this commit the name appeared eleven times
in `docs/` and nowhere else: `initial-design-roadmap.md` (twice), `course-correction.md`,
`language/08-performance-and-sound.md` (three times), `language/00-semantics.md`, and prompts 92 and 130. Two prompts
name the object; nineteen were made contingent on deciding what kind of object it is.

## 2. The candidates

- **A. Score-only core (the status quo).** The kernel is a calculus of notated occurrences. Performance, instruments,
  and sound are compiler pipelines with no calculus, and the census's group-3 `none` rows are permanent.
- **B. One kernel, several payloads.** The same finite temporal calculus, instantiated at `ScoreFact`, at a
  gesture/control payload, and at whatever the instrument boundary requires. `timeline`/`sequence`/`overlay`,
  normalization, semantic equality, and the semantic hash are reused rather than reimplemented per layer.
- **C. One dependently typed core.** Payloads and the indices that constrain them — part, voice, meter, tuning,
  transposition — become types in a single dependently typed calculus checked by normalization by evaluation.

## 3. The five tests

### 3.1 Translation totality

Every census row is translated, or the candidate is rejected by naming the rows it cannot serve.

| Candidate | Group 1 (notation) | Group 2 (value/module) | Group 3a (performance) | Group 3b (studio) |
| --- | --- | --- | --- | --- |
| A | yes | yes, by the value calculus | **no** — permanently `none` | no, permanently `none` |
| B | yes | yes, by the value calculus | **yes** — `Timeline[Gesture]` | no, by decision (§5) |
| C | yes | yes | yes | no, for the same reason as B |

**A is rejected here.** The rows it cannot serve are `profile`, `performance`, `groove`, and the realization half of
`dynamic`, `crescendo`, `diminuendo`, `mark`, `slur`, and `senza` — nine spellings whose product is an exactly-timed
object over a rational extent. Under A, that object exists (prompt 130 must build it) but is defined by no calculus, so
its ordering, its equality, its identity, and its behaviour under `sequence`, `overlay`, and `scale` are whatever the
pass does. Peyton Jones §3's criterion applied literally: those nine constructs would have no semantics. That is a
census row A cannot serve, not a preference.

A survives on group 3b, and so does everything else. Nothing translates a signal graph into a timeline; see §5.

### 3.2 Law survival

For each law of `docs/kernel/04-algebraic-laws.md` and the theorems of `10-term-calculus.md`: does it hold at a new
payload unchanged, hold with a new proof, or fail?

| Law | At a second payload under B | At a second payload under C |
| --- | --- | --- |
| L1–L3 (sequence) | **Unchanged.** Quantify over timelines, never over `A`. Proved at `u8` today. | Unchanged, but restated over indexed types. |
| L4–L6 (overlay) | **Unchanged.** | Unchanged, restated. |
| L9–L12 (payload map) | **Unchanged.** Quantify over `f : A → B` for arbitrary `A`, `B`. | Unchanged only if the index is preserved by `f`; `f` must become index-respecting, which is a **new proof**. |
| L13–L15 (time scaling) | **Unchanged.** | Unchanged, restated. |
| L16–L17 (restriction) | **Unchanged.** | Unchanged, restated. |
| L18 (synchronized interchange) | **Unchanged.** | Unchanged, restated. |
| L19 (equality is a congruence) | **Unchanged**, given `Canonical`. | **New proof.** Congruence must be shown for the conversion relation, not just for occurrence multisets. |
| L20–L23 (queries) | **Unchanged.** `prevailing` takes a selector `σ` over payloads; a gesture payload supplies its own. | Unchanged, restated. |
| L24 (span-alone for `Progress`) | **Unchanged**, and it is the law that makes a control curve a payload *value*. | Unchanged. |
| X1–X3 (non-laws) | **Unchanged.** X3 in particular: no `join`, at any payload. | Unchanged. |
| T1 (constructors are a homomorphism) | **Unchanged.** | Unchanged. |
| T2 (`let` is transparent) | **Unchanged.** | **New proof.** Transparency must survive type-level occurrences of the bound name. |
| T3 (evaluation is normalization) | **Unchanged.** | **New proof**, and it is the hard one: normalization by evaluation for a dependent theory. |
| T4 (totality; deterministic, no recursion) | **Unchanged.** The term grammar is unchanged and still finite and first-order. | **New proof.** Totality stops following from the grammar and becomes a metatheorem about the whole system. |
| T5 (observation commutes with sharing) | **Unchanged.** | Unchanged. |
| T6 (instantiation preserves denotation up to payloads) | **Unchanged.** | Unchanged. |

B needs **no new proof of any temporal algebra law**, because it adds no term or temporal operation. The obligation it
does create is on identity: a second payload supplies a deterministic total key, an owner id, and a quotient version.
Key equality defines the admitted payload equality and may deliberately identify raw stored values whose omitted fields
are presentation-only. The persisted temporal record must frame arbitrary key bytes exactly. `docs/kernel/` states this
at prompt 129a.

**C is rejected here**, by a law of the calculus rather than by tone. `docs/kernel/10-term-calculus.md` states the
kernel's **scope rule**: a form belongs to the calculus only if it serves sharing, deferred observation, or interchange,
*and* its meaning is a timeline that `03-denotational-semantics.md` already defines. Π-types, Σ-types, universes, and a
conversion judgment serve none of the three, and D1–D12 give them no denotation. Admitting them is not an extension of
the calculus; it is a replacement of it, which is why T2, T3, T4, and L19 all need new proofs above. The scope rule was
written to be applied, and this is the case it was written for.

### 3.3 Layer separation

Roadmap §2's table must survive intact.

| §2 distinction | Under A | Under B | Under C |
| --- | --- | --- | --- |
| written pitch ≠ MIDI number | kept | kept — the gesture payload carries written pitch until tuning | kept |
| notated duration ≠ performed duration | kept | kept — two timelines, two payloads, one map between them | kept |
| voice ≠ mixer track | kept | kept — voice is payload metadata; a mixer track is a `StudioSpec` node with no extent | **at risk** |
| part ≠ synthesizer | kept | kept — `PartId` rides the gesture payload; the instrument instance is bound at preparation | **at risk** |
| dynamic marking ≠ decibels | kept | kept — the marking is a `ScoreFact`, the amplitude is a gesture control, the gain is a graph parameter | kept |
| motif definition ≠ its expansions | kept | kept | kept |
| source ≠ widget state | kept | kept | kept |

B keeps every row, and keeps them the way §2 intends: by putting the objects in *different timelines with different
payloads*, not by naming them differently inside one. The reason B does not collapse "notated duration ≠ performed
duration" is exactly that it does not fuse the two timelines; it says they are two instances of one calculus, which is a
claim about the *algebra of time*, not about the vocabularies. Music theory keeps notation, performance, and
orchestration as separate vocabularies (OMT 007, 074, 118, 114–115) while using one arithmetic of duration for all
three; B is that arrangement and nothing more.

C's two "at risk" entries are honest hedges rather than proofs. C's stated attraction is making part, voice, meter,
tuning, and transposition *indices of the core type*. Once the score's part index and the mix's channel index inhabit
one index language, §2's separation is maintained by the type system's discipline rather than by the objects being
different objects. That is a weaker guarantee than the one §2 currently has, and it is a cost, but it is not on its own
a collapse. C is rejected by §3.2, not by this table.

### 3.4 Cost in prompts

See §7 for the full ledger. In summary: A costs zero prompts and leaves nine census rows without semantics. B repairs
five prompts, adds one, and deletes none. C rewrites the kernel specification (twelve documents), invalidates the law
suite as written, and makes every prompt from 130 onward contingent on a metatheory that does not exist yet — the
cheapest honest estimate is that prompts 127–146 would be replaced rather than repaired.

### 3.5 Cost to undo

- **A → B, later.** Cheap and mechanical. The gesture object exists either way; reversing means changing what type it is
  and re-deriving ordering and equality by hand. One prompt, no data loss. This is the *reason* A is not catastrophic —
  but "cheap to fix later" is not an argument for shipping nine constructs with no semantics, and prompt 130 must build
  the object now.
- **B → A, later.** Cheap. Stop instantiating the kernel at the second payload and write the bespoke structure. The
  kernel is untouched by B, so there is nothing to withdraw from it.
- **B → C, later.** The normal cost of adding a type theory to a language that has a settled core: large, but ordinary,
  and it is exactly the cost C has today. B does not make C harder.
- **C → anything, later.** **This is the whole argument against C, and it is written down rather than gestured at.**
  Once indices are load-bearing, every term, every law, every proof, and every downstream consumer is stated in terms of
  them. You cannot un-index a payload: a `Timeline[Gesture @ part p, voice v]` does not become `Timeline[Gesture]` by
  deletion, because the operations that were type-correct only under the index have no meaning without it, and the
  callers that relied on the index to establish an invariant have to re-establish it some other way. Reversal is a
  rewrite of `musa-kernel`, `musa-compiler`, the twelve documents of `docs/kernel/`, and the law suite. Musa has one
  falsifiable reason to want C — that a wrong part/voice pairing should be a type error rather than a diagnostic — and
  that reason is served today by the resolver at a cost of one diagnostic each. The decision to pay a rewrite for it is
  not one this project has evidence to make.

## 4. The decision

**Candidate B. The core is a calculus of occurrences of a canonical payload over exact rational time — and it always
was.** `ScoreFact` is one payload. A gesture/control payload is a second. The temporal algebra and generic carrier do
not change. The identity contract is repaired: current display-text hashing is not uniquely framed and the old
“injective-on-values while quotienting fields” wording is contradictory.

What changes is the compiler's answer to the question "what is a performance?" Today: a `Vec` of scheduled events built
by a pass. Under B: a `Timeline[Gesture]`, normalized, semantically equal or not, semantically hashed, and obeying
L1–L24 without a new line of proof. Prompt 130's `GestureTimeline` becomes an instantiation rather than an invention.

The obligations this creates, in order:

1. **Satisfied by `docs/kernel/12-payload-admission.md`: state the payload-admission and exact-framing rules before
   admitting a payload.** A payload key is deterministic and total, names its owner/quotient version, and defines
   admitted equality. It emits no address, hash-ordered iteration, or float. A versioned length-framed temporal encoder,
   separate from display, is the basis of hashing. L1–L24 and T1–T6 transport at every admitted payload, with L24
   conditional on `Progress`. Prompt **129a**.
2. **Instantiate at the gesture payload** — prompt 130, repaired to say "instantiate the kernel" rather than "define
   exact internal `GestureTimeline`/`GestureLane` values".
3. **Do not instantiate at the signal boundary** — §5.

### Where this document governs

Over `docs/language/` wherever they differ, in particular `docs/language/08-performance-and-sound.md`, whose
`GestureTimeline` prose predates this decision and now means "the kernel at the gesture payload". Under
`docs/course-correction.md`, amended at §36 in this commit.

## 5. The signal question

**Signals stay out of the core.** Decided in the terms prompt 126 required, not by taste.

A DSP signal is **coinductive**: an unbounded stream, defined by what it produces at each observation, consumed at a
sample rate, with no last sample. The temporal kernel is **inductive, finite, and total** — a timeline is a rational
extent plus a *finite* multiset of occurrences, and T4's totality is derived from that finiteness. The two have
different induction principles. Putting a signal in an occurrence payload would require either that the payload be
finite (then it is not a signal) or that the kernel's finiteness be given up (then T4, and every law that rests on
normalization terminating, is gone). Kernel Q1 ("infinite / live patterns") has stood open since prompt 8 for this
reason and stays open.

There is a second, independent reason, and it is musical rather than metatheoretic. A signal graph has **no extent**. It
is a typed-port dataflow DAG with delay-gated cycles (roadmap §13.3); asking for its duration is a category error the
way asking for a mixing desk's duration is. Every kernel law is stated over `duration(M)`; a payload whose carrier has
no duration is not a payload the kernel can be a calculus of.

### The object that crosses

**The prepared execution.** The compiler produces `Timeline[Gesture]` — exact, finite, normalized, and versioned. Audio
preparation (prompt 131) binds its semantic projection, instrument/mix bindings, realization seed, and complete options
into an opaque prepared execution containing a finite typed process definition and initial resources. That is what the
audio layer holds. The boundary is one function, and it is the only place a rational becomes a float.

### The law

Written into `docs/kernel/07-backend-contract.md` by prompt 129a:

> **R1 — execution preparation factors through meaning.**
> `prepare_execution : Sem_Gesture × Bindings × Seed × Options → Result PreparedExecution PrepareError` is pure and
> deterministic. Equal complete arguments produce an equal complete result. Presentation-only data is handled by a
> separate lineage pass and cannot affect execution. Equal rendered frames additionally require equal external inputs,
> initial state/allocation semantics, and conforming deterministic processors.

R1 permits a sound-side cache only when its versioned argument record includes **all** four inputs and lookup confirms
their exact framed bytes after digest lookup. Hash equality alone is never a cache hit. Prompt 144 measures the
factorization and runtime premises; prompt 145 audits them.

### What reopens this

Deferral without a trigger is forbidden by this prompt's own Stop. The signal question reopens on **either** of two
events, and on nothing else:

1. **A surface construct is proposed whose meaning is a signal with musical extent** — a written `.musa` form that both
   sounds continuously and has a notated duration that other music is positioned against. Live coding and reactive input
   are the obvious sources. This is kernel Q1's trigger, and it reopens Q1 rather than this document.
2. **R1 is measured false.** If prompt 144 finds equal complete semantic arguments whose preparation results differ, the
   operation is observing omitted or ambient state and the boundary is wrong. If preparation agrees but execution
   differs, the failed allocation/input/processor premise is identified separately rather than blamed on the temporal
   kernel. The report belongs here either way.

## 6. What this forbids

A later prompt may not quietly re-open any of these. Re-opening requires amending this document and
`docs/course-correction.md` §36 together.

1. **No fourth combinator.** `timeline`, `sequence`, `overlay`, `scale`, `restrict`, `let`, and the reference are the
   forms. A surface convenience that cannot be elaborated from them is a finding to report, not a kernel change. The
   scope rule of `docs/kernel/10-term-calculus.md` is the test.
2. **No bespoke temporal structure above the kernel.** If an object has a rational extent and finitely many things
   positioned in it, it is a `Timeline<A>`. Writing a second one — a `Vec<(Beat, Beat, T)>` with its own ordering and
   its own equality — is the defect this decision exists to prevent. Prompt 145 audits for it.
3. **No payload without an admission record.** A new payload requires a `Canonical` implementation and a row in the
   admission table `docs/kernel/` gains at prompt 129a, stating what its key includes and what it deliberately quotients
   away.
4. **No signal in a payload**, and no coinductive value in a payload, and no absolute time (seconds, frames, samples) in
   a payload. Physical time appears at the prepared-plan boundary and nowhere earlier.
5. **No dependent indices in the kernel.** Part, voice, meter, tuning, and transposition stay payload data and resolver
   facts. §3.5 is the reason and it does not expire.
6. **No `join`.** X3 stands at every payload: nothing flattens `Timeline[Timeline[A]]`.
7. **No temporal or universal source calculus under the studio.** `StudioSpec` stays a finite typed-port description
   with no musical extent. It elaborates to the private process calculus in `docs/spec/03-process-calculus.md`:
   whole-node scheduling, first-order total transitions, and feedback only through explicit registers. This formal
   semantics is required; it does not make process nodes source values or temporal payloads.
8. **No second kernel crate, and no kernel dependency on a musical type.** `musa-kernel` stays a leaf that knows nothing
   musical. A gesture payload is defined in `musa-compiler` and implements `musa-kernel`'s trait, in exactly the
   direction `ScoreFact` already does.

## 7. The ledger over prompts 127–146

Every prompt in the range is costed. "Repaired" means its Design changes in this commit; "unchanged" means the decision
leaves it exactly as written, and its **Contingent on prompt 126** banner is replaced with a pointer to this document.

| Prompt | Verdict | What changes |
| --- | --- | --- |
| 127 elaboration-performance-closure | **unchanged** | Score-side measurement. Carried no banner and needs none. |
| 128 studio-vocabulary | **repaired** | Vocabulary remains a closed description surface; its target is the private process calculus, not an untyped graph or temporal term. |
| 129 exact-studio-values | **unchanged** | Exactness until the one float boundary is R1's precondition. Banner replaced. |
| **129a payload-admission-rule** | **repaired** | States payload equality/schema admission, replaces the refuted display-text hash with exact framing, and writes exact R1 into `07-backend-contract.md`. Must land before 130. |
| 130 performance-gestures | **repaired** | `GestureTimeline` becomes `Timeline[Gesture]` — an instantiation of `musa-kernel`, not a new structure. `depends_on` gains 129a. The prompt keeps its Target; what changes is that ordering, equality, and hashing come from the kernel rather than being re-specified. |
| 131 instrument-contracts | **repaired** | Preparation takes complete semantic arguments/options and returns a complete result; presentation lineage is separate. The private plan implements the process calculus. |
| 132 part-instrument-routing | **unchanged** | `PartId` on the gesture payload is what B already implies. Banner replaced. |
| 133 expressive-control-realization | **unchanged** | Controls are payload values; resolution to private parameters is downstream of the boundary. Banner replaced. |
| 134 ergonomic-sound-bindings | **unchanged** | Surface ergonomics over independent declarations. Banner replaced. |
| 135 reproducible-assets | **unchanged** | Banner replaced. |
| 136 pinned-package-imports | **unchanged** | Banner replaced. |
| 137 sampler-runtime | **unchanged** | Banner replaced. |
| 138 sfz-instruments | **unchanged** | Banner replaced. |
| 139 soundfont-instruments | **unchanged** | Banner replaced. |
| 140 media-cue-semantics | **repaired** | The distinction it draws — beat-fitted clip versus fixed-physical-duration cue — is exactly the §5 boundary. A musical clip has extent and is positioned in a timeline; a cue's asset keeps its physical duration and therefore belongs to the prepared plan. Design says so explicitly. |
| 141 audio-clips | **unchanged** | Banner replaced. |
| 142 sound-mix-workbench | **unchanged** | Banner replaced. |
| 143 audio-language-tooling | **unchanged** | Banner replaced. |
| 144 audio-performance-closure | **repaired** | Measures exact R1 arguments/results, collision-confirmed caching, process conformance, and host-block partition independence. |
| 145 audio-conformance | **repaired** | Audits every §6 rule, including the required private process calculus rather than the former “no calculus” claim. |
| 146 language-conformance | **unchanged** | Graduation criteria are untouched; this decision changes what the prompts do, not whether they are checked. Banner replaced. |

**Deleted: none.** No prompt requires indexed CBPV or a signal-valued temporal kernel. The later process/identity review
repaired 128, 129a, 131, 144, and 145 more deeply than the original ledger: a studio needs private formal process
semantics, and correctness-sensitive identity/cache claims need exact framing and complete arguments. These are
deliberate amendments to the ledger, not evidence for a universal kernel.

### Why 129a is inserted as a suffixed rank

`scripts/renumber-prompts.py`'s naming grammar is `NN[suffix]-slug.md` and its audit keys on the full label, so `129a`
is a legal rank between 129 and 130 that introduces no duplicate and no gap. `make-room --at 130` would have been the
other route, and it would have renumbered prompts 130–153 including six finished prompts (147–152), rewriting their
frontmatter and every reference to them in exchange for nothing. This prompt's Stop forbids renumbering finished prompts
to tidy a range; a suffixed insertion is the minimum disturbance that puts the admission rule before the first
admission. `python3 scripts/renumber-prompts.py audit` is the check that it worked.

## 8. What was read

Peyton Jones (1987) §§1.2–1.3, 2.1–2.3, 3.1–3.2. `docs/course-correction.md`. `docs/kernel/00-purpose.md`,
`03-denotational-semantics.md`, `04-algebraic-laws.md`, `05-normalization.md`, `07-backend-contract.md`,
`08-open-questions.md`, `10-term-calculus.md`. `docs/language-correction.md`, `docs/language/00-semantics.md`.
`crates/musa-kernel/src/{term,timeline,occurrence}.rs` and `tests/laws.rs`.
`crates/musa-compiler/src/{elaborate,studio,profile,performance,groove,resolve,core}.rs`. Roadmap §2, §13.1–13.3. OMT
`007-other-aspects-of-notation.md`, `074-swing-rhythms.md`, `114-core-principles-of-orchestration.md`,
`115-subtle-color-changes.md`, `118-metrical-dissonance.md`.
