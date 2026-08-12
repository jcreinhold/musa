# Declaration templates and structures

Value functions construct values, including contextual `music`. Declaration templates construct declarations before
score contexts are built. Keeping those stages separate permits reusable pieces and voices without making source syntax
or identity-bearing structures first-class.

The source-library boundary is deliberately smaller than the structure system specified below, and the two are not
alternatives: §4's signatures and structures are a checking-time abstraction layer, not a second import mechanism.

A **package** is a directory containing `musa.toml` and a source root. Its root file `lib.musa` declares its children
with `mod`, a directory module declares its own in `mod.musa`, and module paths nest to any depth. Resolution follows
those declarations and never scans the directory: a `.musa` file under the source root that no `mod` reaches is rejected
as declared nowhere, and a `mod` naming no file is rejected as missing. The bundled standard library is one such
package, embedded at build time so that its `musa-stdlib:/std/…` URIs are readable without filesystem access, with the
embedding generated from the `mod` traversal rather than transcribed beside it.

`import "path.musa";` imports a local `library`; `import std::list;` and `import std::tonal::harmony;` import bundled
modules. Both run through the same parser, checker, evaluator, cycle detection, and name-collision rules, and both bind
into the flat value namespace — see `01-surface.md` for why that differs from §4's `Structure.member` rule, and
`docs/notes/research/60-language-decision-record.md` for the correction that introduced packages. There is no prelude,
environment search, registry, or dependency solver.

## 1. Declaration-template judgment

For `κ ∈ {library, piece, part, voice, performance, instrument, mix, structure}`:

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
core, substituted capture-avoidantly into typed declaration holes, and then ordinary declarations are checked. Expansion
precedes context-track construction and contextual-music instantiation. A template cannot receive or return a
declaration as a value, enumerate declarations, inspect source text, or emit a template dynamically.

The static dependency graph over `make`, imports, and functor applications must be finite and acyclic. A cycle is a
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
instance sites with equal arguments remain distinct. The explicit `as` name is the source address but is not hashed
identity. Moving a site may change `SiteId`; an editor move operation records an identity-preserving source edit when it
can prove the move. Generated Origin paths contain both the definition path and instance identity.

**Identity laws.** Recompiling unchanged build closure yields the same IDs. No two different declaration paths within
one accepted project yield the same ID. Alpha-renaming a private template parameter or the explicit instance name does
not change IDs when the source site remains the same; the latter changes the public address and reference-resolution
result, not identity.

## 3. Parameterized structures

The exact surface forms are those in `01-surface.md`:

```musa
template voice answer(subject: Music, transform: Music -> Music) {
    use transform(subject);
}

template piece study(k: Key, subject: Music) "Study" {
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
bearing voice before those structural contexts are computed. A `music` value may not contain those declarations. This is
the semantic reason both constructs exist.

Score, performance, instrument, and mix templates may share value parameters but still expand to separate declarations.
A piece template does not package mutable studio state into a piece value.

## 4. Signatures and structures

Structures are static named collections of values. A signature says what a structure must provide; a structure provides
it; a `template structure` is a functor from structures to a structure. The implemented spelling is:

```musa
signature CanonMaterial {
    let subject: Music;
    let answer: Music -> Music;
}

structure FifthMaterial: CanonMaterial {
    let subject: Music = theme();
    let answer: Music -> Music = transpose(P5);
}

template structure DelayedCanon(C: CanonMaterial, gap: Duration): CanonMaterial {
    let subject: Music = canon(C.subject, C.answer, gap);
    let answer: Music -> Music = C.answer;
}

make DelayedCanon(FifthMaterial, 1/2) as FifthCanon;
```

A signature member is a `let` without its definition: a name and the type. Values, functions, and `music` bindings are
all `let` members, because a function is a value of arrow type. Member kinds that name an identity-bearing declaration —
`instrument`, `profile`, `patch` — are not implemented; when they arrive they will be spelled by their own keyword in
the same position, and matching for them will be nominal rather than transparent.

Matching is transparent and by name: every member the signature lists must be defined by the structure with exactly that
type, and a missing or mistyped member is reported at both the signature's member and the structure. A member the
signature does not list is private — usable by the structure's own definitions and nameable by nothing outside it, which
is what sealing means here.

Paths are `Structure.member`, and inside a structure a sibling member is read by its bare name. Unqualified lookup never
searches remote packages or every imported module.

Functor application checks the argument's signature against the parameter's, expands once at the named site, seals the
result to the result signature, and assigns a stable generative identity derived from the functor, the argument
structures' identities, and the instance site — never the argument *expressions*, so two sites given equal arguments
stay two structures, and never a span, so editing the text above a site does not change what it made. Instance sites are
ordered by what they consume rather than by where they are written, and a cycle among them is refused: instantiation
happens once, before anything is evaluated, so a functor consuming what it produces has no base case.

Expansion is *binding*, not rewriting: the body is checked once per instance in a scope where `C` names the structure
the site passed, so no syntax is copied, no span moves, and no name can be captured. A functor sees exactly its
parameter's signature, whatever else the structure behind it happens to define.

There is no `structure` value type, first-class structure unpacking, recursive structure, implicit functor application,
or Rust `Functor` trait — and the reason is the same one in every case. A structure is a *name for a group of
declarations*: it holds no state, is never a value, and has stopped existing by the time anything is evaluated. A Rust
trait would model a functor as a value with methods and a public structure object would model a structure as a thing
that exists at run time; neither is true, and either would invite the applicative sharing this design refuses. What a
functor *is* is a second checking pass over one body, so what it is written as is a scope — see
`crates/musa-compiler/src/module.rs`, where the whole of it is one `NameScope` and a flattening into the namespace the
core already has.

## 5. Resolution and compilation order

For a project build closure:

1. resolve immutable local and package modules and detect static cycles;
2. collect signatures and template headers;
3. type-check value definitions and template bodies against headers;
4. evaluate template arguments and expand `make` sites in canonical source order;
5. seal structures and assign declaration identities;
6. build score context tracks;
7. instantiate contextual `music`, close the kernel term, and project the score;
8. independently resolve performance, instrument, and mix declarations, then prepare sound.

Earlier stages cannot query results from later stages. In particular, a template cannot branch on an analysis result
computed from the piece it is generating; authors instead call a finite value-level analysis on explicit input data or
write an assertion after construction.

## 6. Rejections

The following are static errors: a parameterized declaration without `template`; `make` without `as`; a template cycle;
first-class `piece`, `voice`, `structure`, or source-syntax use; a structure member that fails its signature; two
generated declarations with the same public address; and a structural declaration embedded in `music`.

Prompts 103–104 implement this stage. Prompt 124 measures expansion and caching. Prompt 144 verifies that identity and
Origin remain stable through the migration.
