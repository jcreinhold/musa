# 75. The language pass, closed

**Status: governs nothing.** This is prompt 170's closing measurement. It puts notes 50, 60, 61, 64, and 66 in one place
and records where the course-correction forecast was right and where executable evidence overturned it.

## What changed in size

The comparable implementation surface is the calculus plus the compiler's old/new language machinery: lowering,
registrations, expansion/quotation/phase support, and the old checker files where they existed. Counts use committed
Rust source at note 50's audit commit `8db2135` and at closure.

|  | Rust files | Lines | Bytes |
| --- | ---: | ---: | ---: |
| before | 59 | 61,749 | 2,674,812 |
| after | 113 | 52,705 | 2,315,233 |
| difference | +54 | −9,044 (−14.6%) | −359,579 (−13.4%) |

The file count rose because the surviving implementation was split into `kernel/` and `elaboration/` deep modules and
their law modules. The line prediction in note 50 was only partly right. The duplicate compiler checker/evaluator did
die (`core.rs` and `elaborate.rs`, 16,979 lines at the audit), and the combined surface shrank by nine thousand lines;
but `musa-calculus` itself is 25,887 lines in 52 files, not the predicted 11–12K. Indexed families, an independent
rechecker, contextual metavariables, pattern unification, and postponement all returned because later committed programs
proved that the proposed first-order replacement could not express the language the pass had admitted.

That is not the pass missing its own gate. Note 50's governing audit rule was that every mechanism names a committed
program that requires it. The final implementation obeys that stronger rule and rejects its inaccurate size forecast.

## The type-checking algorithm that exists

There is one dependent core and one certification boundary.

1. The compiler lowers the lossless CST to `RawProgram`, preserving origins and module identity. It registers finite
   host base types, constructors, and δ-rules; it does not implement a second calculus.
2. Top-level declaration groups are collected before bodies, giving recursive references rigid declared types.
3. Elaboration is bidirectional. Variables, projections, literals, and eliminations synthesize. Lambdas, record and
   constructor literals, quote forms, case methods, and other introductions check against an expected type. Application
   synthesizes a Π, checks the argument against its domain, and substitutes it into the codomain. `Switch` is the only
   conversion site.
4. Conversion evaluates both sides by NbE and compares their quoted normal forms, including β, registered δ,
   constructor/recursor ι, Π η, and record η. It inserts no coercion and performs no search.
5. An omitted inferred argument or locally unknown type creates a metavariable carrying the exact local scope it may
   mention. A flex-rigid equation is solved only when the metavariable's spine consists of distinct local variables—the
   Miller pattern fragment. Spine inversion maps stable de Bruijn levels into declaration-scope variables, preserving
   permutations and weakening. A duplicate, non-variable, escaping, omitted, or cyclic occurrence is not solved.
6. A comparison blocked on unknowns is queued. Solving a metavariable retries affected comparisons; finishing a
   declaration drains the finite queue. A survivor is a deterministic error naming what could not be determined. There
   is no higher-order search, backtracking, or default.
7. Indexed constructor checking unifies constructor result indices with the expected family. Case-tree compilation
   checks coverage and refines branch indices. Recursive calls are accepted only when their designated argument is a
   pattern-bound structural subterm.
8. The deterministic meter charges steps, nesting, constructed nodes, and logical bytes before work or construction.
   Exhaustion refuses the declaration graph without publishing a partial value.
9. Finally, prompt 149's independent kernel rechecker checks the completed term from scratch. Only that rechecked term
   crosses the facade. `crates/musa-calculus/TRUST.md` names the trusted half.

The literature discipline is the usual one, not an improvised inference heuristic: bidirectional checking and NbE follow
the sources collected in `docs/rules/language/citations.md`; metavariable solving follows Miller's unitary pattern
fragment and Idris 2's distinct-local-spine, scope, occurs, postpone, and retry structure. Musa deliberately stops at
that fragment.

## The namespace and method algorithm that replaced traits

Note 50 predicted a flat trait/instance table. Prompt 146 deleted it after measuring zero trait-constrained signatures.
The actual algorithm is smaller and more deterministic:

1. `impl Head { fn m … }` lowers to an ordinary definition named `Head.m`. It creates no instance, dictionary, or
   resolution table.
2. `Head::m(x, y)` writes the complete name and uses ordinary lexical/visibility lookup.
3. For `x.m(y)`, the elaborator first determines the rigid head of `x`'s concrete type. It then reads two tables keyed
   by that same head and member: the visible definition `Head.m`, and a record field `m` on a one-constructor family.
   Exactly one is accepted; zero reports the type/member and available fields; two report the collision and the two
   explicit spellings. Candidates are never trial-elaborated.
4. Operators lower to the corresponding receiver method before elaboration, so they use the same rule.
5. A bare member spelling is filtered by an already-known expected type head against the finite set of namespaces that
   declare it. One survivor resolves; zero or several report the candidates. The filter compares names and never tries
   then rolls back a term.
6. A flexible/generic receiver has no namespace. The author supplies an explicit qualified path or a signature that
   makes the receiver concrete. There is no auto-deref, coercion, free-function fallback, implicit dictionary, or
   package-order preference.

Thus adding a package definition cannot silently capture an old call: it either remains outside the exact receiver's
namespace or produces a loud ambiguity.

## Deleted and surviving mechanisms

| Verdict | Mechanism | Evidence |
| --- | --- | --- |
| deleted | `Id`/`refl`/`J`/`K`, tactics, proof search, well-founded measures, partial definitions | no committed program; definitional equality plus written finite equality suffices |
| deleted | source traits, instances, dictionaries, super-constraints | zero constrained signatures; ordinary namespace definitions serve all committed calls |
| deleted | signature/structure/template/make declaration layer | records, functions, modules, `private`, and ordinary event-track values express every committed example |
| deleted | duplicate compiler checker/evaluator and compiler-private `Datum` path | prompt-149 trusted boundary and prompt-169 second-path audit |
| deleted | 22 modulus-specific post-tonal builtins | `std::cyclic`, `pcset`, `serial`, and `algebra` over indexed `Pc(n)`/`ToneRow(n)`; note 61 |
| retained | NbE and glued evaluation | dependent conversion; staff-page and corpus performance laws |
| retained | contextual metas, postponement, Miller-pattern unification | inferred indices, argument-order laws, index refinement, permutation/weakening/flex-flex laws |
| retained | indexed families and dependent recursors | generic-row-at-12, `Pc(n)`, `ToneRow(n)`, constructor-refinement and case-tree laws |
| retained | independent kernel rechecker | every accepted corpus program; prompt-149 trust law |
| retained | structural recursion and case trees | `std::list`, staff adapter, cyclic/post-tonal library functions |
| retained | typed syntax, sealed traversal, quotation, phase registrations | staff and graph adapters, edit and printer round trips |
| retained | exact multi-axis budget | large-score, generic-row, deep-region refusal, and exact boundary laws |
| retained | host base/builtin registry | provenance, direct event-track/core construction, private musical representation, and finite-work charges; note 61 surveys every row |

## The measured programs

The staff rewrite's fixed prompt-166 measurement is 2,149 lines, 80,374 bytes, and 1,597 non-comment/non-blank lines,
down 366 lines and 18,181 bytes from the file actually migrated. Later repairs brought the current file to 2,289 lines
and 80,281 bytes; the fixed measurement remains the valid before/after comparison. All seven mechanical gates reached
zero except the aspirational phase-operation count: six operations survived against a target of five, because
`syntax_at` and `as_expression` are the separate projection and checked-reading boundaries. Collapsing them would hide a
parse inside a projection.

`examples/staff-page.musa` costs 180,873 reduction steps. After cached duration decomposition, the large desktop fixture
compiles at 110.9 ms P1 median; composing that with the measured UI path yields 360 ms, below B2's 400 ms budget. The
final language table is 2,000,000 steps, 1,000,000 constructed nodes, and 16 MiB logical bytes. Large score spends
355,992 / 239,394 / 1,319,043; the generic twelve-tone row binds steps and nodes at 1,081,475 / 574,098 / 596,344.

The second adapter is materially different and needed no new compiler-facing capability:
`stdlib/src/adapters/graph.musa` is 1,152 lines / 42,286 bytes / 1,083 substantive lines and its value package is 506 /
17,851 / 434, for 1,658 lines / 60,137 bytes / 1,517 substantive lines total. Its committed witness is
`examples/live-studio.musa` plus the graph adapter laws.

## What remains

The language pass is closed, but the prompt-127a runtime cutover is not. `StudioGraphSpec`, `compile_graph`,
`RenderPlan`, caller-block feedback, and block-rate modulation remain live production mechanisms. They are not language
pass failures and are not being hidden: the clean-break ledger reassigns them to prompt 173, after prompt 171 defines
one exact machine step and prompt 172 defines checked scheduling. Prompt 174 owns the cross-stage conformance audit and
complete provenance composition after that boundary.
