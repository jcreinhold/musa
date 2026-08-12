# Metatheory and proof boundary

## 1. What has been proved for the intended target

The following results have fixed statements and proof reviews:

| Result | Status | Governing statement |
| --- | --- | --- |
| Total source core | Existing governing proof, conditional on resource/foreign contracts | `docs/language/02-core-calculus.md` |
| Source-to-temporal closure | Existing governing proof | `docs/language/02-core-calculus.md` §5.7 |
| Finite temporal algebra and unequal overlay | Existing governing laws | `docs/kernel/03`–`05`, `10` |
| Exact versioned temporal framing | Correct mathematical target; implementation pending | `04-identity-and-realization.md` I1 |
| Whole-node registered process totality/determinism/causality | Correct under registry contracts; implementation pending | `03-process-calculus.md` P1–P3 |
| Typed lineage paths form a category | Correct under a fixed well-formed registry; implementation pending | `02-derivation-diagrams.md` |
| Semantic execution factorization and cache correctness | Correct under displayed purity/version/conformance contracts | `04-identity-and-realization.md` R1/C1 |

The independent proof record is retained in `docs/scratch/25`, `30`, `35`, `39`, `42`, and `45`. The failed reviews are
part of the result: each exposed a premise now present in this spec.

## 2. Theorems deliberately not claimed

This specification does not claim:

- that every musical practice factors through one universal object;
- that one global pitch, metre, key, chord, or function domain is adequate;
- that a motif is a split idempotent;
- that harmonic function is a tonic-translation orbit;
- that hashes are collision-free;
- that lineage equality follows from temporal semantic equality;
- that preparation equality implies cross-device bit equality without processor conformance;
- that current caller-block-sensitive feedback implements the process calculus; or
- that nominal theory modules are already implemented or culturally adequate.

## 3. Source representation ownership remains a separate proof gate

The nominal theory-module candidate in `docs/scratch/47-t2b-minimal-source-closure.md` proposes finite non-recursive
nominal data, private constructors, abstract ordinary-structure members, exact `Text`, and compiler-owned `Result`. Its
metatheory and exact programs require independent closure before promotion into `docs/language/` and the prompt stack.

Even after a safety proof, domain adequacy requires real package algorithms and practitioner review. Safety cannot prove
that a ratio list represents gamaka, that a tonic is meaningful for every practice, or that a package's public
vocabulary is respectful and useful.

## 4. Proof obligations for implementation

Every implementation milestone must convert its relevant mathematical premise into executable evidence:

- canonical encoders get delimiter/adversarial/property tests and schema migration tests;
- artifact registries validate exact id/descriptor conflicts;
- process graphs get whole-node schedule counterexamples, registered-feedback, causality, and block-partition tests;
- caches inject digest collisions and confirm full arguments;
- preparation differentials vary every option independently;
- lineage tests retain generated roots/sites and intermediate anchors; and
- source extensions get compile-fail tests plus evaluator equivalence/normalization tests.

Passing a current test suite is evidence about the implementation it exercises, not proof that an absent candidate
representation exists.
