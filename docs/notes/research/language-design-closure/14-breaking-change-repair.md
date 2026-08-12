# Breaking-change repair

**Purpose:** choose the smallest coherent repair for the two flaws in the second proof review.

## Decision

Musa will not treat compiler-owned operations as function values. A call such as `transpose(P8, subject)` is a complete
operation. The new language rejects `transpose(P8)` because one argument is missing.

Ordinary Musa functions remain values. A musician who wants an octave transform as a value writes a function:

```musa
fn up_octave(music: Music) -> Music {
    transpose(P8, music)
}

let answer: Music -> Music = up_octave;
```

This keeps the delayed-canon use case. It removes only the shortcut that made a partly filled compiler operation look
like a source function.

The `map` and `filter` rules will also change. They may enter their private traversal state only after the function and
list are values. They may record a callback result only after that result is a value. These premises make the next step
unique and keep callbacks and resource charges in source order.

## Why this is the right boundary

Three designs could repair partial compiler calls.

1. **Make compiler operations curried values.** This preserves the old spelling, but adds a second kind of function
   value. It must store a compiler operation name, a partly filled argument vector, defaults, and the next argument
   position. Every safety and termination proof then needs cases for that private calling convention.
2. **Insert hidden wrappers when a function is expected.** This avoids a new value but makes elaboration depend on the
   expected type and on which arguments happen to be missing. Error messages and named or default arguments become
   harder to explain.
3. **Require complete calls and let users write functions.** This uses the function construct the language already has.
   The checker has one local rule: every compiler operation receives exactly its declared arguments.

The third design is smaller and easier to teach. It preserves the higher-order uses found in Musa's current examples:
their supplied arguments are fixed values, so named wrappers make the same complete calls. It does not preserve every
possible dynamic partial call. For example, a function that computes `transpose(interval)` from a run-time interval
would need a source closure or a changed interface. No musical case in this study needs that behavior.

Compiler operation signatures will use a distinct notation:

```text
transpose : op(Interval, Music) => Music
map_note_pitches : op(Pitch -> Pitch, Music) => Music
```

The `op` marker says that the name may appear only in a complete operation call. The arrow inside `Pitch -> Pitch` still
denotes an ordinary source function. This notation prevents the specification from claiming both that `transpose` has a
curried function type and that it cannot be partly applied.

## Concrete tests

The following cases decide the rule.

- `transpose(P8, subject)` checks as one complete operation.
- `transpose(P8)` is rejected with a missing-argument error and a suggestion to write a named wrapper.
- `map_note_pitches(raise, subject)` checks because the operation is complete and `raise` is an ordinary function value.
- `map(id, ((lambda(x:Nat) => x)(0)) :: [])` first reduces the list member. It cannot enter `map_state` early.
- `map_wait(id, [], [], (lambda(x:Nat) => x)(0))` first reduces its pending callback. It cannot record a non-value.

## Compatibility claim

The old claim was too broad. It required every detail of the current private evaluator to survive in the new language.
That is not a useful pre-release promise.

The repair has one narrower theorem and one source audit.

1. Current terms in which every compiler operation is fully applied keep their type and result.
2. Each partial compiler call in the current repository has a concrete rewrite to a named Musa function that makes one
   complete call.

The second point is an implementation audit, not a universal theorem. The future compiler should report a clear error
and show the wrapper shape when the supplied arguments are fixed. It need not accept the old shortcut. If two real
musical cases later need dynamically created function values, Musa should consider ordinary anonymous functions rather
than restore a special partial-operation value.

## Repository migration audit

I searched `examples/`, `stdlib/`, the governing and teaching language documents, and the compiler test suite for all
eight controlled operation names. The partial uses have only these shapes:

| Old shape | Named replacement |
| --- | --- |
| `transpose(P8)`, `transpose(P15)`, `transpose(P5)`, or `transpose(P1)` | `fn answer(m: Music) -> Music { transpose(fixed_interval, m) }` |
| `stretch(2)` | `fn broader(m: Music) -> Music { stretch(2, m) }` |
| bare `retrograde` | `fn backwards(m: Music) -> Music { retrograde(m) }` |
| `invert(c4)` | `fn mirror(m: Music) -> Music { invert(c4, m) }` |

These occur in the canon, standard-library, template, and higher-order music examples and their tests. Every supplied
argument is fixed at the definition or call site. I found no partial controlled operation that captures a run-time
value. The implementation prompt must repeat this audit before removing the old evaluator state, because the current
source may change before then.

## Formal counterexample check

The second review gave two terms with competing next steps. Under the repaired rules, each has one:

```text
map(id, ((lambda(x:Nat) => x)(0)) :: [])
```

The `map` entry rule requires a value list, so only the beta reduction inside the list may run.

```text
map_wait(id, [], [], (lambda(x:Nat) => x)(0))
```

The completion rule requires a value result, so only the beta reduction inside the pending expression may run. The same
argument applies to `filter` and `filter_wait`. Once a callback returns a value, the completion rule is the only next
step. Callbacks and their charges therefore remain in list order.

The formal reference now writes the same value premise on folds, complete operations, checked quotation, and
`map_note_pitches`. This does not change their meaning; it removes reliance on an unstated call-by-value convention.

## Gate

This note and the repaired metatheory have not received a new independent proof review. Tasks 7–8 remain blocked until
that review finds no fatal, high, or medium issue. The old reviews stay frozen and continue to describe the drafts they
examined.

## Scope

This repair does not reopen the five musical cases, finite temporal kernel, `Music` recipe, stage boundaries, module
design, package design, or audio process semantics. It changes the formal source calculus, proof outline, metatheory,
and later implementation prompts. The failed reviews remain unchanged beside the repair.
