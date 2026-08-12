# T₂a — a fixed calculus of nominal theory modules

**Status: fixed source-calculus and proof draft; governs nothing.** This is the smallest implementable fragment
extracted from Candidate T₂ and the paper prototypes [31](31-candidate-theory-modules.md),
[36](36-theory-module-paper-prototypes.md). It is written as a fixed target for independent proof review.

T₂a adds:

1. monomorphic, non-recursive nominal algebraic data;
2. private constructors;
3. abstract data members in ordinary static signatures; and
4. realization and sealing of those members by ordinary structures.

It does **not** add user type parameters, recursive user data, type aliases, subtyping, first-class modules, implicit
module search, value dependency, or abstract result members in structure templates. Existing value-only templates remain
unchanged, but templates over the new abstract members are deferred until a separate calculus gives their result-type
identity and alias rules.

---

## 1. Core target

Let `K` be K₃ᴱ's monomorphic total core. Extend its atomic types with nominal stamps `μ`. The existing constructors
`Option`, `List`, and products remain compiler-owned and may contain nominal types. A new user data type does not define
its own equality, recursion, method, or runtime type representation.

### 1.1 Ranked non-recursive declarations

A nominal data declaration is:

```text
data μ {
    C₁(F₁₁,…,F₁k₁),
    …
    Cₙ(Fₙ₁,…,Fₙkₙ),
}
```

Field types are:

```text
F ::= b | μ | F×F | Option F | List F
```

where `b` is an existing inert finite/core data type and every nominal `μ` in a field has **strictly smaller declaration
rank**. There is no arrow anywhere in `F`, and the declared stamp itself cannot occur in its fields. Mutual recursion is
therefore impossible. A `List μ` field is allowed only when that `μ` is earlier, not the type currently being declared.

Each constructor name is unique within its data declaration. Constructors are namespaced by their owner, so equal short
names in different types do not collide.

### 1.2 Term extension

```text
e ::= …
    | C_μ(e₁,…,eₙ)
    | match e with { C₁(x̄₁) => e₁; …; Cₙ(x̄ₙ) => eₙ }

v ::= … | C_μ(v₁,…,vₙ)
```

Every match lists every constructor of the scrutinee's nominal type exactly once. There are no nested constructor
patterns or guards in T₂a. A field variable is a binder. Existing wildcard/literal/`Option`/`List` match forms retain
their governing behavior.

Typing adds:

```text
C_μ : F₁×…×Fₙ → μ    Γ⊢eᵢ:Fᵢ
──────────────────────────────── Data-Intro
Γ⊢C_μ(e₁,…,eₙ):μ

Γ⊢e:μ
for every C_j(F̄_j), Γ,x̄_j:F̄_j ⊢ e_j:A
constructors(μ) = constructors listed exactly once
──────────────────────────────────────────────────── Data-Match
Γ⊢match e with { C_j(x̄_j)=>e_j } : A
```

Evaluation is left-to-right call by value. The new principal reduction is:

```text
match C_j(v̄) with { …; C_j(x̄)=>e_j; … } ↦ e_j[v̄/x̄].
```

No other operation can inspect a nominal data value.

The compiler-owned δ registry is **not** extended over user nominal types. Existing δ-operations retain their already
proved signatures over compiler-owned inert base types. A theory operation over `μ` is an ordinary checked function
defined with constructors and exhaustive match. Adding a foreign primitive over `μ` would require a separate ownership
record and compatibility proof; T₂a does not admit it.

### 1.3 Canonical data

The compiler generates the semantic equality and versioned canonical encoding:

```text
enc_μ(C_j(v₁,…,vₙ)) =
  encode(stable_stamp(μ), data_schema_version, j, enc(v₁), …, enc(vₙ)).
```

Equality is equality of constructor and fields at the same nominal stamp. A package cannot override it. If a domain
needs a quotient, it defines a separate canonical nominal type and a smart constructor returning `Result`.

`stable_stamp(μ)` is derived from locked package identity and qualified declaration path. Renaming or moving a public
type is an identity/schema change. Private constructor renaming is likewise an encoding change unless an explicit
migration retains the old version; the compiler must not silently make stored hashes mean new values.

## 2. Static module syntax

T₂a modules remain compile-time declarations, never values.

```text
signature S {
    data type T₁;
    …
    data type Tₙ;
    let x₁ : A₁;
    …
    let xₘ : Aₘ;
}

structure M : S {
    data T₁ { … }
    …
    data Tₙ { … }
    let y₁ : B₁ = e₁;
    …
    let yₖ : Bₖ = eₖ;
}
```

Signature value-member types may mention prior abstract members as `T` inside the signature. A structure value type may
mention its realized member as `T` inside the structure. Outside, an exported type is named `M.T`.

T₂a has no manifest type specification and no transparent type alias. A structure realizes `data type T;` only by a new
nominal `data T {…}` declaration. Every ascribed structure therefore gives each abstract member one concrete nominal
identity. The structure does not separately declare a representation type and equate it to `T`.

Constructors of an ascribed `data T` are visible only inside `M`. The external environment exports the stamp under path
`M.T` and the `let` members required by `S`; it does not export constructor paths. Extra `let` declarations are private.
Extra data declarations are rejected in T₂a rather than given ambiguous external identity.

## 3. Signature and structure judgments

A static environment `Σ` maps:

- signature names to ordered member specifications;
- qualified external paths `M.T` to nominal stamps;
- exported paths `M.x` to monomorphic types; and
- internal constructor paths to their signatures while checking their owner.

The judgments are:

```text
Σ ⊢ signature S ok
Σ ⊢ structure M : S ⇒ Σ'
Σ ; Γ ⊢ e : A
```

### 3.1 Signature validity

A signature is valid when:

1. member names are distinct;
2. every type member precedes every value signature which mentions it;
3. every value-member type is well formed after replacing each abstract member `T_i` by a fresh rigid stamp `α_i`;
4. value-member types are monomorphic; and
5. the member dependency order is finite and acyclic.

The rigid stamps are skolems used only to check the signature; they do not become runtime types.

### 3.2 Structure checking algorithm

To check `structure M:S`:

1. Look up valid `S`. Reject a missing/duplicate required member or extra data member.
2. For each `data type T_i` of `S`, assign the stable fresh nominal stamp `μ_{M,T_i}` and form substitution
   `θ=[T_i↦μ_{M,T_i}]`.
3. Check the structure's `data T_i {…}` at `μ_{M,T_i}` in signature order. Every field type must be an existing data
   type or a `μ_{M,T_j}` with `j<i`; this supplies the rank condition.
4. Replace abstract members by `θ` in every required value type. Collect all value declarations and their written
   monomorphic types; require every required name with exactly the substituted type.
5. Check every value body in the internal environment containing all `μ_{M,T_i}`, all private constructors, and all
   structure value signatures. Record an edge for every sibling value reference and reject a cyclic dependency graph.
6. Elaborate the finite acyclic value graph to qualified K-core definitions in topological order.
7. Extend `Σ` externally with `M.T_i↦μ_{M,T_i}` and exactly the required `M.x_j:θ(A_j)`. Hide constructors and extra
   values.

The algorithm does not compare implementations structurally. Two structures which ascribe the same signature receive
different stamps and their corresponding abstract members are not definitionally equal.

### 3.3 External path typing

```text
Σ(M.T)=μ
──────────── Path-Type
Σ ⊢ M.T type

Σ(M.x)=A
──────────── Path-Value
Σ;Γ ⊢ M.x:A
```

There is no coercion between `M.T` and `N.T`. A translator is an explicitly exported function whose type mentions both
paths.

## 4. Decidability and elaboration

### Theorem 4.1 — signature and structure checking are decidable

For finite syntax and a finite valid `Σ`, checking a T₂a signature or structure either produces a finite extension `Σ'`
and finite monomorphic core declarations or a finite diagnostic.

*Proof.* Signature-member uniqueness, ordering, and type formation are finite structural checks. The structure algorithm
has seven bounded phases. Stamp generation is deterministic lookup from the qualified owner path. Field formation and
constructor uniqueness recurse over finite syntax; the strict member order decides the rank condition. Required-member
matching is finite name lookup followed by the governing decidable monomorphic type equality. Body checking is the
governing decidable K checker extended by the syntax-directed rules of §1.2. Cycle checking on the finite sibling graph
and topological sorting are decidable. Every phase terminates and has a finite diagnostic for failure. ∎

### Lemma 4.2 — sealing preserves external typing

If the structure algorithm accepts `M:S`, every external exported path has exactly the type obtained by substituting
`M.T_i`'s stamps for `S`'s abstract members, and no external term can name a private constructor through `Σ'`.

*Proof.* Phase 4 checks each required value under precisely substitution `θ`; phase 5 proves its body at that type;
phase 7 exports the same stamp and substituted types. Phase 7 adds no constructor mapping, and the only constructor
typing rule requires lookup of its owner-qualified constructor in the current environment. Therefore external checking
can use exported values and opaque stamps but cannot derive a constructor introduction or match for a hidden member. ∎

### Lemma 4.3 — structure elaboration preserves value typing

If `Σ⊢structure M:S⇒Σ'`, each generated qualified K definition is well typed at the type recorded for it in the internal
environment, and every required external value has the type recorded in `Σ'`.

*Proof.* Topologically order the accepted sibling graph. Induct on that order. The first body refers only to
constructors, external paths, and prior project definitions already well typed in `Σ`; phase 5 checked the body at its
annotation, so its qualified definition is well typed. At the inductive step, sibling dependencies precede the
definition and have the types assumed during phase 5 by the induction hypothesis. Qualifying a sibling name changes
lookup path, not its stamp or type. Therefore the same typing derivation is valid. Required members have the external
types by Lemma 4.2. ∎

## 5. Core safety extension

The new data eliminator means these types are not inert δ-base types. They need their own proof case.

Order nominal types by declaration rank. Define reducibility simultaneously by type structure and nominal rank:

```text
R_μ(e) iff
  e is well typed at μ and strongly normalizing, and
  if e↦*C_j(v₁,…,vₙ), then R_F₁(v₁),…,R_Fₙ(vₙ).
```

Every nominal field type has lower rank, so this definition is well founded. `Option`, `List`, products, and arrows use
the governing reducibility clauses.

### Lemma 5.1 — nominal constructors preserve reducibility

If every `e_i` is reducible at field type `F_i`, then `C_μ(ē)` is reducible at `μ`.

*Proof.* Left-to-right evaluation of finitely many strongly normalizing fields terminates. The result is the constructor
value `C_μ(v̄)`, whose fields remain reducible by closure under reduction. Constructor syntax has no other redex.
Candidate expansion gives reducibility of the original term. ∎

### Lemma 5.2 — exhaustive nominal match preserves reducibility

If the scrutinee is reducible at `μ` and, under reducible substitutions for its fields, every constructor branch is
reducible at `A`, then the exhaustive match is reducible at `A`.

*Proof.* Reduce the scrutinee. Strong normalization yields one constructor normal form by progress and canonical forms.
Its fields are reducible by `R_μ`. Exhaustiveness and uniqueness select one branch, and the branch premise makes its
simultaneous substitution reducible. Candidate expansion handles the scrutinee reductions and the principal match step.
∎

### Theorem 5.3 — conservative core extension

Adding any finite rank-ordered family of T₂a nominal data declarations and their constructor/match forms preserves
preservation, progress, deterministic reduction, and strong normalization of K₃ᴱ, relative to K₃ᴱ's existing foreign
operation and `Music` contracts.

*Proof.*

- **Preservation:** constructor formation uses its declared fields. Match reduction substitutes constructor values for
  variables checked at exactly those field types; the ordinary simultaneous substitution lemma gives the branch result
  type.
- **Progress:** a constructor advances its leftmost non-value field or is a value. A match advances its scrutinee or, by
  nominal canonical forms, sees one declared constructor; exhaustive unique coverage supplies the matching redex.
- **Determinism:** left-to-right contexts select one field/scrutinee, and a constructor tag selects one unique branch.
- **Strong normalization:** extend the governing fundamental lemma with Lemma 5.1 for constructor introduction and Lemma
  5.2 for match. The rank condition makes the new candidate interpretation well founded. Existing higher-order,
  structural-eliminator, δ-operation, and contextual-`Music` cases are unchanged. Existing polymorphic structural
  eliminators instantiate at the new monomorphic types and use their already proved reducibility arguments. Foreign
  δ-signatures remain exactly the old compiler-owned signatures and therefore require no new inspection case.

Thus every closed accepted extended-core term has one finite typed value. ∎

### Corollary 5.4 — accepted ordinary structures are total

If a finite project consisting of existing K₃ᴱ declarations plus T₂a ordinary signatures/structures has an acyclic
project and sibling dependency graph and all ownership contracts hold, evaluating any accepted exported value terminates
deterministically at its declared type.

*Proof.* Theorem 4.1 produces a finite monomorphic core extension; Lemma 4.3 preserves typing. Flatten the finite
topologically ordered project to non-recursive qualified lets. Apply Theorem 5.3 and K₃ᴱ totality. ∎

## 6. Separate compilation and generativity

For one locked package graph, public stamps are reproducible from package identity, qualified path, and schema version.
This is applicative identity for ordinary named structures: importing the same locked declaration twice names the same
type, while two differently qualified structures name different types.

This is not runtime generativity. T₂a has no expression which creates a fresh type, and no stamp comparison operation.
The compiler may represent stamps as internal ids after resolution because all checking is static; exported metadata and
canonical encodings retain the stable external stamp.

Existing `template structure` remains usable only when its parameter and result signatures contain **value members
alone**, exactly its current proved scope. T₂a does not define what happens when a template:

- consumes a structure with abstract data members;
- aliases an argument member into its result;
- returns a fresh abstract member; or
- is instantiated twice at equal values.

Those features require a separate substitution/generativity calculus. The admitted-repertoire sketch in [36 §3.1]
(36-theory-module-paper-prototypes.md) is evidence for that later question, not syntax T₂a has already earned.

## 7. Diagnostics and information hiding

Required static errors are:

- duplicate/missing/extra data member;
- constructor collision inside one data member;
- recursive or forward field reference;
- arrow-valued field;
- missing/duplicate/non-exhaustive match arm;
- value member with a type different from the substituted signature type;
- cyclic sibling value definitions; and
- external reference to a hidden constructor or private value.

Diagnostics display the short path at the use and the fully qualified stamp owner in a note. They do not print internal
numeric stamps. A hidden-constructor error points to the owning signature's `data type T;` and suggests the exported
smart constructor when one has the expected result type.

## 8. Falsification examples

### 8.1 Two theory carriers do not mix

```musa
signature Practice { data type Intent; let default: Intent; }
structure A : Practice { data Intent { AIntent } let default: Intent = AIntent; }
structure B : Practice { data Intent { BIntent } let default: Intent = BIntent; }
```

`A.default` has type `A.Intent`; a function accepting `B.Intent` rejects it. Equal constructor arity or encoding shape
does not identify the stamps.

### 8.2 Private constructor does not leak

Outside `A`, both `AIntent` and `match A.default with AIntent=>…` are rejected because `Σ'` exports no constructor.
`A.default` remains usable through functions whose types mention `A.Intent`.

### 8.3 Translation is explicit

```musa
fn translate(x: A.Intent) -> Option B.Intent { ... }
```

must live where the owners export enough observations or an authorized conversion. No equality/coercion arises from both
structures matching `Practice`.

### 8.4 No recursive escape

```musa
data Loop { Again(List Loop) }
```

is rejected because `Loop` occurs in its own field, even under an existing list. This is a finite value type in many
languages, but no prototype needs user recursive data and its fold/termination surface has not earned admission.

## 9. Verdict and next action

T₂a is the first source extension in this notebook whose new typing and reduction rules are both small and directly
motivated by plural music theories. It does not formalize those theories; it gives them representation ownership.

Do not schedule implementation until an independent review accepts or repairs Theorems 4.1–5.3. If the proof survives,
the next action is a parser/checker spike for exactly T₂a—no recursive data, aliases, or abstract structure templates—on
the two paper packages. Only measured clarity and a real second package should then justify graduating it into the
language spec and prompt stack.
