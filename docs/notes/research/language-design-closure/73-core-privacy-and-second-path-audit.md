# 73. The dependent core privacy and second-path audit

**Status: governs nothing.** This is prompt 169's audit of the implemented boundary. It reads public reachability from
the Rust facade and source reachability from resolution separately; a Rust host API is not automatically a source
feature.

## Privacy

The semantic domain is private. `musa-calculus::kernel` and `elaboration` are private modules; `Value`, environments,
closures, evaluator frames, the meter, case trees, termination witnesses, metavariable state, and `Storable`'s decision
walk are not re-exported. `boundary_laws::the_kernel_names_nothing_the_elaborator_defines` checks the trusted dependency
direction. Public `Term`, `Raw`, declarations, registries, budgets, and facade operations are compiler-host data needed
to construct and audit a program; source has no reflection operation that returns any of them.

The public `Payload::as_any` is reachable only by the host that registered the literal. The core never downcasts it, and
source can inspect an inert base only through a registered total rule or literal pattern. `Base::storable` is an owner
promise, not a source declaration. Source cannot name `Storable`, phase types, registry ownership, a normal form, or a
budget. Diagnostics may print a term and a resource metric; neither is a value a program can consume. Ordinary
resolution omits the phase registry, so `Syntax`, paths, scopes, source information, and quotation operations do not
cross back into ordinary source.

No private representation, registry decision, authored-invisible normal form, or budget is therefore reachable from
source. The narrow host facade intentionally exposes the syntax and audit objects a compiler must provide; it exposes no
semantic `Value`.

## Second paths

| Apparent duplicate | Finding |
| --- | --- |
| Elaborator conversion and kernel rechecking conversion | Intentional independence. Elaboration's walk solves metas and builds mismatch paths; rechecking uses finished terms and kernel `quote ∘ eval`. Sharing it would let the audit trust the solver it audits. Both share the one evaluator, which is why K19 is not NbE evidence. |
| Case-tree compilation and generated recursors | One semantics, two stages. The compiler emits ordinary recursor applications; `kernel/eval.rs` supplies the sole ι evaluator. `coverage_laws::a_match_is_the_recursor_it_compiles_to` detects divergence. |
| `Storable` and event payload admission | Sequential predicates on different facts. `Storable` rejects Π anywhere in the value's type; event admission requires a canonical, versioned exact encoding for the admitted payload. A type must pass both at the stage boundary. |
| Elaboration and `recheck_program` | Deliberately independent typing derivations across the trusted boundary. The latter neither solves metas nor produces user diagnostics and is evidence only for K19. |
| Direct normalization in tests and production conversion | The direct `normalize`/α-equality route is retained only as an oracle in `conversion_laws`; production calls the type-directed conversion walk. This is independent evidence, not a second production semantics. |
| Syntax-pattern matching and ordinary matching | Syntax patterns lower into the ordinary pattern matrix and case tree. There is no syntax evaluator; note 67 F9 and its executable control freeze that fact. |
| Source and phase registries | Deliberately disjoint namespaces with the same core evaluator. Ordinary source never receives the phase registry; calling it a fifth source builtin family would erase the phase boundary. |

No undocumented second semantic path remains in prompt 169's language boundary. Runtime, scheduling, and audio paths are
outside this audit and belong to prompt 174.
