# Musa Implementation Prompts

This directory is the executable work plan for building musa according to
[`../initial-design-roadmap.md`](../initial-design-roadmap.md) as course-corrected by
[`../course-correction.md`](../course-correction.md), with the desktop interface governed by
[`../interface/`](../interface/README.md). Each numbered prompt delivers one feature and builds on the prompts it
depends on. Work them in dependency order; when in doubt, work them in numeric order.

**Where the roadmap and the course correction disagree** — most importantly, the course correction's rule that the
surface grammar does not define the ontology and that a small temporal kernel (`timeline` / `sequence` / `overlay` over
exact rational ambient time) is the semantic core — the course correction wins. Prompts 08–12 specify, implement, prove,
and install that kernel; prompts 13+ proceed exactly as before on top of it.

**Where the roadmap is silent on the desktop interface** — its visual language, engraving quality, interaction model,
states, and performance budgets — `docs/interface/` is the authority. Roadmap §14 still fixes the architecture. Prompts
20–26 implement `docs/interface/`; prompt 26 graduated it from candidate to governing, as prompt 12 did for the kernel.

## Prompt anatomy

Each prompt is a markdown file `NN-<slug>.md` with YAML frontmatter:

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

Phase numbers follow roadmap §18. "Phase 1.5" is the project layer and GUI, which the roadmap places inside Phase 1
("Verovio score preview", "play, stop, seek, loop") but which this sequence deliberately runs after the CLI-provable
slice. Prompt 33 is roadmap Phase 2 scope ("MIDI step entry", "autosave") ordered after the desktop prompts it depends
on; phases describe scope, not strict order.

## Out of scope for this sequence (roadmap §18 Phase 4)

Do not create prompts for these until the native system is stable and the user asks:

- sample playback and SoundFont/orchestral support;
- audio-file clips;
- CLAP hosting and the macOS Audio Unit bridge;
- MusicXML import;
- audio recording and waveform editing (rejected outright unless the product's purpose changes).

Also deferred: the **theory libraries** of roadmap §8.2 (tonal analysis, Roman numerals, neo-Riemannian operations,
scales/modes, voice-leading, counterpoint, auto-voicing). The roadmap lists them under Phase 3 but defines them as
open-ended algorithms over the compositional model, not core features; prompt 35 builds the annotation and chord-symbol
model they would consume. Scope a theory library as its own prompt sequence when a concrete operation is requested.
