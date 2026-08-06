# Musa Implementation Prompts

This directory is the executable work plan for building musa according to
[`../initial-design-roadmap.md`](../initial-design-roadmap.md) as course-corrected by
[`../course-correction.md`](../course-correction.md). Each numbered prompt delivers one feature and builds on
the prompts it depends on. Work them in dependency order; when in doubt, work them in numeric order.

**Where the two documents disagree** — most importantly, the course correction's rule that the surface grammar does
not define the ontology and that a small temporal kernel (`timeline` / `sequence` / `overlay` over exact rational
ambient time) is the semantic core — the course correction wins. Prompts 08–12 specify, implement, prove, and install
that kernel; prompts 13+ proceed exactly as before on top of it.

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
| 20 | desktop-shell | 1.5 | Tauri + Svelte + Verovio preview |
| 21 | score-editing | 1.5 | Semantic edit commands → text edits |
| 22 | notation-details | 2 | Ties, slurs, dynamics, articulations, tuplets |
| 23 | performance-profiles | 2 | Interpretation profiles; MIDI export |
| 24 | studio-language | 2 | Studio DSL → StudioSpec |
| 25 | dsp-modulation | 2 | ADSR, LFO, filters, typed parameters |
| 26 | dsp-effects-mix | 2 | Delay, chorus, reverb, buses, sends |
| 27 | musicxml-export | 2 | `musa render --to musicxml` |
| 28 | midi-entry-autosave | 2 | Live MIDI input, step entry, autosave |
| 29 | transforms-variation | 3 | stretch/retrograde/invert as elaboration-time functions, specialization |
| 30 | annotations-harmony | 3 | Phrase/form and harmony as typed interval payloads |
| 31 | imports-and-curves | 3 | Relative imports, tempo/expression curves |

Prompts 08–12 are the course-correction insertion. The direct CST→score lowering built by prompts 05–06 is **frozen as
the regression oracle** once prompt 11 lands: the new kernel elaboration must reproduce its snapshots exactly
(differential parity), and prompt 12 makes the kernel path canonical. Nothing built before prompt 08 is discarded —
lossless parsing, formatting, exact rational time, provenance, `ScoreSnapshot`, and `NotationPlan` are explicitly
preserved by the course correction (§29, §35.2).

Phase numbers follow roadmap §18. "Phase 1.5" is the project layer and GUI, which the roadmap places inside Phase 1
("Verovio score preview", "play, stop, seek, loop") but which this sequence deliberately runs after the CLI-provable
slice. Prompt 28 is roadmap Phase 2 scope ("MIDI step entry", "autosave") ordered after the desktop prompts it depends
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
open-ended algorithms over the compositional model, not core features; prompt 30 builds the annotation and chord-symbol
model they would consume. Scope a theory library as its own prompt sequence when a concrete operation is requested.
