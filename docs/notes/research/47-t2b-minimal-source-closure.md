# T₂b — minimal source closure

**Status: repaired source-calculus candidate; governs nothing.** The independent review of T₂a
([46](46-proof-review-t2a.md)) accepted its nominal-data safety argument but refuted its adequacy claim and found three
exact module-elaboration gaps. This note repairs those gaps. It also separates two questions which T₂a had conflated:

1. Is the nominal module calculus safe and decidable?
2. Do the proposed music-theory packages inhabit it without borrowing unstated language features?

The answer to the first is now intended to be yes. The answer to the second is deliberately narrower: the two complete
programs in §7 are **mechanism probes**, not adequate music-theory libraries. The richer paper interfaces in
[36](36-theory-module-paper-prototypes.md) remain pressure tests. In particular, the admitted-repertoire template in
§36.3.1 is not a T₂b program.

T₂b is T₂a plus exactly these changes:

- a compiler-owned exact `Text` base type;
- a compiler-owned structural `Result A E` type;
- functional declaration namespaces;
- separate public source and retained private core environments; and
- exact owner-canonical nominal identities and framed encodings.

It adds no aliases, recursive data, user generics, abstract-member templates, first-class modules, dependent types,
subtyping, implicit search, or runtime fresh types.

---

## 1. Why `Text` is ordinary data here

The current core excludes strings because every existing string belongs to an owning declaration grammar. That remains
right for file paths, asset references, notation quotations, and studio parameter paths. A theory package nevertheless
needs finite names which are not members of a compiler-owned musical enumeration: a named fingering, oral lineage,
learned gesture, analytical rule, or vocabulary item. Encoding those names as `Nat` or a universal compiler `Pitch`
would erase the distinction T₂ is meant to preserve.

Define a `Text` value to be a finite sequence of Unicode scalar values obtained after literal escape processing. The
language performs no implicit Unicode normalization: canonically equivalent spellings are unequal unless a package
explicitly normalizes them before construction. The first admission supplies only:

```text
text_equal : Text × Text → Bool
```

and literal formation. There is no parse, reflection, path conversion, quotation conversion, ambient localization, or
general character fold. A package may store or compare a name; it cannot smuggle source syntax or an asset path through
`Text` and ask the kernel to interpret it.

### Lemma 1.1 — `Text` is a conservative base-domain admission

Adding `Text` and the displayed primitive preserves the total-core safety theorems.

*Proof.* Apply `docs/rules/language/02-core-calculus.md` Theorem 5.

- **D1:** a text value is an opaque base constant. Literal and catch-all patterns do not expose its scalar sequence.
- **D2:** equality is total on two finite sequences and returns a closed compiler-owned boolean.
- **D3:** the result depends only on the two scalar sequences. No locale, normalization service, hash order, or ambient
  state is observed.
- **D4:** equality examines at most both finite sequences and constructs a constant-size result. Literal construction is
  charged by scalar length before allocation.

The new reducibility clause is the existing base clause `R_Text(t) iff t:Text and t∈SN`. Theorem 5 therefore gives
preservation, progress, determinism, and strong normalization. ∎

`Text` is not a universal musical carrier. A package that makes every distinction a text tag has abandoned nominal
ownership and should fail review even though it typechecks.

## 2. The compiler-owned `Result` constructor

Extend core types, terms, and values by:

```text
A ::= … | Result A E
e ::= … | Ok(e) | Err(e)
          | match e with { Ok(x) => e_ok; Err(y) => e_err }
v ::= … | Ok(v) | Err(v)
```

The compiler instantiates `Result` at concrete monomorphic types exactly as it already instantiates `Option` and `List`.
This is not user-polymorphic syntax. An exhaustive result match has one `Ok` and one `Err` arm, in either order, with no
duplicate. Typing and principal reductions are:

```text
Γ⊢e:A                         Γ⊢e:E
──────────── Result-Ok        ──────────── Result-Err
Γ⊢Ok(e):Result A E            Γ⊢Err(e):Result A E

Γ⊢r:Result A E    Γ,x:A⊢p:C    Γ,y:E⊢q:C
──────────────────────────────────────────── Result-Match
Γ⊢match r with { Ok(x)=>p; Err(y)=>q }:C

match Ok(v)  with { Ok(x)=>p; Err(y)=>q } ↦ p[v/x]
match Err(v) with { Ok(x)=>p; Err(y)=>q } ↦ q[v/y].
```

Evaluation first evaluates the constructor argument or match scrutinee, left to right as elsewhere.

### Lemma 2.1 — `Result` is a conservative structural extension

Adding `Result` preserves preservation, progress, deterministic reduction, and strong normalization.

*Proof.* The preservation, progress, and determinism cases are identical to the two-constructor fragment of the already
proved `Option` cases. Define

```text
R_Result A E(t) iff
  t:Result A E, t∈SN, and
  t→*Ok(v)  implies R_A(v), while
  t→*Err(w) implies R_E(w).
```

Constructor introduction preserves the candidate of its selected field. For result matching, reduce the scrutinee;
canonical forms selects exactly one arm, whose reducibility follows from the fundamental-lemma premise under the
reducible field substitution. Candidate expansion handles scrutinee and principal reductions. This extends the
fundamental lemma by the same finite structural induction as `Option`. ∎

`Result` earns standard status because it expresses one distinction repeatedly and without musical commitment:
successful finite construction versus an owned finite failure. Requiring every package to declare a bespoke nominal
two-case carrier would not increase representation independence; it would only obscure a structural sum already owned by
the language.

## 3. Functional static environments

T₂b keeps T₂a's rank-ordered non-recursive nominal declarations and constructor/match rules, extending its field grammar
only by `Result F F`. It strengthens declaration formation as follows.

For one locked build graph:

1. signature names, structure names, and exported module paths are unique at their owner scope;
2. all type and value member names within a signature are pairwise unique, including across member kinds;
3. all data and value declaration names within a structure are pairwise unique, including private values;
4. constructor names are unique within their nominal owner, and owner-qualified constructor paths are unique globally;
5. every qualified path resolves to at most one declaration before checking begins; and
6. import aliases affect source lookup only and never nominal identity.

A duplicate is rejected before member matching, body checking, or dependency-graph construction. Thus every member
lookup and sibling graph vertex is a partial function, never a first/last-definition convention.

### Lemma 3.1 — structure checking is decidable and functional

For finite syntax and a finite well-formed input environment, T₂b structure checking either returns one result or one
finite diagnostic.

*Proof.* The new namespace conditions are finite duplicate checks. After they pass, every lookup used by T₂a's
seven-phase algorithm is functional. Field formation, exact monomorphic type equality, body checking, finite dependency
cycle detection, and topological sorting remain decidable. Fix a deterministic diagnostic precedence and canonical
topological tie-break by owner-canonical declaration path. No phase contains search with two accepted results. ∎

## 4. Sealing retains the private core signature

Source opacity and target well-typedness require different environments. Use:

```text
Σ_public  source paths a client may name
Δ_data    all nominal stamps, schemas, and constructor signatures
Γ_core    qualified value signatures retained by the elaborated core program
```

The structure judgment is now:

```text
Σ_public ; Δ_data ; Γ_core ⊢ structure M : S
  ⇒ Σ_public' ; Δ_data' ; Γ_core' ; definitions.
```

During body checking, a temporary internal source environment contains the new nominal member paths, private
constructors, and all unique sibling value signatures. On success:

- `Σ_public'` adds only `M.T` and the signature-required `M.x` paths;
- `Δ_data'` retains every accepted nominal descriptor and private constructor signature;
- `Γ_core'` retains every qualified generated value signature needed to type the flattened definitions; and
- `definitions` contains the checked qualified core bodies in canonical topological order.

Hiding a constructor means omitting its source path from `Σ_public'`; it never deletes its type metadata or compiled
case tag from `Δ_data'`.

### Lemma 4.1 — sealing preserves target typing and source opacity

If the repaired judgment accepts `M:S`, then every returned core definition is well typed under `Δ_data';Γ_core'`, every
public value path has its substituted signature type, and no client checked only under `Σ_public'` can introduce or
eliminate a private nominal member.

*Proof.* T₂a Lemma 4.3 applies to the temporary internal environment and the canonical topological order. All
constructor signatures used by generated bodies are retained in `Δ_data'`, so qualifying and sealing do not invalidate a
typing premise. Required value signatures are copied unchanged into `Σ_public'` and `Γ_core'`. Conversely, constructor
introduction and nominal match require a constructor lookup; private constructor paths are absent from `Σ_public'`, so a
client source derivation cannot form either rule. Retaining the same path in `Δ_data'` affects target validation, not
source name resolution. ∎

## 5. Exact nominal identity

A nominal declaration owns this complete key:

```text
StampKey = {
  nominal_format_version,
  owner_package_ref,
  owner_module_path,
  owner_structure_path,
  member_name,
  member_schema_version,
}.
```

`owner_package_ref` is the exact immutable package reference accepted by the locked package registry, including its
canonical manifest identity. Equal references from separately loaded artifacts must resolve to byte-identical canonical
manifests; otherwise registry merge rejects an identity conflict. Module, structure, and member paths are the owner's
canonical declaration paths after resolution, never local import spellings. Every component implements K₃.3's
`CanonicalData` contract with domain tags, version tags, counts, and length-framed variable children.

Nominal type equality is exact `StampKey` equality. A finite digest may index a stamp table, but lookup confirms the
full framed key. Two import aliases of one locked declaration therefore name one type. Two different structure owners
name different types even when their declarations have identical text. A constructor/field/schema change requires a new
member schema version or owner package identity; reusing the exact key for changed schema is a registry admission error.

A nominal value encodes as:

```text
encode_nominal(v) = frame(
  nominal_value_encoding_version,
  StampKey,
  constructor_ordinal,
  field_count,
  frame(enc(field_1)), …, frame(enc(field_n)),
).
```

Constructor order is part of the member schema. Encoding equality is admitted nominal equality: same exact stamp,
constructor, and recursively equal fields. Display text is separate and is never hashed as the nominal value encoding.

### Lemma 5.1 — nominal encoding is exact at a fixed schema

At one well-formed registry, two closed nominal values have equal canonical bytes exactly when they are equal by stamp,
constructor, and admitted field equality.

*Proof.* Unique framing decodes the outer version and exact `StampKey`, then the ordinal, count, and each field without
delimiter ambiguity. Induct over the rank-ordered field grammar, using each compiler-owned child's `CanonicalData`
contract. Conversely, equal nominal structures emit equal framed components by determinism. ∎

## 6. Combined safety theorem

Let `r(A)` be the maximum declaration rank of any nominal stamp in `A`, with no nominal stamp assigned rank zero. Order
candidate definitions lexicographically by `(r(A), size(A))`. Product, `Option`, `List`, `Result`, and arrow clauses
recurse to a strict subtype, so rank does not increase and size decreases. The nominal clause at stamp `μ` recurses to
field types whose nominal stamps all have rank strictly below `rank(μ)`, even when a field wraps them in a list, option,
or result.

### Theorem 6.1 — conservative source extension

Assume the governing total-core foreign-operation and contextual-`Music` contracts. Adding finite-sequence `Text`,
compiler-owned `Result`, and any finite rank-ordered family of T₂b nominal declarations preserves preservation,
progress, deterministic reduction, and strong normalization. Every accepted finite ordinary structure graph evaluates
its exported values deterministically and terminates within the separate resource-acceptance premise.

*Proof.* Lemma 1.1 admits `Text`. Lemma 2.1 admits `Result`. T₂a's reviewed constructor and nominal-match cases
establish the nominal extension; the displayed lexicographic measure makes their simultaneous candidate definition
explicit. No nominal field contains an arrow or its own stamp, the foreign δ registry cannot inspect user nominals, and
source definitions remain finite and acyclic by Lemma 3.1. Lemma 4.1 preserves the elaborated target typing after
sealing. Apply the extended fundamental lemma and the governing resource premise. ∎

This theorem proves language safety, not cultural adequacy, usefulness of a package API, or correctness of a musical
algorithm.

## 7. Two complete mechanism probes

These probes use core notation (`λ`, explicit constructor owners, and flat exhaustive matches) so every term is visible.
They are not proposed surface spelling.

### 7.1 W — wrapping an existing launch domain without aliasing it

```text
signature WrappedPitchTheory {
  data type Written;
  data type Motion;
  let written : Pitch → Written;
  let motion : Interval → Motion;
  let observe_written : Written → Pitch;
  let observe_motion : Motion → Interval;
}

structure W : WrappedPitchTheory {
  data Written { WrittenValue(Pitch) }
  data Motion { MotionValue(Interval) }

  let written : Pitch → Written = λp. WrittenValue(p);
  let motion : Interval → Motion = λi. MotionValue(i);

  let observe_written : Written → Pitch =
    λw. match w with { WrittenValue(p) => p; };

  let observe_motion : Motion → Interval =
    λm. match m with { MotionValue(i) => i; };
}
```

This probe establishes only the language mechanism: an ordinary package can wrap a compiler-owned launch domain, export
theory-owned carriers, and keep their representation constructors private without a manifest alias. It does **not**
establish a theory of transposition or show that future theory packages should reuse the current Western `Pitch`; the K
probe does not.

### 7.2 K — a phrase/gesture-shaped owner with no universal pitch or metre

```text
signature PhraseGesturePractice {
  data type Context;
  data type SvaraIntent;
  data type GamakaIntent;
  data type Phrase;
  data type Gesture;
  data type Error;

  let context : Ratio → List Ratio → Context;
  let svara : Text → SvaraIntent;
  let gamaka : Text → List Ratio → GamakaIntent;
  let bind : Context → SvaraIntent → GamakaIntent → Result Phrase Error;
  let realize : Phrase → Gesture;
  let explain : Error → Text;
}

structure K : PhraseGesturePractice {
  data Context { ContextValue(Ratio,List Ratio) }
  data SvaraIntent { SvaraValue(Text) }
  data GamakaIntent { LearnedGesture(Text,List Ratio) }
  data Phrase { BoundUnit(Context,SvaraIntent,GamakaIntent) }
  data Gesture { Trajectory(List Ratio) }
  data Error { EmptyContour(Text) }

  let context : Ratio → List Ratio → Context =
    λtonic. λdrone. ContextValue(tonic,drone);

  let svara : Text → SvaraIntent = λname. SvaraValue(name);

  let gamaka : Text → List Ratio → GamakaIntent =
    λname. λcontour. LearnedGesture(name,contour);

  let bind : Context → SvaraIntent → GamakaIntent → Result Phrase Error =
    λctx. λs. λg.
      match g with {
        LearnedGesture(name,contour) =>
          match contour with {
            [] => Err(EmptyContour(name));
            [head,..tail] => Ok(BoundUnit(ctx,s,g));
          };
      };

  let realize : Phrase → Gesture =
    λphrase.
      match phrase with {
        BoundUnit(ctx,s,g) =>
          match g with {
            LearnedGesture(name,contour) => Trajectory(contour);
          };
      };

  let explain : Error → Text =
    λerror. match error with { EmptyContour(name) => name; };
}
```

Unused bound names in the mathematical presentation are permitted and erase normally; a surface implementation may spell
them `_`. The only list destructor is the governing flat empty/cons match. Constructors are private, so every externally
obtained `K.Phrase` passed through `bind` and therefore contains a nonempty contour. `realize` needs no failure branch.
`Context`, svara intent, learned gesture, phrase, and realization remain distinct types even though the toy
representation uses only exact ratios and text.

This program is not a definition of Karnatak music. It makes no claim that a ratio list adequately represents gamaka,
that a text name determines svara, or that context is exhausted by tonic and drone. Those deliberate inadequacies make
it a parser/checker mechanism probe and keep [36](36-theory-module-paper-prototypes.md) as a domain falsifier rather
than laundering its placeholders into ontology.

## 8. What is still not expressible

T₂b still cannot express:

- the `InContext(P,practice,lineage)` template from [36 §3.1](36-theory-module-paper-prototypes.md), because its result
  aliases `P.Phrase`, re-exports argument members, and creates a fresh admitted type;
- a generic `Derived S T E`, because the source has no user type parameters;
- a value whose type depends on a key, tonic, metre, lineage, or runtime context;
- recursive trees or user-defined folds;
- a structure which realizes an abstract member by a manifest existing type rather than a fresh nominal wrapper; or
- generic traversal over every theory owner.

None of those follows from safety or from the two mechanism probes. The first escalation test remains concrete: write
two real package algorithms and find the smallest operation that cannot be expressed or becomes materially misleading.
Only then specify aliases, abstract-member templates, or dependency. In particular, do not add a `world` or
`theta_link`; owner paths and typed passes already express the distinction these probes need.

## 9. Recommendation

T₂b is small enough to prototype, but not yet ready to govern. The next source-language work should be:

1. independently review Theorem 6.1 and both complete probes;
2. if accepted, implement a parser/checker spike outside the governing surface for `Text`, `Result`, nominal data,
   abstract signature members, private constructors, and ordinary structure sealing;
3. typecheck the two probes exactly and measure diagnostics/API verbosity;
4. then replace their toy bodies with one real common-practice library operation and one practitioner-reviewed
   non-Western or cross-cultural operation; and
5. promote only the constructs both real operations use.

This is a candidate for the **source elaboration language**, not the temporal kernel. It supplies representation
ownership. K₃'s finite timeline, process IR, lineage, and explicit cross-stage passes remain separate.
