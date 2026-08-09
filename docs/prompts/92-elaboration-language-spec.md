---
id: 92
slug: elaboration-language-spec
status: pending
depends_on: [49, 63, 86, 91]
phase: 3
---

# Specify the Elaboration Language Candidate

## Task

Turn `docs/elaboration-language.md` from a revised proposal into a precise candidate specification before any new syntax
or compiler path is implemented. Reconcile the roadmap, course correction, kernel elaboration documents, style guide,
and prompt stack around one staged design: a total value calculus; contextual, context-neutral `music`; closed kernel
terms; declaration templates; and a typed kernel escape. Settle the remaining surface spellings with a corpus that a
musician can read and a language implementor can type-check without hidden rules. The candidate is the
implementation contract for prompts 93–118 but does not outrank the existing governing documents until prompt 119's
audit graduates it.

## Read

- `docs/elaboration-language.md`, in full. Its rejection of timeline flattening, distinction between open `music` and
  a closed term, theory-domain separations, equality relations, and private compiler boundary are the decisions this
  prompt makes precise rather than re-litigates.
- `docs/course-correction.md` §§2–5, 13–14, 19–20, 24, 29, 34–35 and every file in `docs/kernel/`, especially
  `06-surface-elaboration.md` and `10-term-calculus.md`. The kernel still has no join, lambda, scale, chord, or musical
  payload knowledge.
- Roadmap §§2–10, 15, 17–19; `docs/interface/02-interaction-model.md` and `04-origin-view.md`.
- Open Music Theory (OMT) `005`, `013`–`021`, `023`–`028`, `033`–`036`, `049`–`051`, `061`–`076`, and
  `099`–`110`
  under `~/Code/papers/music-theory/open-music-theory/`. Cite the exact chapter file for every imported music-theory
  definition. Claims not supplied by OMT must be stated as Musa definitions and proved from those definitions.

## Design

Create `docs/language/` as a small normative candidate specification, not a second essay:

1. `00-semantics.md` — the four representations and two staging judgments; ownership boundaries; contextual
   `instantiate`; context-neutrality; closure to `Term<ScoreFact>`; equality and provenance.
2. `01-surface.md` — grammar additions and desugarings, including `fn`, `let`, types, calls, `music`, `in scale`,
   assertions, structural templates, module parameters, and kernel quotation. Every example must be both readable aloud
   and unambiguous to the lossless parser.
3. `02-core-calculus.md` — monomorphic STLC after elaboration, finite inductive data, structural eliminators,
   call-by-value evaluation, static judgments, resource rejection, and the proof obligations later prompts discharge.
4. `03-musical-domains.md` — pitch/interval, spelled pitch class versus `pc12`, key versus scale, degree/register,
   chord class/triad/voicing, row, and analysis-result definitions with OMT citations or local proofs.
5. `04-templates-and-modules.md` — declaration-template judgment, stable generative identity, module signatures and
   static functors; no first-class pieces, voices, modules, or source reflection.
6. `05-verification.md` — constructor invariants, explicit assertions, interpretive analyses, laws, counterexamples,
   and the compatibility/performance gates for prompts 93–119.

The surface corpus includes the root-dependent turn, major/dorian rebinding, a higher-order canon, a harmonizer using a
controlled pitch traversal, a key-parameterized piece, a parameterized voice, a chord class in two voicings, a generic
and symmetric twelve-tone row, a successful and failing assertion, a standalone `.musa.kernel` document, and a local
quote with antiquotation. For each, state the desugaring and the equality under which it is correct.

Compare the rejected public `musa-elaboration` crate with the chosen private `musa-compiler` subsystem. Record the
actual callers and why `Type`, `Value`, `Closure`, `Music`, module environments, and theory algorithms stay private.
No new public API is justified by a specification document.

## Target

- `docs/language/{00-semantics,01-surface,02-core-calculus,03-musical-domains,04-templates-and-modules,
  05-verification}.md`.
- Deliberate repairs to `docs/{initial-design-roadmap,course-correction,style-guide}.md` and
  `docs/kernel/06-surface-elaboration.md`; remove or mark every contradiction while keeping existing governing
  precedence until prompt 119.
- `docs/language/README.md`: candidate status, precedence, scope, document map, and prompt-119 graduation condition.
- `docs/elaboration-language.md`: marked as non-governing design input and linked to the split candidate specification.
- A source-map table in `03-musical-domains.md`: concept, Musa definition, OMT chapter or local theorem, falsifying
  example, and implementing prompt.

## Check

```sh
test -s docs/language/00-semantics.md
test -s docs/language/01-surface.md
test -s docs/language/02-core-calculus.md
test -s docs/language/03-musical-domains.md
test -s docs/language/04-templates-and-modules.md
test -s docs/language/05-verification.md
rg -n "Timeline\[Timeline|context-neutral|pc12|declaration template|antiquotation" \
  docs/language docs/kernel/06-surface-elaboration.md
git diff --check
cargo fmt --check
```

Commit as `Specify the elaboration language candidate`.

## Stop

- No Rust, Svelte, tree-sitter, or example-source changes.
- Do not add a kernel constructor or weaken the `.musa.kernel` calculus.
- Do not leave syntax alternatives in a normative candidate rule. Open punctuation is decided here from the corpus.
- Do not claim a music-theory law from terminology alone; cite OMT or give Musa's definition and proof.
- No macro system, general recursion, effects, first-class syntax, first-class piece/voice values, or public elaboration
  crate.
