# Musa Implementation Prompts

This directory is the executable work plan for building musa according to
[`../initial-design-roadmap.md`](../initial-design-roadmap.md) as course-corrected by
[`../course-correction.md`](../course-correction.md), with the desktop interface governed by
[`../interface/`](../interface/README.md). Each numbered prompt delivers one feature and builds on the prompts it
depends on. Work them in dependency order; when in doubt, work them in numeric order.

`docs/governance/` now owns identity-level commitments. `docs/spec/` owns cross-stage formal semantics, including the
private process calculus and exact preparation/cache laws; `docs/architecture/` maps those rules to current code. A
pending prompt which contradicts them is repaired and committed before implementation, per execution rule 5.

**Where the roadmap and the course correction disagree** — most importantly, the course correction's rule that the
surface grammar does not define the ontology and that a small temporal kernel (`timeline` / `sequence` / `overlay` over
exact rational ambient time) is the semantic core — the course correction wins. Prompts 08–12 specify, implement, prove,
and install that kernel; prompts 13+ proceed exactly as before on top of it.

**Where the roadmap is silent on the desktop interface** — its visual language, engraving quality, interaction model,
states, and performance budgets — `docs/interface/` is the authority. Roadmap §14 still fixes the architecture. Prompts
20–26 implement `docs/interface/`; prompt 26 graduated it from candidate to governing, as prompt 12 did for the kernel.

**The elaboration-language direction is a candidate until it earns graduation.** Prompt 92 turns
`docs/elaboration-language.md` into a precise candidate specification under `docs/language/`; prompts 93–145 implement,
measure, and audit its score, performance, sound, asset, and package semantics; prompt 146 makes it governing only if
the complete conformance matrix is green. Until then, the roadmap, course correction, and existing kernel remain
authoritative where the candidate differs.

## Prompt anatomy

Each prompt is a markdown file `NN[suffix]-<slug>.md` with YAML frontmatter. The number is an execution rank, not an
identity. A lowercase suffix (`129a`) inserts a prompt between two existing ranks without renumbering anything after it,
and is the right insertion when the ranks that would move include finished work; `scripts/renumber-prompts.py make-room
--at N` is the right insertion otherwise. `id` carries the suffix, and so does every `depends_on` that names the prompt.

```yaml
---
id: NN
slug: short-name
status: pending        # pending | in-progress | done
depends_on: [NN, ...]  # earlier prompt ids
phase: 1               # roadmap phase this feature belongs to
---
```

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
   - `cargo fmt --check`;
   - the prompt's behavior checks on `examples/*.musa` fixtures.
5. If a prompt turns out to be mis-scoped (two independent features, or a missing prerequisite), repair the prompt files
   first, commit that repair, then implement.

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

| # | Prompt | Phase | Delivers |
| --- | --- | --- | --- |
| 01 | workspace-skeleton | 0 | Seven crates, facades, CLI stub |
| 02 | lexer | 1 | Trivia-preserving tokens |
| 03 | parser-cst | 1 | Lossless CST, recovery, typed wrappers |
| 04 | formatter | 1 | `musa format` |
| 05 | compiler-core | 1 | `musa check`; ScoreSnapshot + provenance |
| 06 | motifs-and-transforms | 1 | Motifs, repeat, transpose, expansion laws |
| 07 | notation-plan | 1 | Measures, beaming, tie decomposition |
| 08 | kernel-spec | 1 | `docs/kernel/` temporal-kernel specification (candidate) |
| 09 | temporal-kernel | 1 | `musa-kernel`: timeline/sequence/overlay/restrict/scale/normalize |
| 10 | kernel-laws | 1 | Algebraic law proofs incl. non-laws |
| 11 | kernel-elaboration | 1 | Surface → kernel elaboration + differential parity |
| 12 | kernel-switch | 1 | Kernel becomes the canonical semantics |
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
| 26 | source-workspace | 1.5 | CodeMirror 6 + musa language, two-way linking; graduates `docs/interface/` |
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
| 37 | kernel-observation | 3 | Composable observation; ambient extension deleted; kernel spec repairs |
| 38 | semantic-benchmarks | 3 | Measured baseline for the semantic pipeline |
| 39 | score-facts | 3 | Every notated fact is an occurrence; snapshot becomes a projection |
| 40 | context-facts | 3 | Key, meter, sections, harmony as occurrences; one timeline per piece |
| 41 | retire-the-oracle | 3 | The direct lowerer and the `Elaboration` switch deleted |
| 42 | snapshot-projection | 3 | `ScoreSnapshot` closed behind its interface |
| 43 | semantic-identity | 3 | Semantic hash; playback keyed on meaning, not revisions |
| 44 | kernel-queries | 3 | `covering` and `prevailing`: the kernel answers questions, not just states facts |
| 45 | kernel-progress | 3 | `Progress` — continuous shape in the denotation; **Q4 resolved** |
| 46 | kernel-term-spec | 3 | `docs/kernel/10-term-calculus.md` (candidate) |
| 47 | kernel-terms | 3 | `Term`, evaluator, soundness laws |
| 48 | kernel-interop | 3 | `musa kernel`; `.kernel` round-trip; graduates the calculus |
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
| 68 | open-form | 3 | Mobile form, free duration, improvisation; *In C* and Klavierstück XI |
| 69 | groove | 3 | Swing, shuffle, push: a `Beat → Beat` warp in the profile layer |
| 70 | notation-marks | 3 | Fermata, pedal, ottava, ornaments, text, sample, cue |
| 71 | grace-notes | 3 | Point occurrences with an ordering index; the profile decides the steal |
| 72 | tempo-facts | 3 | The tempo marking becomes a fact, separated from the `Beat → Second` map |
| 73 | tempo-ramps | 3 | *rit.* and *accel.* via `Progress`, exact in seconds-per-beat |
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
| 86 | the-kernel-file-reads | 3 | The interchange payload as named, quoted words; the corpus becomes `.musa.kernel` |
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
| 120 | kernel-source-inclusion | 3 | Valid `.musa.kernel` documents and typed whole-document inclusion |
| 121 | typed-kernel-quotation | 3 | Hygienic local quote/antiquote at the context-neutral boundary |
| 122 | elaboration-language-tooling | 3 | One compiler-backed semantic tooling model across editors |
| 123 | observable-pipeline | 3 | `tracing` spans on the facades, `MUSA_LOG`, and a subscriber in every shell |
| 124 | elaboration-workbench | 3 | Musician-first desktop interaction for types, origin, assertions, analysis |
| 125 | language-and-theory-handbook | 3 | Tested musician and implementor paths with theory citations |
| 126 | core-boundary-decision | 3 | What the core is a calculus of, decided and costed before the sound block |
| 127 | elaboration-performance-closure | 3 | Profiled latency, allocation, memory, caching, and budget closure |
| 128 | studio-vocabulary | 3 | One generated processor/parameter vocabulary, hover, terminology |
| 129 | exact-studio-values | 3 | Exact written quantities through audio preparation |
| 129a | payload-admission-rule | 3 | What a kernel payload owes, and the rendering law, before the first second payload |
| 130 | performance-gestures | 3 | Instrument-independent note gestures and musical control curves |
| 131 | instrument-contracts | 3 | Typed exposed controls over private native/sample implementations |
| 132 | part-instrument-routing | 3 | Per-part instrument instances and routing isolation |
| 133 | expressive-control-realization | 3 | Marks and automation reach exposed controls, then private parameters |
| 134 | ergonomic-sound-bindings | 3 | Musician-facing sound/profile choice and stable defaults |
| 135 | reproducible-assets | 4 | Content-addressed project/package audio assets and invalidation |
| 136 | pinned-package-imports | 4 | Exact remote source/asset packages, lockfile, offline builds, no solver |
| 137 | sampler-runtime | 4 | Deterministic native sample-map instrument implementation |
| 138 | sfz-instruments | 4 | Checked SFZ v1-core adapter and compatibility matrix |
| 139 | soundfont-instruments | 4 | Checked SoundFont 2.04 adapter and compatibility matrix |
| 140 | media-cue-semantics | 4 | Musical clips versus fixed-physical-duration cues |
| 141 | audio-clips | 4 | Prepared clip/cue playback, routing, seek, offline/live laws |
| 142 | sound-mix-workbench | 4 | Progressive musician/developer Sound and Mix interaction |
| 143 | audio-language-tooling | 4 | LSP/editor/handbook coverage for sound, assets, packages, formats |
| 144 | audio-performance-closure | 4 | Measured preparation/render/asset/UI performance and RT closure |
| 145 | audio-conformance | 4 | Complete performance/sound/assets conformance audit |
| 146 | language-conformance | 4 | Whole-language audit and conditional language-spec graduation |
| 147 | wasm-shell | 5 | `musa-wasm`: the whole pipeline as one small WebAssembly module |
| 148 | shared-engrave-package | 5 | `packages/musa-engrave`: the worker engraver shared by desktop and web |
| 149 | web-package-scaffold | 5 | `@musa/web` ESM package: low-level `parse`/`render` |
| 150 | dom-typesetting | 5 | `MusaWeb.typeset`, `<musa-score>`, error boxes, MutationObserver |
| 151 | provenance-interaction | 5 | Event-id callbacks and highlight via the MEI `xml:id` contract |
| 152 | web-distribution-and-examples | 5 | CDN iife build, example pages, build-time typesetting recipe |
| 153 | snippet-playback | 5 | **Deferred**: in-page PCM playback with playhead provenance |

Prompts 08–12 are the course-correction insertion. The direct CST→score lowering built by prompts 05–06 was **frozen as
the regression oracle** when prompt 11 landed: the new kernel elaboration had to reproduce its snapshots exactly
(differential parity), and prompt 12 made the kernel path canonical. The oracle was retained through prompt 40 and
**deleted at prompt 41**, once the migration it guarded was finished; what replaced it is the `examples/` corpus with
goldens at every backend, the law suites, the kernel's property tests, and the kernel normal forms. Nothing built before
prompt 08 is discarded — lossless parsing, formatting, exact rational time, provenance, `ScoreSnapshot`, and
`NotationPlan` are explicitly preserved by the course correction (§29, §35.2).

Prompts 20–26 are the interface block. They replace a single "Tauri + Svelte + Verovio" prompt that treated the desktop
app as plumbing and left its design, engraving quality, interaction model, and performance entirely unspecified — which
would have produced exactly the four-panel toolbar application this project exists to improve on. The block is ordered
so that **the design is settled before any plumbing exists** (prompt 20 is a fixture-driven prototype with real Verovio,
real fonts, and committed screenshot goldens, and nothing else), then wired (21), then made to hold up under real use
(22, 23), then given its distinguishing interaction (24) and its editing story (25, 26). `docs/interface/` is the
specification all seven implement.

Prompts 51–53 are the second interface block, and they answer one complaint in three parts: the app is a text editor
with a picture beside it. **51** makes the picture a document — front matter, instrument labels, measure numbers, a
running head, a final barline — because a page that looks like a printout of a data structure is not a page anyone
proofreads. **52** makes the two views legible against each other: one shared focus, marked in both at once, carrying
the plural answer musa actually has (one statement is several notes; one generated note has two statements). **53**
makes the page a way to *write*, under a rule narrow enough to survive roadmap §14.5's objection — a pointer gesture
replaces one token with one value, and nothing else. They run in that order: being able to read the correspondence is
what stops dragging it from being a guess.

**54–55 finish the same complaint from the other end.** 51–53 are about the notes; 54 is about everything else the
screen prints — a title, a composer, a tempo, a key — none of which could be changed anywhere but in the text, and none
of which said so. Its answer is one sentence: nothing selected is not nothing, and the list of what a piece can say is
the list of fields, with the empty ones on it. **55** is the pair of decisions the app was making on the composer's
behalf rather than badly: how large its text is, and whether its editor is modal. Neither touches the score — the
score's size is zoom, which is a re-layout and already has its own control, and keeping those two apart is the whole
reason 55 is a prompt and not a slider.

**56–58 are about the language as something a person uses.** The three earlier blocks each made musa able to *do* more;
this one makes it answerable when a composer gets something wrong, and gives them the unit they were already thinking
in. **56** is the shape of a diagnostic: today every one of them is a string and a byte range, which is why they all
read like a parser talking to itself, and why the app prints `184` where a location belongs. **57** adds `bar { … }` — a
delimiter you can copy, a name you can reuse, and, because a bar declares what it claims to be, the first construct musa
can catch a composer disagreeing with. **58** stops `repeat` from printing its own expansion; it is the layer table's
own example, and the page has been wrong about it since prompt 06.

They run in that order because each is the last one's payoff. The bar-length error is unreadable without secondary
labels, and the ending rules are unreadable without both.

**59 is the frame rather than the music.** Every prompt above it adds something the application can do; this one is
entirely about where what it can already do is *found*. Four sibling `Export …` verbs are one verb with four objects, a
View menu of twelve flat items answers none of the questions its reader is asking, and three preferences were filed
under View because there was nowhere else to put them. It adds one command — `settings.open` — and moves the rest.

**60 makes the most obvious control in the window do something.** The hairline between the source and the page looks
exactly like a splitter and was inert, and `01-visual-language.md` had a reason for that which, read closely, forbade a
pane that resizes *itself* rather than one a composer resizes. So the reason survives the repair and the seam becomes a
separator — dragged, double-clicked back to the measure, and reachable by key, because a drag never carries a capability
on its own.

**61–76 are the temporal block, and they start from an accusation the earlier prompts had earned.** Musa could not write
a clef change, a modulation, a meter change, a fermata, a grace note, a *rit.*, a swung eighth, a cadenza, or any music
that leaves a decision to its performer — which is most of what is played anywhere. The natural reading is that the
temporal model is too small. It is not: `Timeline<A>` is generic in its payload, and a clef change is a `FactKind` with
a span costing zero kernel lines. What blocked all of it was above the kernel — four separate mechanisms for "what is in
force here" (a `KeyMap` scalar, a `MeterMap` scalar, `Part::clef`, a `TempoMap` singleton), a measure number computed by
dividing by one of them, and a closed five-variant enum standing in for the whole vocabulary of notation. **Every prompt
in this block adds nothing to the kernel** — no operation, no term form, no constructor. That is the block's own
falsification test, and §34's, applied sixteen times.

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
exactly the argument that justified `Progress` at prompt 45 — and it does not: `choose` cannot express *In C*'s
unbounded repeats or Klavierstück XI's 19! orderings, it makes T2 ambiguous, and it leaves a `.kernel` file with no
normal form and no hash, which destroys the corpus that motivated it. So a realization becomes a **compile parameter**,
the freedom becomes a **payload value**, and Q2 closes on its own stated trigger in favour of its own working stance. 75
carries the same discipline forward as a gate rather than a conclusion: polymeter has repertoire and consumers,
polytempo had neither when the prompt was written, and the prompt says to ship half of itself if the gate does not open.
It opened — Nancarrow's *Canon X* is the piece, and `MusicXML` and MEI attach a tempo to a part and a staff *in the
format*, which is what made polytempo something musa can hand to another program — and the prompt records the reasoning
where the decline would have gone.

Prompts 37–50 are the kernel consolidation block. Prompt 12 made the kernel canonical but deliberately kept what the
migration needed: the direct lowerer as a regression oracle, and a `ScoreSnapshot` shaped exactly as the pre-kernel
compiler had left it. The consequence, five prompts later, was a system with one semantic core and two temporal
representations — notes in the timeline, and slurs, dynamics, key, meter, sections, and harmony in parallel side tables
keyed by ids the adapter assigned. **37–43 finish the migration and delete what it was keeping**: every temporal fact
becomes an occurrence, the snapshot becomes a projection behind an interface, the oracle goes, and semantic identity
replaces the revision counters that stood in for it. **44–45 make the kernel useful rather than merely correct**: a
representation that can state everything and answer nothing is ceremony, so the kernel gains the two queries its
consumers were each writing privately and differently (`covering`, `prevailing`), and `Progress` puts a crescendo's
*shape* in the denotation instead of inside `performance.rs` — closing Q4, and closing it with a payload value that
needs no new operation and breaks no law. **46–50 add the term calculus** — `docs/kernel/01-grammar.md`'s long-promised
syntax, with `let` for sharing, an evaluator, soundness theorems, and the interchange format that Q6 said would justify
a parser. The block follows the 08–12 shape: specify (46), implement and prove (47), install (48–49), and measure before
optimizing (38, 50).

Read 44 and 45 together as the answer to a fair objection: the kernel was supposed to be a simpler interop target than
the surface language, but no backend consumed it and no consumer asked it anything. 39–43 made it *total* — the snapshot
is now a projection of one timeline. 44 gives it an interface, 45 gives it the one thing it genuinely could not say, and
48 makes the artifact real. Each of the three is a payoff the earlier prompts were only setting up. That 44 and 45 need
**zero** new constructors is the standing evidence for course correction §34: the operation set was right; the *surface*
was not.

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
`docs/kernel/01-grammar.md` already states about unexplained shorthand. **87** makes a duration cost one character,
which is the whole constraint: musa keeps scientific pitch, so the octave digit is spoken for and a separator is the
only way out — every other system freed that digit by making octave non-numeric. **88** spells accidentals as musicians
do. **89** is the substance: a `|` where notation uses a separator, no `;` where the next pitch already says the note
ended, and the two marks with exact ASCII analogues. **90** then spends what 89 earned, one bar per line with the beat
groups in the whitespace — a beam, written in text — and **91** offers the one thing musa genuinely cannot decide for a
project, whether that line is drawn to scale.

Read 87 and 90 together as the pair: 87 argues that notation gets away with stating every duration because a glyph is
*free*, so text must make it cheap; 90 argues that notation groups beats with a beam, so text must group them with
space. Neither prompt invents a device. Both take one notation already has and ask what it costs in characters.

**92–99 build the elaboration-language foundation without weakening the kernel boundary.** 92 specifies the static and
dynamic judgments before syntax is accepted, including the score→gesture→instrument→signal→mix factorization and
asset/package closure; 93 freezes both observable compatibility and cost before the implementation can move either;
94–96 add syntax, a total typed functional core, finite data, and deterministic limits; 97–98 make `Music` a
context-neutral elaboration result and allow higher-order construction only through structure-preserving operations; 99
makes the standard library ordinary inspectable Musa source. This order makes the kernel the score denotation, not the
programming language or audio engine, and keeps evaluator/type-checker types private to `musa-compiler`.

**100–119 are a bounded theory block, not a universal “music theory engine.”** 100–105 establish distinct domains for
spelled pitches, intervals, scales, keys, degrees, chord classes, voicings, `pc12`, pitch-class sets, and rows before an
operation can accidentally collapse them. 106–107 and 115 then implement transformational, tonal-construction, schema,
and harmonization libraries as total source functions over those types. 116 separates explicit assertions from advisory
inference; 117 gives analyses a narrow evidence-bearing service; 118–119 add tonal, voice-leading, and counterpoint
profiles with their repertoire and convention stated. The prompts cite the local Open Music Theory corpus where it is
authoritative and require local definitions, proofs, and exhaustive finite models where it is not.

**108–114 interrupt that block, and they are the correction it earned.** Governed by
[`../language-correction.md`](../language-correction.md), they answer a complaint that the language had started to read
as improvised, and each of the seven names a fault that was found by *writing* or *running* musa rather than by auditing
it. **108** is the one that matters: `02-core-calculus.md` §5 proves safety and normalization for a fragment of six base
types and says in its own words that later additions are not covered by an appeal to standard STLC — and prompts 100–107
then added twelve musical base types and sixty-nine compiler-owned operations without one compatibility case between
them. The fix is not twelve inductions nobody reads; it is one parametric theorem whose premises the primitive registry
carries as checked facts, so the next domain costs an entry rather than a proof and cannot be added without one. **109**
separates `use` the import from `use` the splice, which the grammar had been telling apart by whether the operand
happened to be a string. **110** makes the standard library a package with a real module tree, because
`stdlib/manifest.toml` was compiled under `#[cfg(test)]` while four hand-maintained parallel lists did the actual
resolving — a state that let `stdlib/sequences.musa` be committed, be unreachable from every import, and build green.

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
were a type name *and* a music statement keyword, which the parser survived by keeping a whitelist of the keywords a
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

**120–124 pay the interchange and usability costs.** A `.musa.kernel` file becomes a valid typed Musa document before
local quote/antiquote is admitted; quotation crosses only the context-neutral `ScoreFact` boundary and preserves hygiene
and provenance. The language server, editor extensions, and desktop consume the same compiler/project facts. They may
explain types, origins, assertions, and competing analyses, but may not grow a second checker, editable expanded AST, or
visual programming model. 123 sits between the tooling and the workbench because it is the same debt in the other
direction: the pipeline had a voice — nine `tracing::warn!` calls — and no shell installed a subscriber to hear it. It
gives each public facade one span, fixes `MUSA_LOG` as the filter, keeps every log line off stdout, and keeps every log
line out of the audio callback.

**125–127 close the score-elaboration implementation without prematurely graduating the language.** 125 tests two
documentation paths — one by musical task and one by language implementation — and generates standard-library signatures
from source. 126 stops before the sound block and answers the question nineteen prompts were about to assume: what the
core is a calculus of. It takes a census of every surface construct against the kernel term it elaborates to, costs
three answers against that census, settles whether signals join an inductive calculus at all, and is allowed to repair,
delete, and create prompts — the sound block is contingent on it. Its answer, `docs/core-boundary.md`: the core is a
calculus of occurrences of any canonical payload, which is what `musa-kernel` was always generic over; signals stay
outside it because a signal is coinductive and a signal graph has no extent; and the prepared render plan is what
crosses. 127 compares score elaboration to 93's baseline and permits caching or incrementality only when semantic keys
and measured need are demonstrated. Audio retains its frozen baseline and receives its own measured closure at 142.

**128–134 replace the accidental score↔DSP wire with a typed instrument boundary.** 128 makes the studio vocabulary
discoverable from one catalogue; 129 restores exact written quantities; 129a states what a kernel payload owes before
any second payload exists, so the rule cannot be fitted to the payload it will admit; 130 names the missing object — an
exact instrument-independent gesture/control timeline, which after 126 is `Timeline<Gesture>` rather than a new
structure; 131 makes an instrument a deep contract over a private implementation; 132 preserves part identity through
prepared routing; and 133 binds musical controls to private parameters only at audio preparation. 134 then spends that
simplicity at the surface: choosing a sound/profile is one musical action, while expert graph and mix declarations
remain available and source-compatible.

**135–141 add external sound without making builds or time semantics implicit.** 135 defines verified content-addressed
assets before a decoder exists. 136 adds exact-pinned fetch/lock/offline packages while retaining the roadmap's
rejection of a registry and version solver. 137 builds one deterministic sampler runtime; 138 and 139 translate SFZ and
SoundFont into it through explicit compatibility matrices rather than adopting either format as Musa's ontology. 140
distinguishes a beat-fitted clip from a point cue whose asset keeps its physical duration; 141 renders both through the
same prepared offline/live plan.

**142–146 make the sound language usable and make graduation expensive.** 142 repairs Sound/Mix around instruments,
exposed controls, part outputs, assets, and media without creating GUI-owned state. 143 extends generated editor facts
and the two-path handbook. 144 measures preparation, rendering, decoded memory, callback deadlines, and UI updates. 145
audits every performance/sound/asset/package law and format support claim. Only 146 combines that green matrix with the
score/theory/kernel/tooling matrix and conditionally graduates `docs/language/`.

**Prompts 147–153 are the web block: musa as a MathJax-like library for any page.** The stack the desktop app already
proved — Rust compiles source to MEI, a worker engraver turns MEI into SVG, `xml:id`s carry provenance — is packaged,
not reinvented. **147** crosses the existing pipeline to WebAssembly as a shell crate with the post-wasm-pack toolchain
(`wasm-bindgen --target web` + pinned `wasm-opt`; wasm-pack was sunset in 2025). **148** extracts the desktop's worker
engraver into `packages/musa-engrave` so two platforms share one provenance-critical module instead of drifting apart.
**149** scaffolds `@musa/web` with the low-level `parse`/`render` pair (the mermaid shape). **150** adds the MathJax
layer: `typeset()`, the `<musa-score>` element, visible error boxes, an opt-in observer — with the source kept in the
DOM, because text is canonical on the web too. **151** wires the `event-<hex>` contract to page callbacks, the feature
that makes it musa and not another notation renderer. **152** ships the CDN single-tag build (Blob-inlined worker),
example pages, and the build-time recipe for static sites. **153** is deferred: in-page playback, scheduled only when a
real need is demonstrated.

Phase numbers follow roadmap §18. "Phase 1.5" is the project layer and GUI, which the roadmap places inside Phase 1
("Verovio score preview", "play, stop, seek, loop") but which this sequence deliberately runs after the CLI-provable
slice. Prompt 33 is roadmap Phase 2 scope ("MIDI step entry", "autosave") ordered after the desktop prompts it depends
on; phases describe scope, not strict order.

## Out of scope for this sequence

The user has now asked for the Phase 4 sample/media/library work, and it is deliberately ordered after prompt 129's
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
