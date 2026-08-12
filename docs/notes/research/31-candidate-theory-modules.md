# Candidate T₂ — theory modules, not a universal musical ontology

**Status: candidate source-language extension; governs nothing.** K₂ repairs the semantic host, but [28 §8]
(28-candidate-k2.md) leaves a practical question unanswered: how can a Musa package define its own musically meaningful
carriers if every base type is compiler-owned?

**Refined by the paper prototypes.** [36](36-theory-module-paper-prototypes.md) found that the first extension needs
neither user-generic nor user-recursive data. Monomorphic non-recursive nominal sums/products plus the existing
`Option`/`List` types express both probes. The broader polynomial grammar below remains an upper-bound candidate, not
the recommended first implementation.

The current elaboration calculus is disciplined: its musical base types are inert, every foreign δ-primitive is
first-order and arrow-free, and higher-order collection operations are separately proved structural. That design should
survive. Its long-term limitation is the **closed registry** of compiler-owned types such as `pitch`, `pc12`, `scale`,
`key`, `degree`, `row12`, and `roman`. A library can arrange values and functions, but cannot state that its melodic
intent, tuning position, phrase, or analytical evidence has a distinct representation hidden behind a signature.

Candidate T₂ adds only:

1. first-order nominal algebraic data;
2. abstract data members in static signatures; and
3. opaque or manifest realization of those members by structures.

This is the smallest source feature found so far that lets several musical theories coexist without variants of one
global `Pitch` or `MusicTheory` enum.

---

## 1. The extension

### 1.1 Polynomial data declarations

```text
data PhraseUnit {
    Svara(SvaraIntent),
    Gamaka(GamakaGesture),
    Bound(SvaraIntent, GamakaGesture),
}

data Claim P E {
    Claim(proposition:P, evidence:E, confidence:Option Ratio),
}
```

The field grammar is first-order polynomial:

```text
F ::= scalar | parameter | D Ā | F × F | F + F | Option F | List F | Vec n F
```

No field contains an arrow, a structure, syntax, mutable state, a signal, or an arbitrary user type constructor. A
recursive occurrence of the declared type is allowed only in a finite positive position. Constructors and exhaustive
matches are generated; structural folds are the only eliminators which recur.

Every declared type is nominal. Two declarations with the same constructors remain different types.

### 1.2 Abstract members

```text
signature MelodicPractice {
    data type Context;
    data type Intent;
    data type Gesture;
    data type Evidence;

    let admit : Context × Intent → Verdict Evidence;
    let realize : Context × Intent → Result Gesture RealizationError;
}
```

`data type T;` promises a finite canonical carrier but reveals no constructors. A structure supplies either a private
declaration or a manifest type:

```text
structure SomePractice : MelodicPractice {
    data type Context = ...;
    data type Intent = ...;
    data type Gesture = ...;
    data type Evidence = ...;

    let admit = ...;
    let realize = ...;
}
```

Outside the structure, clients can obtain and use `SomePractice.Intent` only through exported operations. Matching is by
member name and exact type. Unlisted values and constructors are private. A signature may deliberately expose a data
definition when interoperability is more important than abstraction.

### 1.3 Static templates

The existing static `template structure` supplies module-level abstraction:

```text
template structure Repertoire(
    P : MelodicPractice,
    context : P.Context,
) : AdmittedRepertoire {
    data type Admitted;
    let admit : P.Intent → Result Admitted P.Evidence = ...;
    let realize : Admitted → Result P.Gesture RealizationError = ...;
}
```

Each named instance gives its abstract result members a fresh nominal identity. This obtains the useful effect of a
family `Admitted(context)` without placing the value `context` in ordinary type conversion. Dynamic selection of a
context returns `Result`; it does not attempt runtime type generation.

The word **functor** is justified only in the static module sense: the template maps structures matching one signature
to structures matching another, and substitution respects member paths. It does not make every musical transformation a
category-theoretic functor.

## 2. Static judgments

Add module paths `p` and path-dependent abstract types `p.T` to the source static language. The principal judgments are:

```text
Σ ⊢ data D Ā = constructors ok
Σ ⊢ signature S ok
Σ ⊢ structure M : S ⇒ Σ'
Σ ⊢ make F(M̄,v̄) as N ⇒ Σ'
Σ ; Γ ⊢ e : A
```

`Σ` contains stable nominal identities for packages, declarations, structures, and abstract members. Structure checking
proceeds in four finite stages:

1. collect signature and structure headers;
2. assign nominal member identities and check the finite dependency graph is acyclic;
3. check data declarations and value bodies under those identities; and
4. seal the result to its ascribed signature.

Representative rules are:

```text
Σ(M) = S    data type T ∈ S
────────────────────────────── Abstract-Path
Σ ⊢ M.T : Data

Σ ⊢ R : Data    constructors C̄ form a finite polynomial over R
────────────────────────────────────────────────────────── Data-Def
Σ ⊢ data T = C̄ : Data

S requires data type T    M defines data T = R    Σ ⊢ R : Data
──────────────────────────────────────────────────────────── Match-Data
Σ ⊢ M.T realizes S.T

S requires let x:A    Σ;Γ_M ⊢ e:A
────────────────────────────────── Match-Value
Σ ⊢ let x:A=e realizes S.x
```

After sealing, `M.T` is definitionally equal to its representation only within `M`; clients compare it by nominal
identity. Static template bodies are checked under abstract paths such as `P.Intent`, not rechecked against hidden
representations.

There is no structural subtyping, implicit signature search, higher-kinded type member, first-class structure, recursive
module, or runtime unpacking.

## 3. Equality, canonical data, and serialization

User code does not define an arbitrary equality function for a data type. Equality of a transparent data value is
nominal type identity plus structural equality of its finite constructor tree. This gives a compiler-generated canonical
encoding:

```text
encode_D(Cᵢ(v₁,…,vₙ)) =
    (stable_type_id(D), constructor_ordinal(i), encode(v₁), …, encode(vₙ)).
```

For a sealed abstract member, the compiler retains the same private structural encoding while clients cannot inspect it.
The stable type identity is derived from locked package identity, qualified declaration path, and language/encoding
version. A generative template result additionally contains the already-existing stable instance identity—not a source
byte offset.

This rule deliberately refuses user-overloaded quotient equality. If two representations should denote one semantic
value, the package defines a separate normalized nominal type and a smart constructor:

```text
normalize : RawSpelling → Result CanonicalSpelling Error.
```

The quotient boundary is therefore an explicit function with testable laws. It cannot silently make timeline hashing
depend on an unproved user equality.

## 4. Operational semantics and metatheory impact

Data constructors are values when their fields are values. Match reduces by selecting the constructor branch and
substituting the fields. Fold reduces only on an exposed constructor and recurs on strict finite subtrees. These are the
polynomial-data cases already isolated in K₂; they preserve progress, preservation, deterministic evaluation, and strong
normalization by constructor-size induction.

Abstract types add no term reduction. Sealing erases representations from the client typing environment, not from the
runtime value. Static structure/template elaboration produces a finite monomorphic declaration graph. A future theorem
must prove:

```text
Σ ⊢ project ok ⇒ elaborate(project) = core
                   ∧ ∅ ⊢ core : A
                   ∧ project and core have the same result.
```

Acyclicity alone is not that theorem. It also needs exact syntax for path resolution, generative identity, signature
matching, sealing, and a substitution lemma for template parameters. Until that proof exists, this candidate does not
inherit K₂ core totality merely by intention.

## 5. What becomes natural

### 5.1 Algebra belongs to the theory which uses it

```text
signature Action {
    data type Point;
    data type Move;

    let identity : Move;
    let compose : Move × Move → Option Move;
    let act : Move × Point → Option Point;
}
```

A transpositional theory can instantiate this with pitches and intervals. A tuning system with only partial moves can
return `Option`; a theory with no useful transposition action does not implement the signature. The group/torsor laws
are package obligations, not consequences of merely matching these types.

For a total action, mapping `act(g,–)` over a finite list or multiset constructs transposition of a collection. That
extension falls out from structural traversal. A chord still needs a package definition of voicing, simultaneity, and
classification; generic parallel composition does not name one.

### 5.2 Keys become one optional context theory

```text
signature KeyedPractice {
    data type Pitch;
    data type Key;
    data type Degree;
    data type FunctionClaim;

    let locate : Key × Pitch → Option Degree;
    let analyze : Key × Phrase → List (Claim FunctionClaim Evidence);
}
```

This makes scale degree and analysis explicitly contextual. It does not identify harmonic function with tonic
translation and does not ask an unkeyed repertoire for a dummy `Key`.

### 5.3 Phrase- and gesture-primary practices need no pitch surrogate

A rāga, maqām, drum-language, dance/drum interaction, or cue-structure package may expose phrase, pathway, gesture,
stroke, movement, or evidence types without supplying pitch-class, scale, chord, key, or meter members. Its finite
values can be placed on timelines when exact placement is one desired realization; the package's definition is not
forced to be the timeline.

### 5.4 Instruments are also structures

An instrument contract can expose nominal gesture/control data and hide its process graph:

```text
signature Instrument {
    data type Gesture;
    data type Control;
    data type Error;

    let prepare : Timeline Performed Gesture × List Control × Seed
                → Result PreparedInstrument Error;
}
```

The prepared runtime value remains opaque and stage-restricted. A structure may implement it with synthesis, samples,
physical modelling, an adapter, or a combination. Score part, instrument declaration, runtime instance, and mixer track
remain different nominal identities.

## 6. Translation between theories

There is no implicit coercion between abstract types, even if two private representations happen to be identical. A
package importing two practices writes a named pass:

```text
translate : A.Intent → Result (Derived A.Intent B.Intent) TranslationError.
```

The `Derived` value carries finite lineage and explicit losses. A deterministic translation is an ordinary function. A
many-valued transcription returns a finite alternative/evidence value. An observational estimator returns claims or a
distribution. These distinct meanings should not be collapsed into a generic `link` relation.

This is also the constructive residue of the IUT analogy. Separate structures have separate nominal carriers and
equalities; a translation cannot identify them by definitional equality. But the source language needs no first-class
`world` or theta-link. **A sealed structure is the world when one is actually needed; a named typed pass is the link.**

## 7. Pressure tests

| Case | Can T₂ state the native distinction? | Forced field or structure |
| --- | --- | --- |
| Common-practice chord construction | yes: package pitches, intervals, voicing, context, analysis | none; chord laws still supplied |
| Karnatak phrase/gamaka context | yes: phrase and bound gesture constructors or private carrier | no scale/key requirement |
| Arabic maqām pathway and intonation | yes: sayr, jins, phrase, intonation context | no 12-TET quotient |
| Gamelan ensemble-relative position | yes: ensemble-specific hidden position and pairing | no global frequency class |
| Hindustani tāl | yes: cycle vocabulary/gesture plus optional timeline realization | no universal pulse layer |
| Bomba dance/drum exchange | finite event types yes; live relation still belongs to `Process` | no score or meter required |
| Instrument contract | yes: abstract gesture/control/error and hidden process | no DSP node leaks |

The cases do not establish cultural adequacy. They establish only that the type system does not demand the wrong global
carrier before package authors begin.

## 8. Costs and falsifiers

The extension is not free:

- generative identities and stable serialization must survive separate packages and incremental builds;
- diagnostics for path-dependent types and signature mismatches must remain readable;
- module elaboration needs a real preservation proof;
- unrestricted public ADTs can fossilize a poor taxonomy, so packages need ordinary abstraction discipline; and
- a source package still cannot prove deep musical laws merely by declaring operation types.

Reject or shrink T₂ if either falsifier holds:

1. two non-Western package prototypes can be written clearly with existing types and structures, without textly tags,
   accidental representation coupling, or compiler-owned additions; or
2. nominal data and abstract members require first-class dependent packaging or runtime type equality to support the
   intended callers.

The first would show that new syntax has no caller. The second would show that the supposedly small extension is not
small.

## 9. Recommendation

Do not add all of T₂ directly to the implementation stack yet. First write two executable paper packages against this
syntax:

1. refactor the existing common-practice theory into a sealed `WesternTonal` structure; and
2. model one phrase/gesture-primary practice without reusing its pitch, scale, key, chord, or meter types.

Use them to fix the data grammar, path identity, constructor visibility, and translation API. Then specify the source
module elaboration and prove preservation before scheduling implementation prompts.

If those prototypes remain concise, abstract nominal data—not dependent types, CBPV, worlds, or a universal music
object—is the one major language change the current research actually supports.
