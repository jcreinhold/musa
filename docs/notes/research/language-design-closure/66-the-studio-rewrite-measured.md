# 66. The studio rewrite, and what the second adapter found

**Status: governs nothing.**
[`../../../plan/prompts/167-studio-rewrite.md`](../../../plan/prompts/167-studio-rewrite.md) is the work plan; this page
records its measurement. The comparison is with [`60-the-staff-rewrite-measured.md`](60-the-staff-rewrite-measured.md),
whose staff numbers remain the fixed prompt-166 measurement rather than being re-counted after later formatting changes.

One adapter chosen after its language proves very little. The studio graph was written on paper before prompts 138–141
and differs from staff notation in its musical structure, so it is the useful second witness.

## 1. Size and the five eliminations

| File | Lines | Bytes | Non-comment, non-blank |
| --- | ---: | ---: | ---: |
| `stdlib/src/adapters/graph.musa` | 1,152 | 42,286 | 1,083 |
| `stdlib/src/sound/graph.musa` | 506 | 17,851 | 434 |
| **trial total** | **1,658** | **60,137** | **1,517** |

The adapter alone is 53.6% of prompt 166's measured 2,149-line staff adapter. That ratio is not a quality score: the
staff grammar has more forms, while the graph package has more global validation. Keeping package and adapter separate
is the meaningful boundary.

| Gate carried from the staff rewrite | Studio result |
| --- | ---: |
| `callN` construction helpers | **0** |
| hand-allocated syntax roles | **0** |
| string dispatch on token kinds or delimiters | **0** |
| wide positional state destructures | **0** — `Collected`, `Parsed`, and `Parameters` are records |
| forward traversal of child and declaration lists | **yes** — `fold_from_start` |

Construction is exclusively `quote at here { … }` plus splices. Token and delimiter decisions use `token_kind_equal` and
`delimiter_equal`; text matching begins only after the typed token has been classified as a graph word. The fixed lexer
retains `scale`, `output`, and `in` as keyword kinds even inside an imported region, so the adapter has three explicit
typed-kind-to-word cases. That is vocabulary-specific work, not a second lexer.

The language has no fixed-length list pattern. Only `[]` and `[head, .. tail]` are patterns, so the graph's connection
grammar is a visibly long sequence of head/tail matches. A quote pattern would describe Musa expression shape, not the
graph language, and helped no more here than it did for staff. Adding fixed-length patterns was not necessary for
correctness and this trial therefore did not add them, but this is the largest readability cost in the file.

The recursor remains useful only as the bottom-up boundary: it turns every group into a finite child list. Once there,
ordinary forward folds are clearer than selective descent. Headers choose which parser reads a group's already-collected
children; they do not decide whether the traversal visits those children. This is the answer to the paper trial's open
question.

## 2. The generality table

The compiler-facing rows agree:

| Question | Staff | Studio |
| --- | --- | --- |
| Input | `Syntax<TokenTree>` | `Syntax<TokenTree>` |
| Traversal | `recurse_syntax`, sealed child steps, finite folds | `recurse_syntax`, sealed child steps, finite folds |
| Output | one `Syntax<Expr>` | one `Syntax<Expr>` |
| Result value | `StaffDocument` | `StudioDescription` |
| Later validation | realization, bars, ties, notation policy | descriptors, ports, units, bindings, graph cycles |
| Source map | `Nat` anchors minted from the region | `Nat` anchors minted from the region |
| Edit result | one anchored pitch replacement | one anchored parameter-value replacement |
| Printer claim | canonical value-level round trip | canonical value-level round trip |
| Hidden compiler input | none | none |
| Inferred-type access | none | none |

The musical rows all differ:

| Musical question | Staff | Studio |
| --- | --- | --- |
| Primary structure | nested written sequence | directed finite graph |
| Local unit | note, rest, chord, or layout form | port, node, edge, parameter, or binding |
| Order means | written and metrical succession | declaration priority only; edges state topology |
| Nested body | notation recursively contained by a form | parameter table selected by a node descriptor |
| Equality needed | written musical constructors | exact port kind, including audio channel count |
| Global refusal | bar/tie/notation inconsistency | missing names, wrong directions or kinds, cycle |
| Later consumer | realization and engraving | future preparation and render-plan cutover |

The second adapter therefore needed no compiler-facing capability the first lacked. It did need different ordinary data,
different grammar functions, and different validation — precisely the package-owned differences the interface is meant
to permit.

## 3. Divergences are findings

The paper program could not be copied literally:

- The fixed token tree has no line groups, so declarations end in semicolons and node parameters use braces.
- `studio` is a statement keyword and cannot name a module segment. The value package is `std::sound::graph`; the phase
  package is `std::adapters::graph`. Adding another parser exception would have hidden the finding.
- The paper ports used `expression_depth.control` and `room.audio` as both input and output. Exact direction validation
  requires `control_in`/`control` and `audio_in`/`audio`, making the topology unambiguous.
- Ordinary source cannot store phase `Syntax`, so declarations carry the `Nat` from `syntax_anchor`.
- No bridge currently turns `StudioDescription` into the legacy Rust `StudioSpec`. The source description is finite and
  validates completely, but it does not drive DSP until prompt 174's cutover.
- The printer normalizes both `instrument` and `processor` nodes to `processor`. `StudioDecl::Node` deliberately stores
  no distinction, so the law is value-level re-expansion, not textual identity.

The executable evidence is `examples/live-studio.musa` and the thirteen laws in
`crates/musa-compiler/tests/suite/graph_adapter_laws.rs`: seven reading groups, four validation errors whose returned
anchors become observable event onsets, the exact `3/10` to `2/5` edit, and canonical printer re-expansion.
