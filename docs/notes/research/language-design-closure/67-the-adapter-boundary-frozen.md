# 67. The adapter boundary, frozen

**Status: governs nothing.** This note freezes the implemented boundary for prompt 168. Governing meaning remains in
`docs/rules/`; this note gives the proof argument and the executable evidence map for that meaning.

## 1. The theorem and its assumptions

Let `S` be the ordinary dependent source calculus of `02-core-calculus.md`, checked in an ordinary environment `Σ`. Let
`P` be the phase calculus: the same term grammar, elaborator, case-tree compiler, re-checker, NbE evaluator, and
budgets, checked in `Σ, Σφ`. `Σφ` contains the closed `Cat` literals, `Syntax c`, `SyntaxStep C A`, and the phase
operations. Resolution constructs `Σφ` only while checking an adapter module. Expansion accepts one `Syntax TokenTree`,
evaluates one checked adapter function, gates the returned `Syntax Expr`, and then checks that ordinary expression in
`Σ`.

The proof is relative to the core obligations stated honestly in `05-metatheory.md`: typed core terms normalize and the
re-checker accepts exactly the admitted elaborator output. Prompt 169 owns the core obligation matrix. This note proves
that the phase boundary adds no new premise to those core obligations and that its one new eliminator preserves them.

## 2. The frozen rules

Each `E` label is an executable selection run by `scripts/check-syntax-adapter-conformance.sh`.

1. **F1 — phase separation.** Ordinary resolution never contains `Σφ`; no ordinary term can resolve `Cat`, `Syntax`,
   `SyntaxStep`, quotation, or a phase operation. The phase returns `Syntax Expr`; the gate parses and elaborates that
   tree in `Σ`, and only a storable ordinary value may then survive its enclosing declaration boundary. Evidence:
   **E1**.
2. **F2 — finite expansion.** Adapter definitions are acyclic checked definitions. Ordinary reduction is the core's
   total reduction; `recurse_syntax` exposes only immediate proper children; every construction and reduction is
   charged; every limit ends in a refusal and publishes no partial value. Evidence: **E2** and **E11**.
3. **F3 — determinism.** Resolution, elaboration, case compilation, NbE, quotation identity, recursor execution, the
   output gate, and each budget transition are functions of their written inputs, registry version, and compiler
   options. Equal inputs therefore give equal values, paths, diagnostics, and charges. Evidence: **E3**.
4. **F4 — hygienic quotation and substitution.** A literal quoted binder receives a derived scope coordinate; spliced
   syntax retains its existing scopes. Neither can capture the other. Instantiating a quote with `quote { e }` at a hole
   produces the same checked expression and type as writing `e` at that position. Evidence: **E4**.
5. **F5 — unique derived identity.** A literal quotation node is identified by `(origin, quotation, literal-path)`. Tree
   positions are injective inside one quote, quotation coordinates separate quotes at one origin, and origins separate
   expansion sites. Equality of all three coordinates is sufficient and necessary. The duplicate-path gate remains an
   executable assertion against an implementation defect. Evidence: **E5**.
6. **F6 — source attribution.** Spliced nodes retain `Original`; literal nodes receive `Generated` with the quote as
   generation site and the `at` node as source root. Traversal supplies the compiler-assigned child path and
   `syntax_anchor` maps it to the original region. Expansion records cover the nodes they emit. Evidence: **E6**.
7. **F7 — edit locality.** An adapter edit returns one text patch within its owned region. Applying it changes exactly
   the selected anchored spelling; bytes outside the patch are identical. Unsupported commands refuse without an edit.
   Evidence: **E7**.
8. **F8 — claimed print round-trip.** Staff and graph printers claim value-level, not byte-level, round trip. Parsing
   and expanding their canonical output returns the same `StaffDocument` or valid `StudioDescription`; every
   unrepresentable value is a named loss. Evidence: **E8**.
9. **F9 — one match evaluator.** A syntax quotation pattern compiles through the ordinary pattern matrix and case-tree
   compiler. It constrains a shape but does not enumerate `Syntax`; therefore ordinary coverage still requires a
   fallback. Matching ignores trivia and identity while bound syntax retains both. Evaluation is the one core case-tree
   evaluator. Evidence: **E9**.
10. **F10 — derivation coverage and composition.** Every addressable output has a finite derivation DAG to source
    anchors. Composition grafts the first derivation at every matching input leaf of the second, rejects a missing exact
    presentation/anchor, preserves all parents of combined or generated steps, covers every output, and is associative
    up to the graph's canonical ordering. Evidence: **E10**.
11. **F11 — sealed structural descent.** A `SyntaxStep C A` is compiler-minted for one immediate proper child and the
    algebra that exposed it. Running it supplies exactly the requested context to that child under that algebra. It
    cannot reveal or replace its child, path, scopes, source, or algebra. It may be omitted, captured, delayed, or run
    finitely many times; every run repeats the same association and is charged. Running all children once in order at
    unused context is the derived leaf fold. Evidence: **E11**.
12. **F12 — two unprivileged witnesses.** Staff and studio receive the same public token-tree value, use the same
    recursor and ordinary finite folds, return one checked expression, receive neither inferred types nor hidden parser
    input, and require no operation beyond the frozen set. Their musical data and validation are disjoint. Evidence:
    **E12**.

## 3. Proofs

### 3.1 Termination and the sealed-step reducibility case

Use the core reducibility interpretation `⟦A⟧`. For a finite syntax subtree `s`, define a sealed step over `s` to be
reducible at `SyntaxStep C A` exactly when, for every `c ∈ ⟦C⟧`, running the step at `c` terminates in `⟦A⟧`.

Proceed by structural induction on `s`. Leaves have no child steps. At a group, every exposed step is sealed to one
strict child `si`; the induction hypothesis supplies reducibility for each such step. The algebra is a checked core
term, so the fundamental lemma applies when those reducible steps are substituted into it. This proves the group result
reducible.

Function-valued `C` and `A` are already covered by the function clause of `⟦-⟧`; no first-order exception is used.
Capture and delayed use are substitution into a reducible closure. Duplication invokes the same reducible function a
finite number of times. A nested traversal is another application of the same lemma to a finite subject; after it
normalizes, a captured outer step still uses the outer induction hypothesis because sealing prevents reassociation. Thus
a global runtime tree-size decrease is neither claimed nor needed. Definition acyclicity prevents a branch from naming
itself, and the deterministic meter refuses finite work that exceeds the published budget.

### 3.2 Determinism, hygiene, identity, and substitution

The evaluator has one selected rule at each state; primitive rules are functions; the case tree has one selected
constructor edge; and the meter's successor is a function. Structural induction on the finite evaluation therefore gives
F3. Quotation literal scopes and identities are coordinates, not fresh choices. Induction on the parsed quote tree gives
path injectivity. At a splice, the instantiated node is the supplied syntax value unchanged; at a literal, the node is
rebuilt with its derived coordinate. The two cases prove non-capture and F5.

For substitution, induct on the quote context. The hole case is the definition of instantiation. Every surrounding node
is rebuilt homomorphically, while the splice retains `e`'s scopes and source. Parsing the instantiated tree uses the
same parser and the ordinary checker sees the same term as direct insertion. Hence splicing `quote { e }` equals writing
`e` there, observationally at checked term, type, printed source, and source attribution.

### 3.3 Matching, provenance, edit, print, and derivations

Quotation patterns lower to ordinary pattern columns whose tests compare token-tree shape. The existing pattern-matrix
coverage and case-tree soundness proof therefore applies without a second evaluator; the syntax-specific lemma is that
trivia removal preserves the compared shape and that splice binders return the corresponding original subtrees.

F6 follows by induction on construction and matching: splice is identity on provenance; literal construction adds one
derived coordinate; matching never reconstructs a bound subtree. F7 follows from the patch contract's interval split
`prefix ++ replacement ++ suffix`. F8 is the two executable equalities `expand(print(v)) = v`; losses are explicit
results, not silently discarded fields. F10 is structural grafting on a finite DAG. Grafting at every matching leaf
preserves coverage; graft-at-leaf commutes with grafting at disjoint leaves and associates at the same leaf, proving
associativity after canonical ordering.

### 3.4 Conservativity over the dependent core

**Theorem.** If an ordinary source judgment is written without phase syntax, then `Σ ⊢ e ⇐ A` (or `⇒ A`) before the
phase extension exactly when it holds after the implementation of `P`, with the same elaborated core term, conversion
result, normal form, and indexed-family constructor.

**Proof.** Ordinary documents are resolved with `Σ`, not `Σ,Σφ` (F1). Induct on their bidirectional derivation. Every
rule consults the same binding, family, constructor, universe, and conversion entries as before; there is no phase case
because no phase name resolves. At conversion, NbE receives the same core terms in the same context, so it produces the
same normal forms. Indexed families do not change this argument: `Syntax` is registered only in `Σφ`, and adding a
disjoint family to a registry that the judgment does not receive creates neither a constructor nor an equality in the
ordinary context. Thus derivability and meaning are unchanged.

In the other direction, phase evaluation cannot smuggle a phase value across: the boundary demands `Syntax Expr`, checks
its tree with the ordinary parser and elaborator in `Σ`, and admits only the resulting storable ordinary value.
Consequently the extension is conservative rather than merely inaccessible by convention. This proves law 11 without
claiming prompt 169's independent core-normalization obligation.

## 4. Scope of the freeze

This freezes the implemented two-category interface and no more. It does not freeze a third syntax category, adapter
generated declarations, type-directed expansion, a general macro system, a bridge from `StudioDescription` to DSP, or
the exact numerical budget table. Notes 60 and 66 record the measured costs and the one staff/studio asymmetry: both use
recursor plus folds, but neither needed selective descent after collecting a group.
