# 71. Final review of the adapter freeze

## Verdict

**Correct under the stated contracts.** No fatal, High, or Medium finding remains.

The reviewed object is notes 67–70 and `scripts/check-syntax-adapter-conformance.sh` after the repair in note 70. The
verdict is deliberately bounded: prompt 169 still owns the core metatheory matrix, and note 67 states its reliance on
the core normalization/re-checking obligations instead of presenting executable tests as a mechanized proof.

## Re-review

- **Termination:** the phase adds one eliminator. The structural induction and reducibility candidate cover leaves,
  groups, higher-order context/result, capture, duplication, delayed use, and nested traversal. The proof does not rely
  on a false globally decreasing runtime subject. E2 and E11 exercise the local equations, hostile nested case, repeated
  use, charging, safe deep acceptance, and bounded refusal.
- **Determinism, hygiene, substitution, and identity:** quotation uses structural coordinates rather than a fresh-name
  effect. The literal/splice induction covers binder introduction and retained scopes. E3–E5 run the distinguishing
  cases that defeated the historical design.
- **Matching and source:** syntax patterns use ordinary coverage and the ordinary case tree. Matching ignores trivia and
  identity for shape while returning the original bound subtree. E6 and E9 exercise both halves.
- **Editing and printing:** the claims are scoped to patch locality and value-level round trip, with refusal/loss rather
  than unclaimed textual identity. E7 and E8 run both adapters.
- **Derivations:** the object is the implemented finite multi-parent DAG. Grafting covers all leaves and is associative;
  it is not the unary list concatenation rejected historically. E10 includes both graph algebra and real compilation
  coverage.
- **Conservativity:** ordinary resolution receives no phase environment. Induction over the unchanged dependent
  judgment, including its conversion case, proves unchanged ordinary derivability and normal forms. The output gate
  removes phase values before ordinary checking. E1 now runs the direct environment differential and storable refusal.
- **Musical adequacy:** note 68 supplies concrete types, values, stage transitions, losses, added choices, and both
  routes for all five retained cases. It neither invents a shared final representation nor hides an open performance
  choice in notation.

## Historical audit

Historical notes 34–37 at `d0f4a527^` were read twice where required. Their failures have direct successors: typed
quotation replaces undefined quotation; derived coordinates replace contradictory freshness; ordinary case trees replace
an undefined match target; path-aware traversal supplies constructible paths; generated derivation edges retain reuse
sites; and a DAG represents combined ancestry. None is dismissed merely because a test is green.

## Residual low observations

The conformance script intentionally repeats a few fast laws under more than one rule because one witness crosses two
contracts. The five musical programs remain research pressure tests rather than installed standard-library APIs. These
are not correctness findings and require no repair in this prompt.
