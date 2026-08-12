# Course correction: familiar syntax with inferred types

## Purpose

This note resets the source-language candidate around one goal:

> A musician should be able to read a function as written, while the compiler infers its type whenever the code already
> determines that type.

The syntax should look like Rust or Python. The type inference should work like the well-understood core of OCaml and
Haskell. Those are separate choices. Musa does not need ML syntax to use ML-style inference.

This note starts a new design target. Files 00–18 remain the record of the earlier target and its failed proof gate. This
note governs nothing.

## Start with ordinary code

The common case should look like this:

```musa
fn twice(f, value) {
    f(f(value))
}

let up_octave = |music| transpose(octave, music);
```

The compiler infers:

```text
twice      : (('a -> 'a), 'a) -> 'a
up_octave  : Music -> Music
```

`'a` means any type. The type of `twice` says that its first argument is a function from some type to the same type and
its second argument has that type. No annotation in the source is needed to establish this.

The syntax carries no hidden call rule:

- `f(a, b)` calls `f` once with both arguments;
- `|x| body` creates a function explicitly;
- a missing argument is an error; and
- a user function, imported function, and compiler-backed wrapper are called in the same way.

## The key decisions

### Use Hindley–Milner inference

Hindley–Milner inference assigns the most general type supported by a pure program. “Most general” means that any other
valid type for the same term comes from replacing its type variables with more specific types.

For example:

```musa
fn identity(value) {
    value
}
```

has the inferred type `'a -> 'a`. Calling it with a pitch gives `Pitch`; calling it with a chord gives `Chord`. The
function is checked once, not copied once per type.

Musa will use rank-1 polymorphism: a `let`-bound value may work at many types, but a function argument cannot demand a
value that is itself polymorphic. This boundary preserves principal types and a small, decidable inference algorithm.

### Keep the surface Rust-like

The surface uses:

- `fn`, `let`, braces, commas, and semicolons;
- `|x| expression` for an anonymous function;
- `data` declarations with named constructors;
- Rust-like `match` expressions; and
- nominal records for named configuration data.

Parameter and result annotations are optional on ordinary functions:

```musa
fn resolves(role) {
    match role {
        HarmonicRole::Dominant => true,
        _ => false,
    }
}
```

The constructor pattern fixes `role` as `HarmonicRole`, and both branches fix the result as `Bool`. The inferred type is
`HarmonicRole -> Bool`.

Annotations remain useful at deliberate boundaries:

- an abstract module signature;
- a compiler primitive declaration;
- a public interface when the author wants to promise a narrower type; or
- an ambiguous error that the programmer chooses to clarify.

They are not required on every local binding or function parameter.

### Make calls complete and closures explicit

A function call supplies every argument. Musa does not treat a missing argument as an implicit request to build a
closure.

Write the closure:

```musa
let up_octave = |music| transpose(octave, music);
```

This is longer than `transpose(octave)`, but it says exactly what value is captured and what argument remains. A reader
does not need the declaration's parameter list to understand the expression.

Several written parameters elaborate to one product argument. In outline:

```text
fn f(x, y) { body }    becomes    let f = |pair| match pair { (x, y) => body }
f(a, b)                becomes    f((a, b))
```

The implementation may use a private multi-argument form rather than construct a literal pair. The public meaning is
the same: the call is complete, and arguments evaluate once from left to right.

### Remove named and default function arguments

Named and default arguments caused the failed proof because their meaning depended on which slots a call filled and
when a closure captured omitted defaults. They also make an alias behave differently from the function's declared name.

Configuration belongs in data:

```musa
let options = NoteOptions {
    articulation: None,
    dynamic: Some(mf),
};

note_with(options, c4, quarter)
```

A library may offer a short common-case wrapper:

```musa
fn note(pitch, duration) {
    note_with(default_note_options, pitch, duration)
}
```

The record fields remain named and visible. Defaults are ordinary values. Function application keeps one meaning.

### Let `Option`, `List`, and `Result` infer their missing parts

Constructors have polymorphic types:

```text
None : Option<'a>
[]   : List<'a>
Ok   : 'a -> Result<'a, 'e>
Err  : 'e -> Result<'a, 'e>
```

The old candidate required an expected type for these forms. The new checker creates fresh type variables and lets use
sites constrain them. A non-recursive `let` may generalize variables that remain free.

For example:

```musa
let result = Ok(3);
```

infers `Result<Nat, 'error>`. Later use may determine `'error`; an otherwise unconstrained `let` may remain polymorphic.

### Keep inference predictable

The first design excludes features that weaken principal types or make inference depend on distant choices:

- no subtyping;
- no implicit numeric conversions;
- no overloading or type classes;
- no higher-rank polymorphism;
- no polymorphic recursion;
- no dependent or refinement types; and
- no general recursion.

Each literal form has one type. Musical conversions use named functions. User data may have type parameters but is
non-recursive in the first version. Built-in `List` supplies the common recursive container, and finite folds supply its
total eliminator.

Because the language is pure, it needs no ML value restriction. Every non-recursive `let` may be generalized after its
right-hand side is inferred.

## Compiler operations do not need special partial values

A compiler primitive remains a saturated operation with a declared type. The source environment exposes an ordinary
function around it:

```musa
fn transpose(interval, music) {
    primitive::transpose(interval, music)
}
```

The wrapper is an ordinary inferred function. Users may pass it as a value. A specialized function is an explicit
closure:

```musa
let down_fifth = |music| transpose(down_fifth_interval, music);
```

The evaluator therefore needs closures and saturated primitives, but no `BuiltinValue` that stores a partly filled
argument vector.

## The formal core

The core needs only these type forms:

```text
type ::= variable
       | Unit | Bool | Nat | Ratio | Text
       | type_constructor(type, ...)
       | product(type, ...)
       | type -> type

scheme ::= forall variable, ... . type
```

`type_constructor` covers `Option`, `List`, `Result`, compiler bridge types, and build-local nominal data types.

The inference environment maps each name to a scheme. Inference follows five standard steps:

1. Replace a used scheme's bound variables with fresh variables.
2. Infer subexpressions from left to right.
3. Unify types that the syntax requires to be equal.
4. At a non-recursive `let`, generalize variables not fixed by the surrounding environment.
5. At an annotation or module signature, check that the inferred type is at least as general as the promised type.

“Unify” means solve equations such as `'a = Nat` or `'f = Pitch -> Pitch`. Constructor names and build-local nominal
types unify only with themselves.

Modules do not require persistent type identity. One resolved build assigns a fresh identity to each abstract type. An
unsealed module may expose its inferred type schemes. Sealing checks those schemes against an explicit signature and
hides private constructors.

## What changes from the failed candidate

| Failed candidate | New target |
| --- | --- |
| Every function parameter and result is annotated | Infer ordinary functions; annotate chosen boundaries |
| Monomorphic user definitions | Rank-1 `let` polymorphism |
| Bidirectional rules need expected types for `None`, `[]`, `Ok`, and `Err` | Fresh variables plus unification infer them |
| Multi-argument functions curry implicitly | Complete product calls; closures are explicit |
| Named and default arguments alter call elaboration | Named records hold configuration; calls have one rule |
| Compatibility with selected old partial calls is a theorem | Partial calls are rejected with migration diagnostics |
| No anonymous-function surface | Rust-like `|x| expression` closures |

This is a new language target, not another repair to the old compatibility theorem.

## Proof obligations

The proof should address the new language directly. It should not compare closure layouts or default capture with the
current compiler.

Prove:

1. inference finishes;
2. each inferred term has a principal type;
3. elaboration preserves the inferred type;
4. substitution, preservation, progress, and deterministic evaluation hold;
5. every source term terminates;
6. exhaustive matching and sealing remain sound;
7. closing `Music` returns a finite typed temporal term or an explicit error; and
8. typed stage passes compose through exact derivation records.

The termination proof may erase type schemes after checking and reuse the logical relation for the simply typed total
core. The inference proof should use the standard unification argument. Neither proof needs a theorem that old partial
calls keep their behavior.

Migration is an engineering audit:

- find every annotation that inference can remove;
- find every partial, named, or default call;
- give each rejected form a precise diagnostic and mechanical rewrite; and
- keep executable fixtures for the rewrites.

## Bounded restart

The restart has six deliverables:

1. Rewrite the five paper programs in Rust-like inferred syntax and count the annotations that remain.
2. Specify the surface grammar, type schemes, unification, generalization, modules, and errors.
3. Specify elaboration into the existing total core and saturated compiler operations.
4. Prove inference, type safety, termination, `Music` closure, and stage composition.
5. Freeze the result for one hostile proof review, allow one repair, then run one final review.
6. Promote only after the final review reports no fatal, High, or Medium issue.

Do not change the governing language documents, code map, or implementation prompts before step 6. Do not implement
the compiler during this restart.

## Recommendation

Adopt this as the sole active source-language candidate. Do not repair the partial-call translation again. The next
piece of evidence should be rewritten musical programs with inferred types, followed by the inference rules those
programs actually need.
