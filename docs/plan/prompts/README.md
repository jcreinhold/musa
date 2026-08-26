# Musa Implementation Prompts

**Status: the work plan.** A prompt that contradicts a governing document is repaired and committed before it is
implemented, per execution rule 5.

This directory is the executable work plan for building musa according to [`../roadmap.md`](../roadmap.md), with the
event-track governed by [`../../rules/events/`](../../rules/events/README.md) and the desktop interface by
[`../../rules/desktop/`](../../rules/desktop/README.md). Each numbered prompt delivers one feature and builds on the
prompts it depends on. Work them in dependency order; when in doubt, work them in numeric order.

`docs/rules/` owns identity-level commitments. `docs/rules/across-stages/` owns the current cross-stage semantics, and
`docs/plan/code-map/` maps those rules to current code. Prompts 127a–127e and 171–174 deliberately replace the current
split between the temporal core and a separate studio calculus. Prompt 127a amends the rules first; no code prompt may
implement the new design against stale rules.

**Where the roadmap and the events specification disagree** — most importantly, on the rule that the surface grammar
does not define the ontology and that a small event-track core (`empty`, `event`, `follow`, `together`, `map_payloads`,
`duration` over exact rational ambient time) is the semantic core — the event track wins. Prompts 08–12 specify,
implement, prove, and install it; prompts 13+ proceed exactly as before on top of it.

**Where the roadmap is silent on the desktop interface** — its visual language, engraving quality, interaction model,
states, and performance budgets — `docs/rules/desktop/` is the authority. Roadmap §14 still fixes the architecture.
Prompts 20–26 implement `docs/rules/desktop/`; prompt 26 graduated it from candidate to governing, as prompt 12 did for
the event track.

**The elaboration-language specification has governed since prompt 193.** Prompt 92 wrote the first candidate. Prompts
127a–127e and 171–174 replace its contextual `Music` core with the reviewed event-track and machine calculus in one
clean break. Prompts 175–192 then implement, measure, and audit performance, sound, assets, and packages on that base.
Prompt 193 made the resulting language governing after the complete conformance matrix passed. The constitution,
across-stage rules, and adjacent stage specifications remain authoritative at their boundaries under the
[precedence ladder](../../README.md#which-document-wins).

## Prompt anatomy

Each prompt is a markdown file `NN[suffix]-<slug>.md` with YAML frontmatter. The number is an execution rank, not an
identity. A lowercase suffix (`155a`) inserts a prompt between two existing ranks without renumbering anything after it,
and is the right insertion when the ranks that would move include finished work; `scripts/renumber-prompts.py make-room
--at N` is the right insertion otherwise. `id` carries the suffix, and so does every `depends_on` that names the prompt.

```yaml
---
id: NN
slug: short-name
status: pending # pending | in-progress | done | superseded
depends_on: [NN, ...] # earlier prompt ids
phase: 1 # roadmap phase this feature belongs to
---
```

`superseded` means a later prompt absorbed this one's whole Task. The file stays — completed prompts, research notes,
and the ledger link to it, and a dangling link buys nothing that deleting the file was worth — but it carries a banner
naming the prompt that absorbed it and is never executed. Its row stays in the sequence overview so the table remains a
complete index of the directory.

Body sections (a prompt omits a section when it has nothing to add):

- **Task** — the one feature or outcome, in a paragraph.
- **Read** — pointers into the design roadmap and into files produced by earlier prompts. Context, not restatement: the
  prompt cites; the roadmap explains.
- **Design** — prompt-specific decisions. Public facade signatures and key types are fixed here where the roadmap
  already fixes them; internals are left to the worker under the conventions below. If implementation evidence forces a
  deviation, fix the code **and** update the prompt in the same commit.
- **Target** — exact deliverables: crates/modules touched, public items added, example or fixture files created.
- **Check** — the exact commands that must pass before the prompt is done.
- **Stop** — explicit out-of-scope items. Do not implement them "while you're in there."

## Execution rules

1. A prompt's dependencies must be `done` before it becomes `in-progress`.
2. Flip `status` to `in-progress` when you start; flip to `done` in the same commit that makes its **Check** pass.
3. Commits are prompt-bound: one prompt, one commit (or one small squashed set). Do not mix two prompts' work in a
   commit.
4. Every prompt's Check includes, scoped to the crates it touches:
    - `cargo nextest run -p <crate>` (fall back to `cargo test -p <crate>` if nextest is not installed);
    - `cargo clippy --all-targets -p <crate> -- -D warnings` — the workspace lints are strict on purpose; fix the code,
      don't allow-list the lint;
    - `cargo fmt --check` — but `make fmt-check` instead whenever the prompt writes Markdown, TOML, `packages/`, or
      `apps/musa-desktop/ui`, which for a documentation or specification prompt means always. `cargo fmt` reads only the
      Rust half; root `AGENTS.md` requires all four formatters green before a commit, and a prompt whose Check names
      only the Rust one is how prompts 158 through 161 each shipped Markdown that `mdwright` would rewrap, ten files'
      worth by the time commit `9904677a` swept them. `make fmt-check` runs `cargo fmt --all --check` itself, so it
      replaces the line rather than joining it;
    - the prompt's behavior checks on `examples/*.musa` fixtures.
5. If a prompt turns out to be mis-scoped (two independent features, or a missing prerequisite), repair the prompt files
   first, run `python3 scripts/renumber-prompts.py audit`, commit that repair, then implement.

## Conventions every prompt follows

These are stated once here and referenced by every prompt. They come from the roadmap itself (§2, §3, §10.6, §13.2, §17)
and apply the deep-module discipline the roadmap prescribes.

### Deep modules

- State each new public API and its invariants as doc comments **before** implementing it.
- No public item without a caller in this prompt or a named later prompt. Keep it private until then.
- No pass-through wrappers; every layer must hide real work (parser recovery, motif expansion, graph topological sort,
  CPAL negotiation) behind a narrow surface.
- A trait with one implementor is a concrete type. Genericity over imagined pitch systems, temperaments, or backends is
  explicitly rejected (roadmap §8.1, §19).
- Compiler pass types, Rowan internals, DSP internals, and CPAL types never cross a crate boundary (roadmap §10.6, §15).

### Layer separation

The roadmap §2 table is a checklist. When a prompt touches two layers, verify none of these pairs collapse into one
representation: written pitch / MIDI number, notated duration / performed duration, voice / mixer track, part /
synthesizer instance, dynamic marking / decibels, articulation / gate multiplier, motif definition / its expansions,
score ordering / DSP ordering, project source / widget state, audio graph / visual layout.

### Source declarations versus host boundaries

Apply `docs/rules/language/00-semantics.md`'s ownership test before assigning a sound or musical noun to a Rust crate.
Data families, records, total functions, policies, standard controls, profiles, instrument signatures/mappings, studio
descriptions, sample maps, instruments, and presets that can be expressed through public values and operations belong in
ordinary `.musa` source, normally `stdlib/`. Registered primitive state and resource contracts, provenance-preserving
track construction, verified asset bytes, scheduling, DSP conversion, compact prepared indices, and real-time state
remain host-owned.

A Rust representation of a source declaration is a caller/runtime projection, never a second authoritative language: it
is not independently constructible, every field derives from one checked source value, and exact differential laws hold
the projection to that value. Tooling derives source vocabulary from declarations and source indexes, not a handwritten
Rust catalogue. Dependent relationships use ordinary source indices and the existing Miller-pattern unifier; no
domain-specific inference table, default, coercion, or host-side guess is permitted. Note 79 records the repair that
made this convention explicit.

### Tests

- `insta` snapshots for CST shapes, diagnostics, formatting, and MEI/LilyPond/MusicXML output (roadmap §17.1).
- `proptest` for the algebraic laws of roadmap §17.2 at the prompt that introduces each operation, and the formatting
  laws of §17.3 at the formatter prompt.
- DSP prompts: determinism, silence-for-disconnected, and NaN/infinity-free tests (§17.5).
- The engine prompt: callback instrumentation proving no allocation, locking, or large destruction on the audio thread
  (§13.2, §17.5).

### Performance

No speculative optimization. The real-time rules of roadmap §13.2 are **design constraints** at the audio/engine prompts
(preallocated render plans, `rtrb` queues, retired-plan return channel), verified by instrumentation — not by
intuition-driven tuning. Anything slower must be measured on a real workload before it is "fixed."

## Sequence overview

The table says what each prompt delivers, and deliberately does **not** say whether it is done: status lives in one
place, each prompt's own `status` frontmatter, so that a column here cannot go stale against it. To see where the work
stands:

```sh
grep -l 'status: pending' docs/plan/prompts/*.md      # queued
grep -l 'status: in-progress' docs/plan/prompts/*.md  # being worked now
python3 scripts/renumber-prompts.py audit             # the whole stack, in dependency order
```

A `done` prompt's file stays here after it lands. It is the record of what that commit was asked to do, and later
prompts cite it by number, so it is history rather than clutter — most of this directory is finished work.

| # | Prompt | Phase | Delivers |
| --- | --- | --- | --- |
| 01 | workspace-skeleton | 0 | Seven crates, facades, CLI stub |
| 02 | lexer | 1 | Trivia-preserving tokens |
| 03 | parser-cst | 1 | Lossless CST, recovery, typed wrappers |
| 04 | formatter | 1 | `musa format` |
| 05 | compiler-core | 1 | `musa check`; ScoreSnapshot + provenance |
| 06 | motifs-and-transforms | 1 | Motifs, repeat, transpose, expansion laws |
| 07 | notation-plan | 1 | Measures, beaming, tie decomposition |
| 08 | events-spec | 1 | `docs/rules/events/` event-track specification (candidate) |
| 09 | event-track | 1 | `musa-events`: timeline/sequence/overlay/restrict/scale/normalize |
| 10 | events-laws | 1 | Algebraic law proofs incl. non-laws |
| 11 | events-elaboration | 1 | Surface → event-track elaboration + differential parity |
| 12 | events-switch | 1 | The event track becomes the canonical semantics |
| 13 | mei-export | 1 | `musa render --to mei` |
| 14 | lilypond-export | 1 | `musa render --to lilypond` |
| 15 | performance-plan | 1 | Tempo as `Beat → Second`; frame scheduling |
| 16 | audio-core | 1 | Graph→RenderPlan compiler, offline blocks |
| 17 | polysynth-wav | 1 | `musa render --to wav` |
| 18 | engine-transport | 1 | `musa play`; CPAL + transport |
| 19 | project-session | 1.5 | ProjectSession, undo, exports |
| 20 | interface-prototype | 1.5 | Design system, bundled fonts, worker engraver, static Compose screen, screenshot goldens |
| 21 | desktop-shell | 1.5 | Tauri command boundary, live snapshots, stale-revision behavior |
| 22 | score-engraving | 1.5 | Anchored re-render, zoom re-layout, virtualization, raster goldens |
| 23 | score-interaction | 1.5 | Selection + caret, keyboard map, palette, playhead, a11y floor |
| 24 | origin-view | 1.5 | Provenance lens, occurrence selection, diagnostics into the score |
| 25 | score-editing | 1.5 | Semantic edit commands → text edits; keyboard entry |
| 26 | source-workspace | 1.5 | CodeMirror 6 + musa language, two-way linking; graduates `docs/rules/desktop/` |
| 27 | notation-details | 2 | Ties, slurs, dynamics, articulations, tuplets |
| 28 | performance-profiles | 2 | Interpretation profiles; MIDI export |
| 29 | studio-language | 2 | Studio DSL → StudioSpec |
| 30 | dsp-modulation | 2 | ADSR, LFO, filters, typed parameters |
| 31 | dsp-effects-mix | 2 | Delay, chorus, reverb, buses, sends |
| 32 | musicxml-export | 2 | `musa render --to musicxml` |
| 33 | midi-entry-autosave | 2 | Live MIDI input, step entry, autosave |
| 34 | transforms-variation | 3 | stretch/retrograde/invert as elaboration-time functions, specialization |
| 35 | annotations-harmony | 3 | Phrase/form and harmony as typed interval payloads |
| 36 | imports-and-curves | 3 | Relative imports, tempo/expression curves |
| 37 | event-track-observation | 3 | Composable observation; ambient extension deleted; events spec repairs |
| 38 | semantic-benchmarks | 3 | Measured baseline for the semantic pipeline |
| 39 | score-facts | 3 | Every notated fact is an occurrence; snapshot becomes a projection |
| 40 | context-facts | 3 | Key, meter, sections, harmony as occurrences; one timeline per piece |
| 41 | retire-the-oracle | 3 | The direct lowerer and the `Elaboration` switch deleted |
| 42 | snapshot-projection | 3 | `ScoreSnapshot` closed behind its interface |
| 43 | semantic-identity | 3 | Semantic hash; playback keyed on meaning, not revisions |
| 44 | events-queries | 3 | `covering` and `prevailing`: the event track answers questions, not just states facts |
| 45 | events-progress | 3 | `Progress` — continuous shape in the denotation; **Q4 resolved** |
| 46 | events-term-spec | 3 | `docs/rules/events/10-term-calculus.md` (candidate) |
| 47 | events-terms | 3 | `Term`, evaluator, soundness laws |
| 48 | events-interop | 3 | `musa events`; `.event track` round-trip; graduates the calculus |
| 49 | elaboration-emits-terms | 3 | Sharing: `repeat` and motifs become `let` |
| 50 | windowed-observation | 3 | Deferred observation, if measurement justifies it |
| 51 | engraved-edition | 2 | Front matter, instrument labels, measure numbers: the page as a real edition |
| 52 | linked-reading | 2 | One shared focus: which note is which, in both directions |
| 53 | pointer-editing | 2 | The score writes the source: token-scoped pointer edits |
| 54 | editable-facts | 2 | The piece's own facts, editable where they are printed |
| 55 | reading-preferences | 2 | Text size and vim mode: how the composer reads and types |
| 56 | diagnostics-that-teach | 2 | Codes, labelled spans, help, and applicable fixes, in both frontends |
| 57 | bars | 2 | `bar { … }`: the unit musicians think in, checked and nameable |
| 58 | repeats-and-endings | 2 | `repeat` becomes notation; first and second endings |
| 59 | menus-and-settings | 2 | Grouped menus, an Export submenu, and Settings behind `⌘,` |
| 60 | a-wider-source-column | 2 | The seam becomes a separator: drag the source column wider |
| 61 | bar-lines | 3 | Measure numbering as a function of the meters in force |
| 62 | mark-vocabulary | 3 | The notation vocabulary becomes a table, not a closed enum |
| 63 | one-context | 3 | Key, meter, clef and tempo unified as scoped facts with real spans |
| 64 | meter-changes | 3 | Mid-piece meter, written where the music changes |
| 65 | key-and-clef-changes | 3 | Modulation, and a clef that changes mid-measure |
| 66 | indeterminacy-spec | 3 | Where indeterminacy lives; **Q2 resolved** |
| 67 | realization | 3 | `Realization`, `ChoicePath`, `--seed`; reproducible performances |
| 68 | open-form | 3 | Mobile form, free duration, improvisation; _In C_ and Klavierstück XI |
| 69 | groove | 3 | Swing, shuffle, push: a `Beat → Beat` warp in the profile layer |
| 70 | notation-marks | 3 | Fermata, pedal, ottava, ornaments, text, sample, cue |
| 71 | grace-notes | 3 | Point occurrences with an ordering index; the profile decides the steal |
| 72 | tempo-facts | 3 | The tempo marking becomes a fact, separated from the `Beat → Second` map |
| 73 | tempo-ramps | 3 | _rit._ and _accel._ via `Progress`, exact in seconds-per-beat |
| 74 | unmeasured | 3 | `meter none`: cadenzas, chant, proportional spacing |
| 75 | polymeter-and-polytempo | 3 | Per-scope barline grids and tempo maps; the gate passed on Nancarrow |
| 76 | realization-in-the-page | 2 | The freedom printed, the decision shown, the seed in Settings |
| 77 | language-server | 3 | `musa-lsp` over stdio: diagnostics, formatting, hover, definition, symbols, fixes, tokens, completion |
| 78 | references-and-rename | 3 | The resolver records use-sites; references and rename rewrite recorded spans only |
| 79 | folding | 3 | Folding ranges from the CST, valid source or not |
| 80 | tree-sitter-grammar | 3 | tree-sitter-musa, corpus pinned token-for-token to the real lexer |
| 81 | vscode-extension | 3 | vscode-musa: generated TextMate grammar plus the language client |
| 82 | zed-extension | 3 | zed-musa: WASM extension, grammar queries, server wiring |
| 83 | lint-pass | 3 | Style-guide warnings as ordinary diagnostics; suppression lives in the source |
| 84 | keyword-documentation | 3 | Every keyword's plain-English doc, exhaustive by construction, over hover |
| 84a | the-project-is-the-unit | 3 | `Project` above `ProjectSession`: a running order, a piece each, material that opens |
| 85 | the-contents-page | 2 | The volume's front matter on the leaf, and the running order in the margin |
| 86 | the-events-file-reads | 3 | The interchange payload as named, quoted words; the corpus becomes `.musa.events` |
| 87 | the-note-is-one-word | 2 | `c4/4` and the augmentation dot; the `Duration` node the shorthand needs |
| 88 | sharps-and-flats | 2 | `f#3` and `eb4`, the spelling every DAW and chord chart uses |
| 89 | the-bar-is-the-line | 2 | `\|` for the anonymous bar, events without `;`, `[c3 g3]/2`, `>` and `^` |
| 90 | the-formatter-draws-the-bar | 2 | One bar per line, beat groups in the whitespace — a beam, written in text |
| 91 | bars-drawn-to-scale | 2 | One optional setting: horizontal position proportional to time |
| 92 | elaboration-language-spec | 3 | Normative candidate spec, formal judgments, decisions, proof obligations |
| 93 | elaboration-baseline | 3 | Frozen compatibility oracle and end-to-end performance baseline |
| 94 | expression-syntax | 3 | Lossless typed-expression grammar, formatting, editor grammar parity |
| 95 | total-functional-core | 3 | Private total typed evaluator/checker and metatheory law suite |
| 96 | finite-data-and-budgets | 3 | Finite values, structural folds, deterministic resource budgets |
| 97 | contextual-music | 3 | Context-neutral `Music`, one elaboration path, compatibility migration |
| 98 | higher-order-music | 3 | Music-producing functions, structural folds, controlled pitch mapping |
| 99 | bundled-standard-library | 3 | Source-defined, inspectable, versioned standard library |
| 100 | written-pitch-algebra | 3 | Spelled pitch/interval action and distinct pitch-class quotient |
| 101 | scales-degrees-and-context | 3 | Keys, scales, degree resolution, register frames, modal collections |
| 102 | chord-classes-and-voicings | 3 | Chord symbols/classes/voicings with explicit realization |
| 103 | structural-templates | 3 | Typed piece/voice templates with stable generative identity |
| 104 | library-modules-and-functors | 3 | Static signatures/modules/functors for coherent theory contexts |
| 105 | pc12-sets-and-rows | 3 | Explicit post-tonal pitch-class, set, and row domain |
| 106 | transformational-harmony-library | 3 | Source-defined, domain-correct neo-Riemannian operations |
| 107 | tonal-harmony-construction | 3 | Typed Roman and chromatic harmony constructors, not analyses |
| 108 | domain-metatheory | 3 | The musical domains proved a conservative extension, with a checked registry |
| 109 | the-import-keyword | 3 | `import` for imports, `use` for material; the old spelling errors with a fix |
| 110 | packages-and-module-trees | 3 | `musa.toml`, `lib.musa`, `mod`, nested paths, one derived source of truth |
| 111 | the-structure-keyword | 3 | ML's `structure` for the static layer, so `module` means a package node |
| 112 | braced-function-bodies | 3 | `fn f() -> τ { e }`: a block expression holding exactly one expression |
| 113 | capitalized-type-names | 3 | `Pitch`, `Music`, `Some`: the type layer gets a spelling of its own |
| 114 | angle-bracketed-type-parameters | 3 | `Option<τ>` and `List<τ>`, so `[` means a list and nothing else |
| 115 | schemas-and-harmonization | 3 | Finite source-defined schemas, sequences, Rule of the Octave |
| 116 | explicit-theory-assertions | 3 | Identity-preserving opt-in musical constraints |
| 117 | analysis-service | 3 | Narrow evidence-bearing advisory-analysis boundary |
| 118 | tonal-analysis | 3 | Ambiguity-preserving Roman, cadence, tonicization, modulation findings |
| 119 | voice-leading-and-counterpoint | 3 | Explicit style profiles with rule strengths and evidence |
| 120 | events-source-inclusion | 3 | Valid `.musa.events` documents and typed whole-document inclusion |
| 121 | typed-events-quotation | 3 | Hygienic local quote/antiquote at the context-neutral boundary |
| 122 | elaboration-language-tooling | 3 | One compiler-backed semantic tooling model across editors |
| 123 | observable-pipeline | 3 | `tracing` spans on the facades, `MUSA_LOG`, and a subscriber in every shell |
| 124 | elaboration-workbench | 3 | Musician-first desktop interaction for types, origin, assertions, analysis |
| 125 | language-and-theory-handbook | 3 | Tested musician and implementor paths with theory citations |
| 126 | core-boundary-decision | 3 | What the core is a calculus of, decided and costed before the sound block |
| 127 | elaboration-performance-closure | 3 | Profiled latency, allocation, memory, caching, and budget closure |
| 127a | core-calculus-governance | 3 | Amend the rules for one inferred language of event tracks and machines |
| 127aa | kinded-inference | 3 | Infer source types, with ordinary and storable-data type variables |
| 127ab | text-and-sums | 3 | Add text, binary sums, and one structural `Result` |
| 127ac | nominal-data | 3 | Let a library declare finite strictly positive data with one generated fold |
| 127ad | complete-calls | 3 | Delete partial calls and default parameters; add the anonymous function; migrate the corpus |
| 127b | inferred-source-core | 3 | Close the core: typed evaluator configurations, versioned costs, privacy audit |
| 127c | event-track-cutover | 3 | Replace timelines with coordinate-typed finite event tracks |
| 127ca | builtin-ownership-registry | 3 | One name for compiler-owned operations, freeing `primitive` for registered units |
| 127d | machine-values | 3 | Add finite typed machine descriptions as source values |
| 127da | path-aware-syntax | 3 | Give syntax values derivable paths, a path-aware fold, and pure builders |
| 127db | derivation-graph | 3 | Record origins as a grafted finite graph that keeps reuse and combined ancestry |
| 127dc | adapter-expansion | 3 | Expand named delimited adapter regions at one fixed place in the compiler order |
| 127dca | text-patterns-match | 3 | Make a text literal pattern match the text it spells, instead of falling through |
| 127dcb | adapter-refusal | 3 | Let an adapter refuse a region and point at the node its complaint is about |
| 127dcc | adapter-anchors | 3 | Let an adapter carry a source anchor into the value it produces |
| 127dcd | adapter-edit | 3 | Give an adapter the edit operation and prove its locality, agreement, and preservation |
| 127dce | adapter-print | 3 | Give an adapter the print operation and name the three conformance levels |
| 127dcea | exact-time-arithmetic | 3 | Give the source language exact time and the arithmetic to compute with it |
| 127dceb | adapter-module-scope | 3 | Give an adapter module the phase environment it was promised |
| 127dcec | construction-charges | 3 | Charge a value where it is constructed, not where it is named |
| 127dcf | staff-package | 3 | Write the staff package as ordinary unprivileged Musa |
| 127dcfa | staff-expansion | 3 | Expand a staff region into the staff package, item by item |
| 127dcfaa | list-fold-direction | 3 | Say which end a list fold runs from |
| 127dcfab | expression-if | 3 | Give the surface the conditional the core already claims |
| 127dcfac | record-update | 3 | Let a record be rebuilt by naming only what changed |
| 127dcfad | result-question | 3 | Let a failure propagate without a staircase |
| 127dcfae | recursor-trial | 3 | Paper-trial the sealed-step recursor before any code implements it |
| 127dcfaf | syntax-step-recursor | 3 | Replace the syntax catamorphism with an inherited-context recursor |
| 127dcfag | staff-retrial | 3 | Rewrite staff expansion on the repaired API and measure what changed |
| 127dcfah | printed-literals | 3 | Give the language the two operations a printer needs |
| 127dcfb | staff-edit-print | 3 | Make the staff adapter generative |
| 127dcg | studio-trial | 3 | _superseded by 146_ — write the studio adapter as an unprivileged package |
| 127dd | adapter-trials | 3 | _superseded by 147_ — freeze the adapter rules and carry them through hostile review |
| 127e | source-language-clean-break | 3 | _superseded by 142_ — delete contextual `Music` and migrate notation |
| 128 | core-amendment | 3 | Amend the core to admit a dependent foundation |
| 129 | dependent-core-spec | 3 | Specify the dependent core |
| 130 | trait-and-surface-spec | 3 | Specify records, enums, traits, and their surface |
| 131 | quotation-spec | 3 | Specify typed quotation and syntax patterns |
| 132 | paper-trial | 3 | Paper-trial the dependent language before any code implements it |
| 133 | core-crate | 3 | Build the dependent core as a leaf crate |
| 133a | core-provenance | 3 | Let a core term say where it came from |
| 134 | bidirectional-elaboration | 3 | Elaborate bidirectionally, with metavariables |
| 135 | inductive-families | 3 | Add inductive families, dependent match, and termination checking |
| 136 | records-and-enums | 3 | Give the language records and namespaced enums |
| 136a | module-visibility | 3 | Let a package hide what it maintains |
| 136b | core-divergence-repair | 3 | Remove the ad hoc divergences from the dependent core |
| 137 | traits-and-dictionaries | 3 | Add traits, dictionaries, and coherence |
| 137a | operators-and-methods | 3 | Route operators and methods through traits |
| 138 | typed-syntax | 3 | Give syntax a category, and the phase API its types |
| 139 | quotation | 3 | Implement quotation, splicing, and automatic provenance |
| 140 | syntax-patterns | 3 | Match syntax by quoting the shape you mean |
| 141 | collections | 3 | Let a list be built |
| 141a | adapter-module-diagnostics | 3 | Carry an adapter module's own diagnostics to its author |
| 141b | base-types-and-builtins | 3 | Give the core its base types and builtins |
| 141c | structural-eliminators | 3 | Give the core its structural eliminators |
| 141d | finite-constructor-builtins | 3 | Let a δ-rule speak the finite constructors |
| 141e | compiler-registry | 3 | Say what the compiler owns, in the core's own terms |
| 141f | phase-traversals | 3 | Let a traversal name what it builds |
| 141fa | constructor-checking | 3 | Check a constructor against its family |
| 141g | raw-lowering | 3 | Read the surface as a raw term |
| 141ga | quotation-core | 3 | Give a quotation a core shape |
| 141h | track-core | 3 | Give the track a core shape |
| 141ha | machine-core | 3 | Give the machine a core shape |
| 141i | constrained-definitions | 3 | Give a free definition its dictionary |
| 141j | notation-vocabulary | 3 | Give notation its core words |
| 141k | notation-lowering | 3 | Read a notated block as a track term |
| 141l | qualified-path | 3 | Read the qualified path |
| 141m | rule-refusal | 3 | Let a δ-rule refuse the program |
| 141n | top-level-program | 3 | Collect the signatures before the bodies |
| 141o | document-elaboration | 3 | Elaborate a whole document |
| 141p | piece-structure | 3 | Give the fold a voice to belong to |
| 141q | canonical-readback | 3 | Read canonical data back out of a term |
| 141r | instances-in-the-program | 3 | Declare a document's instances with its definitions |
| 141s | numeral-representation | 3 | A numeral is one node, not a tower |
| 141t | nested-occurrences | 3 | A family may hold a list of itself |
| 141u | glued-evaluation | 3 | A definition stays folded until something needs it open |
| 142 | surface-cutover | 3 | Move the whole language over, once |
| 142a | argument-order-and-sections | 3 | Reorder the argument spine; let a section be written |
| 142b | finish-the-excision | 3 | Finish note 50's phase 2 and repair the prose it falsified |
| 142c | index-amendment | 3 | Amend for a stratified index, and specify it before any code |
| 142d | index-stratum | 3 | A separate index language with a separate decider |
| 142da | index-reflexivity | 3 | Refuse an unreadable index at the type, not at the comparison |
| 142db | one-conversion-relation | 3 | One relation modulo the index theory, and the strategy that keeps it cheap |
| 142e | algebra-and-laws | 3 | Torsor, group, and action, named at the domains that already have them |
| 142f | writable-index | 3 | An indexed type spelled in source, checked at the sort its head declares |
| 142g | errors-below-refusals | 3 | One error type per question: the kernel's below the elaborator's |
| 142h | kernel-and-elaboration | 3 | `musa_calculus::{kernel, elaboration}`, with the direction enforced by a law |
| 142i | core-re-checker | 3 | An independent kernel re-checker for elaborated terms, with negative controls |
| 143 | one-theory-amendment | 3 | Commit the language to one theory; amend the constitution to say so |
| 144 | core-calculus-specification | 3 | Rewrite §§1–3 as one dependent presentation, before any code |
| 145 | repair-the-dependent-rules | 3 | Repair quotation and surface; retire the trait specification outright |
| 146 | delete-the-trait-system | 3 | Delete traits and dictionaries; type-directed disambiguation in their place |
| 146a | finish-the-trait-deletion | 3 | The surface specification and the twenty diagnostic codes 146 left behind |
| 147 | term-and-binder-collapse | 3 | Seventeen shapes to fourteen: one binder node, one literal node |
| 147a | names-through-the-context | 3 | One `Named` node; a name's reduction behaviour becomes a `Definition` |
| 148 | kernel-and-elaboration | 3 | `musa_calculus::{kernel, elaboration}`, with the direction enforced by a law |
| 149 | the-trusted-kernel | 3 | A TCB boundary, a `Checked` newtype, and the re-checker that guards 151–157 |
| 150 | errors-below-refusals | 3 | One error type per question: the kernel's below the elaborator's |
| 151 | delete-the-index-stratum | 3 | Delete `Indexed` and the solver; restore read-back equality |
| 152 | universe-levels | 3 | A non-cumulative polymorphic hierarchy; the two-universe ceiling goes |
| 153 | metavariables-and-unification | 3 | Real metavariables, a constraint queue, pattern unification |
| 154 | implicit-arguments | 3 | Implicit binders written, inserted, and named |
| 155 | case-trees | 3 | Case trees replace generated recursors; the motive becomes dependent |
| 155aa | lift-local-recursion | 3 | A term-position `rec` becomes an auxiliary top-level definition |
| 155a | case-tree-bodies | 3 | A definition body becomes a case tree, and termination is checked on it |
| 156 | indexed-families | 3 | A constructor may choose its index; `Equal` becomes library code |
| 157 | records-leave-the-core | 3 | `record` as one-constructor data with generated projections |
| 158 | recheck-the-whole-core | 3 | Audit every extension, close the re-checker, make it the gate |
| 159 | delete-the-coercion-rule | 3 | The language's one coercion rule deleted, and forgetting written at the site |
| 160 | macros-as-functions | 3 | The macro layer's eight claims gathered into one falsifiable law, and the chapter that teaches them |
| 161 | one-declaration-form | 3 | `data` becomes the one form, and `enum` and `record` two shapes of it |
| 162 | delete-the-module-layer | 3 | `signature`, `structure`, `template structure`, `make`, and the `template` that shares `make` — all five go |
| 162a | module-privacy-for-source | 3 | A source file gets a `ModuleId`, so `private` is refused across files instead of carried |
| 162b | parameterized-record-literals | 3 | `Cell<A>` can be declared and not constructed; the literal learns to find its family |
| 162ba | one-declaration-order | 3 | An index cannot name the document it is written in; the declarations get one dependency order |
| 162c | nested-patterns-parse | 3 | §1 says patterns nest and the parser reads one level; it learns the rest |
| 162d | or-patterns | 3 | `Bass \| Tenor -> true`: one arm for several constructors, and the amendment that admits it |
| 162e | let-in-a-block | 3 | Naming an intermediate value costs a top-level `fn`; the core has had `let` all along |
| 162f | lazy-methods | 3 | An `if` evaluates both branches, so the staff adapter has a classifier the domain never asked for |
| 162g | value-literals-as-syntax | 3 | `dot_count` is four quotes for four numbers, and `doubled` hand-allocates thirteen roles |
| 162h | literal-parts | 3 | `c#5` and `3/8` are one token each, so a consumer re-parses what the lexer already found |
| 162ha | a-literal-is-one-lexeme | 3 | The phase has four node shapes and none is one lexeme, so a quoted `c#5` prints as `c # 5` |
| 162hb | a-region-arrives-in-parts | 3 | a region's `c#5` is still one token, so an adapter cannot tell `4/4` from `1/1` |
| 162i | a-field-is-a-member | 3 | `g.compose(a, b)` is refused because `.` changes meaning when a `(` follows it |
| 163 | laws-as-record-fields | 3 | Group, action and torsor carry the equations they always claimed — after 164 builds them |
| 164 | builtin-collapse | 3 | Collapse the builtin registry behind methods and namespaces |
| 164a | one-document-form | 3 | `library` is `piece` minus five statements, so the three document shapes become one |
| 165 | diagnostics-and-performance | 3 | Make the new failures legible and the new checker fast enough |
| 165a | explicit-control-stack | 3 | Recursion depth leaves the nesting metric for the step budget it belongs to — before 166, with 165b |
| 165b | graph-update-and-data-descent | 3 | Memoize the unfold and take data descent off the nesting metric — before 166, which cannot run without it |
| 165c | release-what-a-compilation-held | 3 | The prelude context is an `Arc` cycle rebuilt per compilation: 257,208 bytes leaked every `compile` — after 166b |
| 165d | the-bar-claims-prefix | 3 | A bar's claim carries the whole prefix of its fold, so a hundred-bar voice elaborates one a hundred times — before 165 |
| 166 | staff-rewrite | 3 | Rewrite the staff adapter on the new language |
| 166b | per-context-memo-stamp | 3 | Scope the unfolding memo's invalidation stamp to its context, so one compilation's step count does not depend on another's |
| 166a | equality-for-declared-types | 3 | Whether a declared type gets an equality, and by which route |
| 167 | studio-rewrite | 3 | Write the studio adapter as an unprivileged package |
| 168 | adapter-freeze | 3 | Freeze the adapter rules and carry them through hostile review |
| 169 | core-conformance | 3 | Discharge the core's obligation matrix |
| 170 | language-pass-closure | 3 | Close the language pass |
| 171 | machine-runtime | 3 | Give each prepared machine one deterministic next step |
| 172 | track-scheduling | 3 | Connect exact event tracks to frame machines with checked decisions |
| 173 | one-frame-audio | 3 | Make one audio frame the reference meaning for every DSP unit |
| 174 | core-calculus-conformance | 3 | Prove and audit the clean cutover before sound-language work resumes |
| 174a | score-without-syntax | 3 | Delete the musa-score to musa-syntax edge; the values crate becomes a leaf |
| 174b | studio-spec-ownership | 3 | Generic checked-value bridge, proved on the finite source studio trial |
| 175 | exact-studio-values | 3 | Exact source quantities through the one audio-preparation conversion |
| 176 | studio-vocabulary | 3 | Source-declared processor/parameter vocabulary joined to private primitive contracts |
| 176a | payload-admission-rule | 3 | What an event-track payload owes, and the rendering law, before the first second payload |
| 176b | performance-source-foundation | 3 | Importable source performance vocabulary and indexed-control checking before the track bridge |
| 176c | unified-dependent-application | 3 | One dependent application syntax and complete indexed-constructor results before the performance bridge |
| 177 | performance-gestures | 3 | Source-declared gestures, indexed controls, profiles, and the provenance bridge |
| 178 | instrument-contracts | 3 | Source instrument signatures/mappings over private registered primitives |
| 179 | part-instrument-routing | 3 | Per-part instrument instances and routing isolation |
| 180 | expressive-control-realization | 3 | Marks and automation reach exposed controls, then private parameters |
| 180a | source-studio-cutover | 3 | Production studio semantics converge on checked source; the Rust language and backward edge leave |
| 180b | dependency-law | 3 | Crate layering enforced after cutover, separately from source-language ownership |
| 181 | ergonomic-sound-bindings | 3 | Musician-facing sound/profile choice and stable defaults |
| 182 | reproducible-assets | 4 | Content-addressed project/package audio assets and invalidation |
| 183 | pinned-package-imports | 4 | Exact remote source/asset packages, lockfile, offline builds, no solver |
| 184 | sampler-runtime | 4 | Deterministic native sample-map instrument implementation |
| 185 | sfz-instruments | 4 | Checked SFZ v1-core adapter and compatibility matrix |
| 186 | soundfont-instruments | 4 | Checked SoundFont 2.04 adapter and compatibility matrix |
| 187 | media-cue-semantics | 4 | Musical clips versus fixed-physical-duration cues |
| 188 | audio-clips | 4 | Prepared clip/cue playback, routing, seek, offline/live laws |
| 189 | sound-mix-workbench | 4 | Progressive musician/developer Sound and Mix interaction |
| 190 | audio-language-tooling | 4 | LSP/editor/handbook coverage for sound, assets, packages, formats |
| 191 | audio-performance-closure | 4 | Measured preparation/render/asset/UI performance and RT closure |
| 192 | audio-conformance | 4 | Complete performance/sound/assets conformance audit |
| 193 | language-conformance | 4 | Whole-language audit and conditional language-spec graduation |
| 194 | wasm-shell | 5 | `musa-wasm`: the whole pipeline as one small WebAssembly module |
| 195 | shared-engrave-package | 5 | `packages/musa-engrave`: the worker engraver shared by desktop and web |
| 196 | web-package-scaffold | 5 | `@musa/web` ESM package: low-level `parse`/`render` |
| 197 | dom-typesetting | 5 | `MusaWeb.typeset`, `<musa-score>`, error boxes, MutationObserver |
| 198 | provenance-interaction | 5 | Event-id callbacks and highlight via the MEI `xml:id` contract |
| 199 | web-distribution-and-examples | 5 | CDN iife build, example pages, build-time typesetting recipe |
| 200 | snippet-playback | 5 | **Deferred**: in-page PCM playback with playhead provenance |

Prompts 08–12 are the event-track insertion. The direct CST→score lowering built by prompts 05–06 was **frozen as the
regression oracle** when prompt 11 landed: the new event-track elaboration had to reproduce its snapshots exactly
(differential parity), and prompt 12 made the event-track path canonical. The oracle was retained through prompt 40 and
**deleted at prompt 41**, once the migration it guarded was finished; what replaced it is the `examples/` corpus with
goldens at every backend, the law suites, the event track's property tests, and the events normal forms. Nothing built
before prompt 08 is discarded — lossless parsing, formatting, exact rational time, provenance, `ScoreSnapshot`, and
`NotationPlan` were explicitly preserved while the event track was built beside them.

Prompts 20–26 are the interface block. They replace a single "Tauri + Svelte + Verovio" prompt that treated the desktop
app as plumbing and left its design, engraving quality, interaction model, and performance entirely unspecified — which
would have produced exactly the four-panel toolbar application this project exists to improve on. The block is ordered
so that **the design is settled before any plumbing exists** (prompt 20 is a fixture-driven prototype with real Verovio,
real fonts, and committed screenshot goldens, and nothing else), then wired (21), then made to hold up under real use
(22, 23), then given its distinguishing interaction (24) and its editing story (25, 26). `docs/rules/desktop/` is the
specification all seven implement.

Prompts 51–53 are the second interface block, and they answer one complaint in three parts: the app is a text editor
with a picture beside it. **51** makes the picture a document — front matter, instrument labels, measure numbers, a
running head, a final barline — because a page that looks like a printout of a data structure is not a page anyone
proofreads. **52** makes the two views legible against each other: one shared focus, marked in both at once, carrying
the plural answer musa actually has (one statement is several notes; one generated note has two statements). **53**
makes the page a way to _write_, under a rule narrow enough to survive roadmap §14.5's objection — a pointer gesture
replaces one token with one value, and nothing else. They run in that order: being able to read the correspondence is
what stops dragging it from being a guess.

**54–55 finish the same complaint from the other end.** 51–53 are about the notes; 54 is about everything else the
screen prints — a title, a composer, a tempo, a key — none of which could be changed anywhere but in the text, and none
of which said so. Its answer is one sentence: nothing selected is not nothing, and the list of what a piece can say is
the list of fields, with the empty ones on it. **55** is the pair of decisions the app was making on the composer's
behalf rather than badly: how large its text is, and whether its editor is modal. Neither touches the score — the
score's size is zoom, which is a re-layout and already has its own control, and keeping those two apart is the whole
reason 55 is a prompt and not a slider.

**56–58 are about the language as something a person uses.** The three earlier blocks each made musa able to _do_ more;
this one makes it answerable when a composer gets something wrong, and gives them the unit they were already thinking
in. **56** is the shape of a diagnostic: today every one of them is a string and a byte range, which is why they all
read like a parser talking to itself, and why the app prints `185` where a location belongs. **57** adds `bar { … }` — a
delimiter you can copy, a name you can reuse, and, because a bar declares what it claims to be, the first construct musa
can catch a composer disagreeing with. **58** stops `repeat` from printing its own expansion; it is the layer table's
own example, and the page has been wrong about it since prompt 06.

They run in that order because each is the last one's payoff. The bar-duration error is unreadable without secondary
labels, and the ending rules are unreadable without both.

**59 is the frame rather than the music.** Every prompt above it adds something the application can do; this one is
entirely about where what it can already do is _found_. Four sibling `Export …` verbs are one verb with four objects, a
View menu of twelve flat items answers none of the questions its reader is asking, and three preferences were filed
under View because there was nowhere else to put them. It adds one command — `settings.open` — and moves the rest.

**60 makes the most obvious control in the window do something.** The hairline between the source and the page looks
exactly like a splitter and was inert, and `01-visual-language.md` had a reason for that which, read closely, forbade a
pane that resizes _itself_ rather than one a composer resizes. So the reason survives the repair and the seam becomes a
separator — dragged, double-clicked back to the measure, and reachable by key, because a drag never carries a capability
on its own.

**61–76 are the temporal block, and they start from an accusation the earlier prompts had earned.** Musa could not write
a clef change, a modulation, a meter change, a fermata, a grace note, a _rit._, a swung eighth, a cadenza, or any music
that leaves a decision to its performer — which is most of what is played anywhere. The natural reading is that the
temporal model is too small. It is not: the event track is generic in its payload, and a clef change is a `FactKind`
with a span costing zero event track lines. What blocked all of it was above the event track — four separate mechanisms
for "what is in force here" (a `KeyMap` scalar, a `MeterMap` scalar, `Part::clef`, a `TempoMap` singleton), a measure
number computed by dividing by one of them, and a closed five-variant enum standing in for the whole vocabulary of
notation. **Every prompt in this block adds nothing to the event track** — no operation, no term form, no constructor.
That is the block's own falsification test, and §34's, applied sixteen times.

**61–63 are three refactors that ship no feature**, and they run first because each is provable by the strongest check
available: the rendered output — MEI, LilyPond, MusicXML, MIDI, audio — stays byte-identical, and any golden that does
move moves for a reason the prompt names in advance. 61 makes measure numbering a function of the meters in force rather
than a scalar threaded through nine functions in `plan.rs`. 62 makes the mark vocabulary a table, after counting the
cost of the enum it replaces — 65 references across 13 files, which is why musa has no fermata. 63 unifies the four
context mechanisms into scoped facts with real spans, with per-kind inheritance, because a single rule gets key wrong in
a way an ordinary viola part exposes. **64–76 then run breadth-first rather than by subsystem**, so that mid-piece meter
and modulation (64–65), open form (66–68), groove (69), the marks people actually write (70–71), gradual tempo (72–73),
unmeasured music (74) and polymeter (75) each reach a different kind of musician before any one of them is finished.

Read 66 as the block's second falsification. Indeterminacy looks like it needs a `choose` form in the term calculus, on
exactly the argument that justified `Progress` at prompt 45 — and it does not: `choose` cannot express _In C_'s
unbounded repeats or Klavierstück XI's 19! orderings, it makes T2 ambiguous, and it leaves a `.event track` file with no
normal form and no hash, which destroys the corpus that motivated it. So a realization becomes a **compile parameter**,
the freedom becomes a **payload value**, and Q2 closes on its own stated trigger in favour of its own working stance. 75
carries the same discipline forward as a gate rather than a conclusion: polymeter has repertoire and consumers,
polytempo had neither when the prompt was written, and the prompt says to ship half of itself if the gate does not open.
It opened — Nancarrow's _Canon X_ is the piece, and `MusicXML` and MEI attach a tempo to a part and a staff _in the
format_, which is what made polytempo something musa can hand to another program — and the prompt records the reasoning
where the decline would have gone.

Prompts 37–50 are the event track consolidation block. Prompt 12 made the event track canonical but deliberately kept
what the migration needed: the direct lowerer as a regression oracle, and a `ScoreSnapshot` shaped exactly as the
pre-event track compiler had left it. The consequence, five prompts later, was a system with one semantic core and two
temporal representations — notes in the timeline, and slurs, dynamics, key, meter, sections, and harmony in parallel
side tables keyed by ids the adapter assigned. **37–43 finish the migration and delete what it was keeping**: every
temporal fact becomes an occurrence, the snapshot becomes a projection behind an interface, the oracle goes, and
semantic identity replaces the revision counters that stood in for it. **44–45 make the event track useful rather than
merely correct**: a representation that can state everything and answer nothing is ceremony, so the event track gains
the two queries its consumers were each writing privately and differently (`covering`, `prevailing`), and `Progress`
puts a crescendo's _shape_ in the denotation instead of inside `performance.rs` — closing Q4, and closing it with a
payload value that needs no new operation and breaks no law. **46–50 add the term calculus** —
`docs/rules/events/01-grammar.md`'s long-promised syntax, with `let` for sharing, an evaluator, soundness theorems, and
the interchange format that Q6 said would justify a parser. The block follows the 08–12 shape: specify (46), implement
and prove (47), install (48–49), and measure before optimizing (38, 50).

Read 44 and 45 together as the answer to a fair objection: the event track was supposed to be a simpler interop target
than the surface language, but no backend consumed it and no consumer asked it anything. 39–43 made it _total_ — the
snapshot is now a projection of one timeline. 44 gives it an interface, 45 gives it the one thing it genuinely could not
say, and 48 makes the artifact real. Each of the three is a payoff the earlier prompts were only setting up. That 44 and
45 need **zero** new constructors is the standing evidence for `../../rules/events/00-purpose.md`'s governing design
rule: the operation set was right; the _surface_ was not.

**84–85 make the project the unit of work.** Roadmap §16 has described directory projects since the beginning and prompt
36 built the half of it a compiler needs — relative imports, and a `musa.toml` read for two keys — but nothing above the
compiler ever learned that a project has more than one piece in it. The consequence is that `examples/album/`, the
fixture §16 exists to justify, cannot be worked with: opening `pieces/01-opening.musa` says nothing about the piece
beside it, and opening `library/motifs.musa` is rejected by `elaborate.rs` under a help line that calls a library
legitimate. **84** adds `Project` above `ProjectSession` — not a facade over it, since it forwards nothing, but the
three facts no session can hold: which pieces there are, which one is in hand, and the running order the manifest sets.
A piece you turn away from keeps its text and its undo history and gives back only the audio device. **85** prints it,
and the argument there is that a file tree is the wrong object — §16 fixes a project's shape, so disclosure triangles
would model a freedom the format lacks while burying the running order. A bound volume already has the two devices
needed, a contents page and an editorial note at the foot for the shared material, and both say things a file browser
cannot. Neither prompt changes anything about a loose `.musa` file: it is a project of one, and with one entry no
contents appears anywhere.

**86–91 are about legibility, and they are one argument in two places.** A notation language has two texts a person
reads — the source and the interchange file — and both had been optimised for the machine that parses them. Measured
across `examples/`, 35% of non-blank lines are a single note averaging 7.5 characters, of which 54% is the duration;
`bulgarian.musa` spends 96 lines on fifteen bars and carries a comment apologising in prose for the grouping its layout
hides. The diagnosis is that musa took `**kern`'s one-event-per-line shape, which Humdrum's own documentation says is
"designed to facilitate analytic applications rather than music printing" — an analysis database's layout for a
composition language. **86** takes the interchange file, and goes first because 88 and 89 each regenerate all 24 goldens
and a diff is only reviewable against a format that can be read; it also settles the extension on the rule
`docs/rules/events/01-grammar.md` already states about unexplained shorthand. **87** makes a duration cost one
character, which is the whole constraint: musa keeps scientific pitch, so the octave digit is spoken for and a separator
is the only way out — every other system freed that digit by making octave non-numeric. **88** spells accidentals as
musicians do. **89** is the substance: a `|` where notation uses a separator, no `;` where the next pitch already says
the note ended, and the two marks with exact ASCII analogues. **90** then spends what 89 earned, one bar per line with
the beat groups in the whitespace — a beam, written in text — and **91** offers the one thing musa genuinely cannot
decide for a project, whether that line is drawn to scale.

Read 87 and 90 together as the pair: 87 argues that notation gets away with stating every duration because a glyph is
_free_, so text must make it cheap; 90 argues that notation groups beats with a beam, so text must group them with
space. Neither prompt invents a device. Both take one notation already has and ask what it costs in characters.

**92–99 build the elaboration-language foundation without weakening the event track boundary.** 92 specifies the static
and dynamic judgments before syntax is accepted, including the score→gesture→instrument→signal→mix factorization and
asset/package closure; 93 freezes both observable compatibility and cost before the implementation can move either;
94–96 add syntax, a total typed functional core, finite data, and deterministic limits; 97–98 make `Music` a
context-neutral elaboration result and allow higher-order construction only through structure-preserving operations; 99
makes the standard library ordinary inspectable Musa source. This order makes the event track the score denotation, not
the programming language or audio engine, and keeps evaluator/type-checker types private to `musa-compiler`.

**100–119 are a bounded theory block, not a universal “music theory engine.”** 100–105 establish distinct domains for
spelled pitches, intervals, scales, keys, degrees, chord classes, voicings, `pc12`, pitch-class sets, and rows before an
operation can accidentally collapse them. 106–107 and 115 then implement transformational, tonal-construction, schema,
and harmonization libraries as total source functions over those types. 116 separates explicit assertions from advisory
inference; 117 gives analyses a narrow evidence-bearing service; 118–119 add tonal, voice-leading, and counterpoint
profiles with their repertoire and convention stated. The prompts cite the local Open Music Theory corpus where it is
authoritative and require local definitions, proofs, and exhaustive finite models where it is not.

**108–114 interrupt that block, and they are the correction it earned.** Their rules now live in `docs/rules/language/`
and their reasoning in
[`../../notes/research/60-language-decision-record.md`](../../notes/research/60-language-decision-record.md); they
answer a complaint that the language had started to read as improvised, and each of the seven names a fault that was
found by _writing_ or _running_ musa rather than by auditing it. **108** is the one that matters: `02-core-calculus.md`
§5 proves safety and normalization for a fragment of six base types and says in its own words that later additions are
not covered by an appeal to standard STLC — and prompts 100–107 then added twelve musical base types and sixty-nine
compiler-owned operations without one compatibility case between them. The fix is not twelve inductions nobody reads; it
is one parametric theorem whose premises the primitive registry carries as checked facts, so the next domain costs an
entry rather than a proof and cannot be added without one. **109** separates `use` the import from `use` the splice,
which the grammar had been telling apart by whether the operand happened to be a string. **110** makes the standard
library a package with a real module tree, because `stdlib/manifest.toml` was compiled under `#[cfg(test)]` while four
hand-maintained parallel lists did the actual resolving — a state that let `stdlib/sequences.musa` be committed, be
unreachable from every import, and build green.

**111** and **112** were found by running the first three. 110 needed a word for a package's children and took `mod`,
while `module` was already spent on the static signature/module/functor layer — 109's fault committed in the act of
fixing it — so **111** renames the rarer construct to ML's own word, `structure`, and `module` is left meaning one
thing. **112** gives a function body braces. That complaint had been examined during the first three and **rejected**,
on the grounds that there is no block form because there is no statement language; the argument was wrong, because a
block is a delimiter and not a statement sequence. `{ e }` elaborates to `e`, holds exactly one expression, and admits
no `let`, no `return`, and no sequencing — so `fn` stops being the one declaration in the language whose body is not
braced, and §5's normalization proof gains a derived form rather than a case.

**113** and **114** were found the same way, by reading the corrected language rather than by writing it, and they
finish the job the other five started: each is a spelling that was chosen locally with nothing comparing it to the rest
of the language. **113** gives every type a capital. Six words — `pitch`, `music`, `scale`, `key`, `degree`, `frame` —
were a type name _and_ a music statement keyword, which the parser survived by keeping a whitelist of the keywords a
type is allowed to be; a capital settles it in the lexer instead, where `key c major;` and `Key` are simply different
words. It also settles `pitchclass` versus `spelled_pc`, which the compiler and the specification had been spelling
differently, in favour of `NoteName` — because Open Music Theory reserves "pitch class" for the reading that forgets
spelling, which is `Pc12`. **114** moves a type parameter into angle brackets, so `[` means a list and nothing else.
That is cheap here and nowhere else: `Option` and `List` are the only parameterized types, both are keyword-headed, and
no user-written type application exists — so the `a < b > (c)` reading that forces a turbofish elsewhere has no term
that could produce it. Neither prompt adds a type, removes one, or changes what any of them mean.

They run in that order because 108 gates everything the theory block does next and depends on nothing; because running
109 before 110 rewrites each import site once rather than twice; and because 111 through 114 rewrite every declaration
and every type annotation in `stdlib/`, which grows with every prompt after them. 113 precedes 114 because it decides
what the words are and 114 only decides what surrounds them.

**120–124 pay the interchange and usability costs.** A `.musa.events` file becomes a valid typed Musa document before
local quote/antiquote is admitted; quotation crosses only the context-neutral `ScoreFact` boundary and preserves hygiene
and provenance. The language server, editor extensions, and desktop consume the same compiler/project facts. They may
explain types, origins, assertions, and competing analyses, but may not grow a second checker, editable expanded AST, or
visual programming model. 123 sits between the tooling and the workbench because it is the same debt in the other
direction: the pipeline had a voice — nine `tracing::warn!` calls — and no shell installed a subscriber to hear it. It
gives each public facade one span, fixes `MUSA_LOG` as the filter, keeps every log line off stdout, and keeps every log
line out of the audio callback.

**125–127 close the first score-elaboration attempt without graduating it.** 125 tests the musician and implementor
documentation paths. 126 records the earlier boundary decision: finite temporal values belong in the event track while
running signals do not. 127 measures that implementation. Later research found the missing distinction. An audio history
is open-ended, but the typed machine that produces its next frame is finite data. Treating the two as the same thing had
pushed the machine description out of the language for the wrong reason.

**127a–127d and 171–174 make the clean correction.** 127a amends the governing documents before code changes. 127aa–127b
then install one small, strict, total, HM-inferred language with complete calls and a checked storable-data boundary, in
five steps that each leave the workspace green: inference and the two kinds of type variable (127aa), `text`, sums and
`Result` (127ab), library-declared finite data (127ac), complete calls and the corpus migration (127ad), and the typed
evaluator configurations, versioned cost table, and privacy audit that close it (127b). 127c renames the finite temporal
value to `EventTrack<C,A>` and tags its coordinate. 127ca then gives compiler-owned operations their one name, so that
_builtin_ and _primitive_ mean what the language specification says they mean before 127d needs the second word. 127d
adds finite `Machine<K,A,B>` values. 127da–127dd then repair the failed adapter boundary one blocker at a time: syntax
values whose paths are derived rather than invented (127da), a grafted derivation graph that keeps every reuse site and
every combined parent (127db), the expansion phase itself at one fixed place in the compiler order (127dc), a text
literal pattern that matches the text it spells so that an adapter can tell one token from another at all (127dca), the
error half of an adapter's answer so that it can refuse a region and point at what is wrong with it (127dcb), an anchor
it can put into the value it produces so that a later package function's complaint still names the composer's own text
(127dcc), the two remaining declared operations — `edit` with its locality, agreement, and preservation law (127dcd) and
`print` with its round trip and the three conformance levels (127dce) — the exact-time algebra the trials have to
compute with, which the governing calculus already fixed and no prompt had yet implemented (127dcea), the module scope
127dc promised an adapter and the implementation never gave it, with the number and the text equality the first trial
proved missing (127dceb), the construction charge the same trial measured — a value charged once per mention rather than
once per construction, which made a project's cost the product of its data size and its program size (127dcec), the
staff trial in three parts — the package the expansion produces (127dcf), the expansion itself with its fourteen items
(127dcfa), and the edit and printer that make it generative (127dcfb) — the list eliminator the expansion found missing,
a fold that runs from the end, so that reading a region into right-nested data stops being a closure chain and lists
stop being the one inductive type whose eliminator is not its own (127dcfaa), the four everyday facilities the staff
trial found the language missing — the conditional the core inventory already claimed and the surface never had
(127dcfab), immutable update of a named record field, which seven near-identical `holding_*` functions in one file were
standing in for (127dcfac), and `Result`-specific `?`, which flattens a six-frame failure staircase without buying a
monad (127dcfad) — the traversal repair those four leave behind: a paper trial of an inherited-context recursor over
sealed steps (127dcfae), its implementation and the rules amendment that precedes it (127dcfaf), and the staff rewrite
that measures which repair removed what (127dcfag), the last thing the language was missing, found by trying to write
the printer and discovering that nothing in the language builds a text at all (127dcfah). The studio trial and the
freeze that were to follow — 127dcg and 127dd — are superseded by 167 and 168, which do the same work on the language
the pass below installs. The ergonomics land before the traversal deliberately: with them in hand, an improvement
measured after the recursor cannot be an improvement `foldr`, `if`, record update, or `?` had already made. 127dcfaf
supersedes 127da's rule that a fold is the only way into a syntax value — an adapter may now look at a node before
choosing whether, in what order, and under what context to read its children — while leaving derived paths, unreadable
`SourceInfo`, and the builder facade exactly where 127da put them. The second blocker — a lowered match with no
executable meaning — closes by _not_ adding a decision-tree target: source `match` remains the one evaluator. Deleting
contextual `Music` was 127e's job and is now 142's, folded into the one migration rather than done twice. 171 gives each
machine one exact next step. 172 makes the time-to-frame policy explicit. 173 makes one sample frame the reference
meaning of audio and treats host blocks only as checked batching. 174 proves and audits the complete path before any
later sound prompt may run. Old syntax, APIs, and serialized forms are removed, not kept behind aliases.

**128–142 rebuilt the surface language on a dependent core, and 171–174 wait for what follows.** The staff adapter is
the evidence: 2,404 lines of Musa for a notation reader, most of it compensating for a language that cannot build a
list, name a field, or say what a piece of syntax is. 128 amends the constitution and obligations to admit that
evidence, keeping totality and widening it to a checked well-founded measure — Musa is almost entirely a compile-time
language, so divergence is a compiler hang, and a dependent checker is an evaluator. 129 specifies the core: universes,
Π, dependent records, inductive families, an identity type, conversion by normalization-by-evaluation, and bidirectional
elaboration with metavariables. 130 says what an author types — records with nested update, namespaced enums, coherent
dictionary-elaborated traits with no search, operators and methods under exact-receiver lookup. 131 adds `Syntax<Cat>`,
`quote at here { … }` with splicing, provenance the elaborator computes rather than the author allocating by hand, and
the inverse pattern form. 132 then trials all of it on paper against ten complete programs before a line is implemented.
No falsifier fires; eight corrections do, of which the largest are that K is dropped as an axiom because no program
unifies an index, that seven of the fourteen phase operations go rather than only the role integers, and that the
dispatch table has a third half no type system removes. It predicts the staff rewrite at 2,050 ± 100 lines, argues that
the line count measures the file rather than the language, and proposes a table of counts in its place. It also finds
the one gap the pass did not cover — a package cannot hide a constructor, so an invariant maintained by a smart
constructor is decoration — which is 136a. Ranks 133–142 carried the implementation — the core crate, elaboration,
families, records and enums, visibility, traits, typed syntax, quotation and patterns, collections, and one surface
cutover rather than two.

**Then the course correction, and its own correction, changed what 164–170 are for.** Note 50 audited every mechanism of
that core against the committed Musa program requiring it, found no user of the identity type, universe polymorphism,
indexed families, postponed constraints, general measures, or constraint-based traits, and deleted them; the compiler
shed 32,482 lines and the old `core.rs` checker with them. Notes 51 and 52 then audited _that_, because the rule tested
only _smallest_ and the number the constitution named as the falsifier had moved the wrong way — the staff adapter is
2,515 lines against the 2,404 the amendment was granted on. Three decisions are named as missteps: deleting indices,
which the corpus pays for as seventeen builtins hardcoded to one modulus and a `fallback` parameter in a public
signature; deleting the elaboration order that makes un-annotated lambdas work; and refusing partial application, which
forbids naming T₃, the first object of post-tonal theory. **142a–142e are that correction** — the two-pass spine and
written sections, the finished excision, and a stratified Dependent ML index whose equality is decided by arithmetic and
never by unification, with torsors, group actions, and laws checked by enumeration over the finite carriers an index
makes knowable. 164–170 survive with their tasks intact and their targets enlarged: the builtin collapse now has the
seventeen to collapse, and the staff rewrite is the gate for the correction and its correction alike.

**174b–181 build musical sound on that core without a second Rust language.** Repaired 174b first establishes a generic
checked-value bridge and proves complete readback of prompt 167's deliberately small source studio trial. 175 first
declares exact source quantities and keeps them exact through the one DSP conversion. 176 then makes processor contracts
and the rest of the source studio vocabulary discoverable and joins primitive-backed declarations to private host
registrations. 176a's payload rule guards the second payload. 176b makes `std::performance`, its source data, and its
ordinary indexed-control checking available before 177 declares gestures, indexed controls, and profiles in
`std::performance` and gives opaque track/provenance work to the host. 178 supplies the executable registered-primitive
wrappers and declares instrument signatures, mappings, and private machine bodies in source. 179 preserves part identity
through prepared routing, and 180 evaluates source control mappings before resolving private parameters. Once that
source side has production parity, 180a deletes the legacy public `StudioSpec` path and the compiler-to-DSP edge; 180b
then enforces the resulting Cargo roles separately from source ownership. 181 gives the surface one clear sound/profile
choice while keeping expert source machine and mix declarations available. Removed patch syntax is a hard error with a
certain fix, not a compatibility path. Notes 79–82 record why the Rust-vocabulary work was reopened, why bridging must
precede cutover, and why source quantities precede vocabulary while executable wrappers remain in 178.

**182–188 add external sound without making builds or time implicit.** 182 defines verified content-addressed assets
before a decoder exists. 183 adds exact-pinned fetch/lock/offline packages while keeping package edges separate from
module imports; exact source bytes establish equality and hashes only locate candidates. It promises no stable compiled
identity or persistent compiled-value cache. 184 builds one deterministic sampler runtime; 185 and 186 translate SFZ and
SoundFont into it through explicit support matrices rather than adopting either format as Musa's ontology. 187
distinguishes a beat-fitted clip from a point cue whose asset keeps its physical duration; 188 implements both as
machines under the same prepared offline/live step semantics.

**189–193 make the sound language usable and make graduation expensive.** 189 repairs Sound/Mix around instruments,
exposed controls, part outputs, assets, and media without creating GUI-owned state. 190 extends generated editor facts
and the two-path handbook. 191 measures preparation, rendering, decoded memory, callback deadlines, and UI updates. 192
audits every performance/sound/asset/package law and format support claim. Only 193 combines that green matrix with the
score/theory/events/tooling matrix and conditionally graduates `docs/rules/language/`.

**Prompts 194–200 are the web block: musa as a MathJax-like library for any page.** The stack the desktop app already
proved — Rust compiles source to MEI, a worker engraver turns MEI into SVG, `xml:id`s carry provenance — is packaged,
not reinvented. **194** crosses the existing pipeline to WebAssembly as a shell crate with the post-wasm-pack toolchain
(`wasm-bindgen --target web` + pinned `wasm-opt`; wasm-pack was sunset in 2025). **195** extracts the desktop's worker
engraver into `packages/musa-engrave` so two platforms share one provenance-critical module instead of drifting apart.
**196** scaffolds `@musa/web` with the low-level `parse`/`render` pair (the mermaid shape). **197** adds the MathJax
layer: `typeset()`, the `<musa-score>` element, visible error boxes, an opt-in observer — with the source kept in the
DOM, because text is canonical on the web too. **198** wires the `event-<hex>` contract to page callbacks, the feature
that makes it musa and not another notation renderer. **199** ships the CDN single-tag build (Blob-inlined worker),
example pages, and the build-time recipe for static sites. **200** is deferred: in-page playback, scheduled only when a
real need is demonstrated.

Phase numbers follow roadmap §18. "Phase 1.5" is the project layer and GUI, which the roadmap places inside Phase 1
("Verovio score preview", "play, stop, seek, loop") but which this sequence deliberately runs after the CLI-provable
slice. Prompt 33 is roadmap Phase 2 scope ("MIDI step entry", "autosave") ordered after the desktop prompts it depends
on; phases describe scope, not strict order.

## Out of scope for this sequence

The user has now asked for the Phase 4 sample/media/library work, and it is deliberately ordered after prompt 175's
native score/elaboration stability point. The following remain outside this sequence:

- CLAP/VST hosting and the macOS Audio Unit bridge;
- a package registry, semantic-version range solver, implicit network during compilation, or packages containing native
  executable code;
- MusicXML import;
- microphone/audio recording, destructive waveform editing, beat detection, transient slicing, and DAW-style timeline
  editing;
- pitch-preserving clip time-warping, convolution, mastering suites, and unbounded disk streaming until a separately
  measured musical workload justifies each one.

Prompts 100–119 are the requested, deliberately finite theory-library scope. Still out of scope are a universal or
style-neutral theory engine, unconstrained automatic composition, corpus-trained inference, probabilistic analysis,
arbitrary tuning-system abstraction, and claims that one analytical vocabulary is musical truth. A new repertoire, style
profile, or theory family needs its own named domain, sources or definitions, tests, and prompt; it must not enter
through a widening “theory” trait or an undocumented default.
