# Declaration templates and static modules

Value functions construct values, including contextual `music`. Declaration templates construct declarations before
score contexts are built. Keeping those stages separate permits reusable pieces and voices without making source syntax
or identity-bearing structures first-class.

The implemented source-library boundary is deliberately smaller than the static module system specified below.
`use "path.musa";` imports a local `library`; `use std::core;`, `use std::list;`, and `use std::option;` import the
version-matched bundled sources at stable virtual URIs. Both paths run through the same parser, checker, evaluator,
cycle detection, and flat-name collision rules. There is no prelude, environment search, registry, or dependency
solver. The signatures/modules in §4 are a later static abstraction layer, not a second runtime import mechanism.

## 1. Declaration-template judgment

For `κ ∈ {library, piece, part, voice, performance, instrument, mix, module}`:

```text
Σ; Γ ⊢ D : declaration κ
Σ; Γ,x₁:τ₁,…,xₙ:τₙ ⊢ D : declaration κ
──────────────────────────────────────────────────── Template
Σ; Γ ⊢ template κ N(x₁:τ₁,…,xₙ:τₙ) D
        : (τ₁×…×τₙ) ⇒declaration κ

Σ ⊢ N : (τ₁×…×τₙ) ⇒declaration κ    Σ ⊢ eᵢ ⇓ vᵢ : τᵢ
──────────────────────────────────────────────────────────── Make
Σ ⊢ make N(e₁,…,eₙ) as I ⇓decl instantiate(N,v̄,I)
```

`⇒declaration` belongs only to the static judgment; it is not the value arrow. Arguments are evaluated by the total
core, substituted capture-avoidantly into typed declaration holes, and then ordinary declarations are checked.
Expansion precedes context-track construction and contextual-music instantiation. A template cannot receive or return a
declaration as a value, enumerate declarations, inspect source text, or emit a template dynamically.

The static dependency graph over `make`, imports, and module applications must be finite and acyclic. A cycle is a
located static diagnostic even if value evaluation would never reach it.

## 2. Stable generative identity

Every `make ... as I;` has a stable source identity `SiteId`. The semantic identity key is:

```text
GeneratedKey = (project-namespace, template-definition-id, SiteId, declaration-path-inside-template)
GeneratedId  = versioned_digest(GeneratedKey)
```

The digest algorithm and canonical encodings are versioned. The compiler retains keys while assigning IDs and rejects
the astronomically unlikely digest collision instead of identifying two declarations. Argument values do not determine
identity: editing `study_in_g`'s musical key changes its content without turning it into an unrelated piece. Two
instance sites with equal
arguments remain distinct. The explicit `as` name is the source address but is not hashed identity. Moving a site may
change `SiteId`; an editor move operation records an identity-preserving source edit when it can prove the move.
Generated Origin paths contain both the definition path and instance identity.

**Identity laws.** Recompiling unchanged build closure yields the same IDs. No two different declaration paths within
one accepted project yield the same ID. Alpha-renaming a private template parameter or the explicit instance name does
not change IDs when the source site remains the same; the latter changes the public address and reference-resolution
result, not identity.

## 3. Parameterized structures

The exact surface forms are those in `01-surface.md`:

```musa
template voice answer(subject: music, transform: music -> music) {
    use transform(subject);
}

template piece study(k: key, subject: music) "Study" {
    key k;
    score {
        part piano {
            voice right { use subject; }
            make answer(subject, transpose(P8)) as follower;
        }
    }
}

make study(key g major, theme()) as study_in_g;
```

An instance is placed by the kind it makes, because a document is still one piece or one library: a piece instance
stands at the file root and is that file's piece, and a voice instance stands among a part's voices.

A template voice may contain key, meter, tempo, and clef declarations because expansion produces an ordinary identity-
bearing voice before those structural contexts are computed. A `music` value may not contain those declarations. This
is the semantic reason both constructs exist.

Score, performance, instrument, and mix templates may share value parameters but still expand to separate declarations.
A piece template does not package mutable studio state into a piece value.

## 4. Module signatures

Modules are static named collections of types, values, and declarations. The candidate syntax is:

```musa
signature ChamberSound {
    instrument lead: note_instrument;
    profile lyrical: profile[note_instrument];
}

module DryStrings : ChamberSound {
    instrument lead from "assets/violin.sfz" conforms note_instrument;
    performance {
        profile lyrical for note_instrument {
            dynamic p -> expression 0.30;
            dynamic f -> expression 0.85;
        }
    }
}
```

A signature member specifies its namespace, name, and type/signature. Matching is transparent for value types and
nominal for identity-bearing declarations. Extra private members are hidden after sealing. Paths are `Module.member`;
unqualified lookup never searches remote packages or every imported module.

Module parameters use a genuine static functor. This example consumes a related bundle that direct value parameters
cannot name as one unit:

```musa
signature CanonMaterial {
    value subject: music;
    value answer: music -> music;
}

module FifthMaterial : CanonMaterial {
    let subject: music = theme();
    let answer: music -> music = transpose(P5);
}

template module DelayedCanon(C: CanonMaterial, gap: duration) : CanonMaterial {
    let subject: music = canon(C.subject, C.answer, gap);
    let answer: music -> music = C.answer;
}

make DelayedCanon(FifthMaterial, 1/2) as FifthCanon;
```

Functor application checks the argument signature, expands once at the named site, seals the result to the result
signature, and assigns stable generative identities. It does not construct a runtime module closure. There is no
`module` value type, first-class module unpacking, recursive module, implicit functor application, or Rust `Functor`
trait.

## 5. Resolution and compilation order

For a project build closure:

1. resolve immutable local/package modules and detect static cycles;
2. collect signatures and template headers;
3. type-check value definitions and template bodies against headers;
4. evaluate template arguments and expand `make` sites in canonical source order;
5. seal modules and assign declaration identities;
6. build score context tracks;
7. instantiate contextual `music`, close the kernel term, and project the score;
8. independently resolve performance, instrument, and mix declarations, then prepare sound.

Earlier stages cannot query results from later stages. In particular, a template cannot branch on an analysis result
computed from the piece it is generating; authors instead call a finite value-level analysis on explicit input data or
write an assertion after construction.

## 6. Rejections

The following are static errors: a parameterized declaration without `template`; `make` without `as`; a template cycle;
first-class `piece`, `voice`, `module`, or source-syntax use; a module member that fails its signature; two generated
declarations with the same public address; and a structural declaration embedded in `music`.

Prompts 103–104 implement this stage. Prompt 118 measures expansion and caching. Prompt 137 verifies that identity and
Origin remain stable through the migration.
