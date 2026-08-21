# `Unit` has no surface value

**Status: research. These notes do not set Musa's rules.**

Prompt 127d added machine values and, with them, two ports typed `Unit`: `count` consumes nothing, and `drop` produces
nothing. Implementing it surfaced a question the language had never been asked. `Unit` is in the offered type
vocabulary, documented as the type with exactly one value, and the surface has no way to write that value — `()` fails
to parse with *expected an expression, found `)`*. Nothing was broken by it: `drop : Machine<K, A, Unit>` types,
projects, and is covered by `crates/musa-compiler/tests/suite/machine_laws.rs`. But the state was implicit, and an
implicit state is one a later prompt closes by accident.

This note records the decision. **`Unit` stays in the offered vocabulary and stays valueless in the surface.** The rest
of the file is the argument, the two alternatives refused, and what was built to keep the decision from drifting.

## 1. What `Unit` is doing in this language

`Unit` earns its place as a **port type**, not as a value type a composer holds.

A machine's port type says what flows there at each step. `Unit` says nothing flows. That is a thing a machine can mean
and two registered units already mean it: `count`'s input and `drop`'s output. Nothing about that role requires a
composer to ever hold the value — machines are wired to each other, and a port is inhabited by the machine layer at each
step, never by an expression at a call site.

That role does, however, require the *word*, and not because a diagnostic happens to print it.
`drop : Machine<K,A,Unit>` is fixed by `docs/rules/across-stages/03-machine-calculus.md` §2 — a *governing* page, above
the candidate language specification in the precedence ladder — and `drop` is one of the seven structural forms the
calculus requires. So a composer who writes `connect(source, drop)` owns a value whose type contains `Unit`, and the
question is not whether a string is printed but whether they may declare what they hold. They may:
`let m: Machine<AudioFrameStep, Ratio, Unit> = connect(machine(primitive("scale", 1, 3/2)), drop)` compiles, and so does
a function taking and returning that type.

Drop the name from the vocabulary and that stops compiling. The type does not go away — nothing about `drop` changes —
it just becomes unspellable, so the value can be held but never annotated, never passed to a declared function, never
named in a library signature. That is the hole the existing law
`every_offered_type_name_is_read_and_written_the_same_way` (`crates/musa-compiler/src/core/mod.rs`) exists to prevent.
So `Unit` is readable, printable, and offered — and inhabited by nothing written.

There is already a precedent for a nameable type with no value: `AudioFrameStep`. `lower_type` says of it that it "is a
type so that `K` unifies like any other index, but it is not a *value* type: nothing inhabits it". `Unit` is not the
same case — it *is* a value type in the calculus, and `02-core-calculus.md` §1.1 counts `unit` as storable data — but
the shape of the answer is the same: a type can be in the language for what it lets other types say, without a literal.

## 2. Refused: add a unit literal

The obvious repair is a literal, spelled `()` or otherwise. It was refused on three grounds.

**Nothing consumes it.** No builtin takes a `Unit` argument, no eliminator matches on it, no musical operation returns
one. A literal would add a value a composer can bind and discard and do nothing else with. `02-core-calculus.md` §1
argues the exclusion of `int` by the rule *removing it makes nothing impossible*; run forward, that rule asks what
adding a unit literal would make possible, and the answer is one thing only — §3 below — which is better handled where
it arises.

**The spelling is not free.** The surface writes its nullary constructors as words: `true`, `false`, `none`, `ok`,
`err`. The consistent spelling for a unit value would be a word, and the two candidate words are taken. `unit` is in
`RESPELLED_TYPES`, mapping to `Unit`, so a composer who writes it gets the capital-letter rule from
`docs/rules/style-guide.md`; making it also a value would make one word mean a type in one position and a value in
another, which is the one thing the capitalization rule exists to prevent. `nothing` reads as `none`. That leaves `()`,
which the parser *can* take unambiguously — `paren_or_product_expr` is only reached where an expression starts, and
`f()` is an argument list read by the postfix loop in `expr`, so an empty pair of parentheses at an atom position is
free — but which would be the surface's first punctuation-only value literal, and would be spelled `()` while its type
is spelled `Unit`, a pair no other type in the language has.

**It is not free of the rules, either.** `02-core-calculus.md` §5.3's canonical-forms proof states the present position
in as many words: "`unit` is presently uninhabited by surface literals, so its closed-value case is vacuous until a unit
constructor is introduced." Introducing one means amending that proof, §1's term grammar, and `01-surface.md`'s
expression grammar, plus the lexer, the parser, the formatter, the tree-sitter grammar and its corpus, and the book.
That is the ordinary amendment path for a candidate specification (`docs/rules/README.md`) and it is available — but it
is a lot of machinery for a value with no consumer.

## 3. The one thing the absence costs, and the prompt that will pay it

A registered primitive whose configuration is `Unit` — a unit with no configuration at all — cannot be instantiated,
because `primitive("name", 1, …)` has nothing to write in the third position. `crates/musa-compiler/src/machine.rs`
already stated the constraint as prose beside the registry: "Every configuration is a value this language can write,
since a unit whose configuration could not be spelled could not be instantiated."

There is a distinction underneath that, and it is a real one. **A port type says what flows. A configuration type says
what is written.** `Unit` can say *nothing flows*, and it cannot say *nothing is written*, because nothing-written is
not something a composer writes. On the five-unit reference family the constraint costs nothing, and it is now checked
rather than promised (§5).

**It will not stay free.** Six processors in the current studio catalogue declare no parameters at all —
`crates/musa-dsp/src/spec.rs`'s `Noise`, `Passthrough`, `Mixer`, `Splitter`, `MonoToStereo`, and `StereoToMono` — and
prompt 152 is *Register each current DSP unit as a registered primitive*, naming the mixer among them: "a mixer is a
primitive from a tuple of frames to one frame". Three of the six are wiring that the structural forms already cover
(`copy` is `Splitter`; the channel adapters are adapters), and `Passthrough` is `identity`. Two are not. `Noise` and a
plain summing `Mixer` are registered units with genuinely nothing to configure.

Both may still acquire an honest configuration rather than an empty one. `Noise` is documented as "xorshift; fixed seed
per node", and an *explicit* `Nat` seed is arguably the better design in a language whose machine identity is exact — a
hidden fixed seed is a value the machine's digest cannot see. A mixer may take per-input gains, which is what the
reference `MIX` already does. So the collision is real but not yet forced, and the choice between "declare the smallest
honest configuration" and "introduce the literal" is a registry-design question best answered with the real units in
hand.

What this note fixes is that the question cannot be reached by accident: the `const` assertion in §5 makes an empty
configuration a build error at the moment prompt 150 or 152 writes one, with the reason in the panic message.

## 4. Refused: remove `Unit` from the offered vocabulary

The other alternative was to stop offering `Unit` — to let it exist only as a type the compiler produces, the way
`AudioFrameStep` is looked up separately from `BASE_TYPES`.

This is worse than the status quo, and for the reason §1 gives. The tempting reply is that if the only problem is that
the compiler *prints* `Unit`, the compiler should stop printing it — but there is nothing to stop. `drop`'s output type
is `Unit` because a governing page says so, and the type appears in composer programs whether or not any diagnostic
mentions it. A composer who can hold `connect(source, drop)` and cannot declare it is worse off than one who can write
the type but never the value: the first has a value they cannot pass to a declared function, the second has a type with
a clear meaning — *nothing flows here* — and no need for a value. Removing the name would also break the round-trip law,
or force a second, quieter vocabulary beside `BASE_TYPES` for words the compiler prints and the composer may not write,
which is three vocabularies again.

The only way to remove `Unit` from the surface honestly is to change what `drop` *is* in
`docs/rules/across-stages/03-machine-calculus.md` §2, under that page's amendment procedure — and there is no candidate
replacement. `drop` produces nothing, and nothing is what `Unit` names.

`AudioFrameStep` is not a counterexample: it is position-restricted to the first argument of `Machine<…>` and
`Primitive<…>`, checked where those are read, and it is not a value type at all. `Unit` is a value type in the calculus
and can stand anywhere a type can.

## 5. What was built

The decision is only worth recording if a later prompt cannot undo it without noticing.

- **`crates/musa-compiler/src/core/mod.rs`** gains `unit_is_the_one_offered_type_no_written_expression_produces`. It
  compiles one literal per type that has one — the table is source text, checked by the parser and the checker rather
  than asserted — collects every base type some δ-builtin answers with at any depth of its result shape, and asserts
  that the offered vocabulary minus that union is exactly `["Unit"]`. A domain reachable only inside an `Option` still
  counts as produced: `pitch_frame` is the only way to make a `Frame` and it reports whether it made one, so a `Frame`
  is a value a `match` arm holds even though no closed expression is annotated with it. `Unit` is produced at no depth
  of anything.
- **`crates/musa-compiler/src/machine.rs`** gains `PortShape::is_writable` and a build-time `const` assertion that every
  registered configuration is writable. The prose promise beside `COUNT` is now a compile error. A product is writable
  at two members or more, because `(e)` is a parenthesized expression and the surface's product literal starts at the
  comma.
- **`crates/musa-language/src/types.rs`** re-words the one line an editor shows beside `Unit`. It read "the type with
  exactly one value", which invites a composer to go looking for the literal; it now says the type carries nothing and
  that no expression writes it.

## 6. What would reverse this, and which half it would reverse

The two halves of this decision are not equally settled, and they should not be read as one.

**`Unit` stays offered** is settled, and not by preference. It follows from `drop`'s governing type, and reversing it
means amending `03-machine-calculus.md` §2 with a replacement for a form that produces nothing.

**`Unit` stays without a literal** is a decision made on the units this build registers, and §3 names the work that will
test it. Prompt 150 registers the reference family; prompt 152 migrates the studio catalogue, where `Noise` and a plain
summing `Mixer` are registered units with nothing to configure. If either declares an empty configuration rather than an
honest one, the literal has its first consumer and §2's first ground — that nothing consumes a unit value — becomes
false. A builtin or eliminator taking or returning `Unit` would do the same.

That is the reversal `02-core-calculus.md` §5.3 already leaves room for. When it comes, this is what changes: the law in
§5 stops expecting `["Unit"]`, the `const` assertion in `machine.rs` loses its `Unit` arm, and the amendment path in §2
runs — §5.3's proof, §1's term grammar, `01-surface.md`, then lexer, parser, formatter, tree-sitter, and book.
