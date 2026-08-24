# What is implemented today

This page distinguishes proved design targets from current Rust code. A green test suite cannot implement a feature
whose data type does not yet exist. Feature names use the vocabulary installed by prompt 127a; the Rust identifiers
still carry their pre-127a spellings, and the pairs are in [`../clean-break-ledger.md`](../clean-break-ledger.md).

Two ledgers answer "is it in yet", and the sections after them carry the argument — one per feature of the language
pass, in ledger order. The second ledger has no sections behind it because those features have nothing yet to argue.

**The four states.** `implemented` means the data type exists and its laws are stated. `implemented, unreached` means
the same and *nothing calls it yet* — the code compiles under an `expect(dead_code)` that prompt 142 will make
unfulfilled. `partial` means some of it stands. `absent` means no code answers to it, whatever the surface accepts.

## Ledger: the language pass

| Feature | Owner | State | Owes |
| --- | --- | --- | --- |
| [Total non-recursive source expressions](#total-non-recursive-source-expressions) | `musa-compiler` | implemented | conformance tests |
| [Definitional equality](#definitional-equality) | `musa-calculus` | implemented | 144, 148 |
| [Bidirectional elaboration](#bidirectional-elaboration) | `musa-calculus` | implemented | 142 |
| [Top-level declaration groups](#top-level-declaration-groups) | `musa-calculus` | implemented | 141o, 142, 162ba |
| [The constructor rule](#the-constructor-rule) | `musa-calculus` | implemented | 141g, 142 |
| [Inductive families and match](#inductive-families-and-match) | `musa-calculus` | implemented | 144, 148 |
| [Base types and builtins](#base-types-and-builtins) | `musa-calculus` | implemented | 142, 143 |
| [Numerals at a counting family](#numerals-at-a-counting-family) | `musa-calculus` | implemented | 142, 143, 148 |
| [The compiler's own domains and operations](#the-compilers-own-domains-and-operations) | `musa-compiler` | implemented, unreached | 142, 143 |
| [The CST read as a raw core term](#the-cst-read-as-a-raw-core-term) | `musa-compiler` | implemented, unreached | 142 |
| [A whole document, elaborated](#a-whole-document-elaborated) | `musa-compiler` | implemented, unreached | 142, 162ba |
| [A written piece as the track it denotes](#a-written-piece-as-the-track-it-denotes) | `musa-compiler` | implemented, unreached | 142 |
| [Records and enums as surface syntax](#records-and-enums-as-surface-syntax) | `musa-syntax` | implemented | 137, 142 |
| [The private marker](#the-private-marker) | `musa-syntax` | implemented | 137, 142 |
| [Module visibility](#module-visibility) | `musa-calculus` | implemented | 142, 166–167 |
| [Records and enums, elaborated](#records-and-enums-elaborated) | `musa-calculus` | implemented | 142 |
| [Traits and instances](#traits-and-instances) | `musa-calculus` | implemented | 137a, 142, 143 |
| [Method resolution](#method-resolution) | `musa-calculus` | implemented | 142 |
| [Traits and operators as surface syntax](#traits-and-operators-as-surface-syntax) | `musa-syntax` | implemented | 142 |
| [Storable](#storable) | `musa-calculus` | implemented | 143, 148 |
| [The syntax index](#the-syntax-index) | `musa-compiler` | implemented | 139, 143, 145 |
| [Quotation as a written form](#quotation-as-a-written-form) | `musa-syntax`/`musa-compiler` | implemented | 145 |
| [Quote patterns](#quote-patterns) | `musa-syntax`/`musa-compiler` | implemented | 145 |
| [Quotation in the core](#quotation-in-the-core) | `musa-compiler` | implemented | 142 |
| [A diagnostic carried whole](#a-diagnostic-carried-whole) | `musa-compiler` | implemented | 144 |
| [Causes restated for the session](#causes-restated-for-the-session) | `musa-project` | implemented | 142 |
| [Causes as related information](#causes-as-related-information) | `musa-lsp` | implemented | 144 |
| [Causes in the problems list](#causes-in-the-problems-list) | `apps/musa-desktop/ui` | implemented | — |
| [Collections](#collections) | `musa-syntax`/`musa-calculus` | partial | 142, 145 |
| [Per-term provenance](#per-term-provenance) | `musa-calculus` | implemented | 138 |
| [User-defined nominal data](#user-defined-nominal-data) | `musa-compiler` | absent | 142 |
| [Inductive families with indices](#inductive-families-with-indices) | `musa-calculus` | absent | 151, 155, 156 |

## Ledger: the rest of the roadmap

These features have a state and an owed test, and nothing yet to argue. When one of them acquires a design worth
defending it earns a section above.

| Feature | Owner | Where it stands | Next evidence |
| --- | --- | --- | --- |
| Building and closing a fragment into a core term | compiler to event track | implemented, but over the deleted contextual `music` type | prompt 142 rebuilds it over ordinary values; differential and closure tests |
| Finite `EventTrack<C,A>` operations, including unequal-duration `together` | `musa-events` | implemented at the old names and without the coordinate index | prompts 127b–127c; current 62 event track tests and final conformance audit |
| Versioned exact bytes for event-track equality | `musa-events` | implemented; the coordinate tag is not yet in the encoding | prompt 127c, then a migration test when a persisted reader is added |
| `Machine<K,A,B>` as a core value of the source language | `musa-compiler`/`musa-dsp` | absent | prompts 171–173 |
| `schedule(format, policy, time map, track)` with a recorded decision list | `musa-dsp` | absent | prompt 173 |
| Gesture event track | `musa-compiler` | absent | prompt 177 and its admission tests |
| Engraving plan and current exports | `musa-notation` | implemented for current score facts | language graduation matrix |
| Analysis packages with their own hidden value types and evidence | `musa-compiler` | partial built-in analyses; no general package mechanism | accepted source type design and real package examples |
| Valid whole-machine step order | `musa-dsp` | partial and not conforming to the new ordering rule | known ordering counterexample, machine-law tests |
| Feedback through initialized one-step state | `musa-dsp` | current behavior depends on caller buffer size | one-frame step, `batch` contract, and partition tests (R1-batch) |
| Complete `prepare_execution` operation returning a `PreparedMachine` | `musa-dsp` | absent in the specified form | instrument and preparation prompts |
| Cache that confirms complete audio arguments after hash lookup | audio/project | absent | exact `ExecArgs` record and forced-collision test |
| Versioned registry of source and derived representations | `musa-project` | absent | exact descriptor and merge validation |
| Complete origin paths with loss records | compiler/render/audio/project | partial source provenance only | generated-event and multi-pass path prototype |
| Structured editing that changes source | project/interface | architectural boundary implemented; feature set incomplete | interface and language graduation audits |

---

## Total non-recursive source expressions

`musa-compiler` · **implemented**

Implemented for the current value types and folds.

**Owes.** Whole-language conformance tests.

## Definitional equality

> Definitional equality over the dependent core, decided by normalization by evaluation.

`musa-calculus` · **implemented** · `02-core-calculus.md` §3

Implemented for universes, Π, dependent records with η, `let`, base literals, numerals at a counting family, and
ι-reduction on generated recursors, under the §4 budget.

**There is no identity type.** Note 50's course correction deleted `Id`/`refl`/`J` on the evidence that no committed
program used one, so the only equality this crate decides is the definitional one, and a program that has to compare two
values calls an `Eq` instance.

### One procedure

The facade's `convertible` is `Conversion::deciding`: the same walk with solving disabled, deciding by a type-directed
walk over values with η and early exit, and reading a value back only to build a mismatch's message.
Normalize-and-compare survives in `conversion_laws.rs` as the oracle it is checked against.

### The comparison is glued

Since prompt 141u (note 44 §6), a definition use is a `Head::Def` neutral carrying its identity, its type, and the value
its declaration computed, and `convert.rs` tries the folded comparison first.

- The same definition at convertible spines answers `true` without unfolding.
- A disagreement unfolds in a fixed order — the definition that can mention the other first — and retries.
- A same-head retry that also fails reports the folded failure, because it names what the author wrote.

`quote.rs`'s two modes keep the name on the diagnostic path and open it where a solution or a canonical readback could
let it escape its scope.

The accounting is exact. An unfold and its spine replay charge nothing — the eliminations were charged when the spine
was built — and a definition's body is still opened to weak-head form once at its declaration. `glued_laws.rs` states
the two laws the design owes: a body is paid for once however many uses it has, and a folded comparison's spend does not
measure the definition's normal form.

**The evaluator is a machine, since prompt 165a.** `kernel/eval.rs` is a `Step`/`Frame` loop rather than a set of
mutually recursive functions: `Frame` is the eleven shapes of pending work — an argument to evaluate, a function waiting
for one, a spine to replay, a memo cell to fill, an induction hypothesis to run — and `Frame::Dump` is Peyton Jones ch.
18 §18.8's saved level, pushed whenever the machine enters a *different* term so that the depth measured is the depth of
one term. Nesting is charged only on the non-tail descents (`Frame::Argument`, `Frame::Body`, `Frame::Codomain`) and
given back at the tail; a recursion is charged steps. The public shape of the module did not move: `eval`, `force`,
`opened`, `unfold_spine`, `apply`, `applying`, `apply_closure`, `neutral_type` and `head_type` have the signatures they
had. `family::iota` decides without evaluating and returns a `Fired`, and `Compiled::reduce` answers a `Matched` rather
than evaluating a body, so that both feed the machine instead of re-entering it.

Reclamation is a machine too, and for the same reason: `Neutral`'s `Drop` dismantles its spine with a worklist
(`kernel/value.rs`) and `List`'s frees its cells in a loop (`kernel/list.rs`), because Rust's derived drop is one host
frame per level of a value and a list of a few thousand elements aborted the process in `drop_glue<Value>` once the
nesting limit stopped refusing it. Nothing about ownership changes; a still-shared child stops the walk.

**And the unfold is recorded, since prompt 165b.** `Neutral` carries an `Option<Arc<OnceLock<(u64, Value)>>>`, allocated
only for the one head shape that unfolds, filled by the first consumer that forces it and read by every later one. That
is Peyton Jones ch. 12 §12.4's update of a shared redex's root, on a value shared by `Arc` rather than by a pointer into
a heap, and it is the difference between an empty staff region costing 455,942 steps and costing 2,818. The `u64` is a
process-wide stamp bumped in `Meta::solve`: equal stamps mean no metavariable was solved between the fill and the read,
which is the soundness condition a memo over a value that may mention an unsolved meta needs, decided in one comparison
and without walking anything. It is conservative in the safe direction — any solution anywhere invalidates every cell,
and being wrong costs only the work being done again. `kernel::eval`'s own test module states both halves.

### §4.1's second half, discharged at the facade seam

`room.rs` runs every entry point — `check`, `infer`, `normalize`, `normalize_type`, `convertible`, `convertible_types`,
the four `declare` doors, `Cx::assume`/`define` — on a scoped thread of `NESTING × FRAME_CEILING` bytes, 40 MiB, derived
from the published limit rather than picked. The wasm shell, and a host that will not give a thread, fall back to the
caller's own stack.

The budget does not move with it, so every host accepts and refuses the same programs and they differ only in what they
survive. `FRAME_CEILING` is 128 KiB since prompt 165a, against a measured 60–64 KiB a level in a debug build on arm64
and 8–16 KiB in a release build. It went *up* when the evaluator's frames went away: a level used to be bought mostly by
`eval` standing inside itself at about 2 KiB a frame, and is now bought by §5.9's traversal, by `quote`, and by the
elaborator's uncharged descent, which cost far more each. The command that produced the number is beside the constant.

It bounds what the *charge* bounds: the traversal and `quote` charge as they descend and are refused at any depth, while
the elaborator's own recursion is charged nothing and reaches the bottom of a raw term before anything is charged — so a
term far past the limit, 1,256 levels of raw `let`, measured, still aborts. `eval` is no longer on either side of that
sentence: prompt 165a made its pending work heap data, so it spends no host frames at depth at all.

**Owes.** Prompt 165: the elaborator's nesting charge and the spine walk that lets 256 stay 256, plus approximate
conversion (note 44 §7; §6's glued evaluation landed at 141u). Prompt 169: the metatheory matrix.

## Bidirectional elaboration

> Check/infer over a surface-independent raw term, with §2.1's metavariables standing for the binders a use site does
> not write, solved by pattern unification or postponed.

`musa-calculus` · **implemented** · `02-core-calculus.md` §2

Implemented at the size note 50's correction left it. The three mechanisms the feature name used to carry are gone.

**No level unknowns.** `level.rs` is `Type 0` and `Type 1` with no arithmetic beyond `succ` and `max`, a bare `Type`
elaborates at the level its use demands, and there is no level sort to solve in.

**No independent re-checker.** `recheck.rs` and the `well_typed` door are deleted, and the suites keep the eval and
conversion laws it used to back.

**Nothing postpones, retries, or generalizes.** A hole is created by one instantiation walk (`elab/spine.rs`) for one
binder the author did not write, solved by first-order assignment in `convert.rs`'s solving mode, and substituted away
by `Elaborator::zonk`. One still unsolved when a declaration ends is `Refusal::Unsolved`, naming which of `MetaSource`'s
two sites it came from, and the author writes the argument.

Introduction forms check only, so a record literal has no inference rule and an elimination applied directly to one is
`Refusal::Uninferable` rather than a guess; a `let` passes the goal through to its body rather than inferring, which is
what lets a case tree's leaf hold any introduction form.

**A hole's solution may itself be headed by a hole**, so `eval::force` sees through to a fixed point rather than one
step: an assignment stores the value the second hole had at that moment, and a caller that trusted one unfolding would
read a solved hole as an unsolved one and assign it a second time — assignment is write-once, so that refuses a program
which determined one thing once. Each pass after the first is charged, so a chain is bounded by the §4 budget rather
than by a claim that chains are short.

**Owes.** Prompt 142 cut the compiler over. What §2.1 now specifies is **absent**: there is no constraint queue, nothing
postpones, and no metavariable outlives the call that created it — the crate implements first-order matching where the
specification states pattern unification. Prompt 153 replaces the mechanism; prompt 154 adds the implicit arguments that
ride on it; prompt 152 gives levels something to solve in. Until then this row is *implemented against a superseded
§2.1*, and that is the gap to read it with.

## Top-level declaration groups

> A document's `data` groups, definitions, and instances, declared as one group.

`musa-calculus` · **implemented** · `02-core-calculus.md` §2.4

Implemented in `program.rs`. `declare_program` takes a `RawProgram` — every definition a document can see, each with an
origin, a name, a visibility, an optional module, an optional written type, and a value, plus every `impl` the document
writes, plus every `data` group beside the module it was written in — and `Cx::defining` brings all of it into scope in
one call.

Since prompt 162ba the families are in the same `RawProgram` rather than declared ahead of it through `declare`, because
the dependency between the two kinds runs both ways: a field is a type, and an index is a term. `declare` is still the
door a group goes through; what changed is that the door is opened from inside the analysis, at the position the order
puts the group in, rather than by a caller holding two lists and guessing which comes first.

A definition is a **global name**, not a binder, and the argument is sharper than symmetry with families: a de Bruijn
binder refers *outward*, so a named `Cx::define` could let the last definition see the first and never the reverse, and
closing that circle needs either substitution (§3: reduction is never performed on syntax) or a fixed point (§1.3
refuses one).

### Signatures before bodies, by dependency analysis

§2.4's "signatures collected before bodies" is delivered by dependency analysis rather than by a separate collection
phase — Peyton Jones ch. 6 §6.2.8, run before type-checking as ch. 8 requires — and one traversal answers both questions
the rule asks: the order the definitions are elaborated in, and the cycles that have nowhere to go.

Edges are read from the written type *and* the value, under a binder stack, so a local `let` or λ named after a
definition shadows it rather than producing a spurious edge — and a spurious edge is a spurious cycle, which is refused.
A self-edge is not a cycle: `declare_program` builds the `rec` form itself and hands it to `rec.rs`'s structural rule
unchanged, so the surface never decides whether a definition is recursive, and a self-recursive definition that wrote no
type is `UntypedRecursion` rather than an inference nobody could perform.

Because the order is computed, an *unannotated* definition is referable from anywhere too — the restriction it looks
like it needs is one the analysis removes.

### Two kinds of edge, because instances travel in the same group

The instances travel in the same group (prompt 141r), because each kind can name the other and neither comes first: an
`impl`'s method bodies are ordinary terms that may call any definition, and a definition may resolve a method by
receiver against an instance the same document declares. That takes two kinds of edge.

- A **name** edge is hard — a free definition name, read from an `impl`'s parameters, arguments, `where` clause, and
  method bodies exactly as it is read from a definition's type and value — and it means "before that one".
- A **method** edge is soft: a declaration writing the spelling `m` edges to every instance of every trait
  `Classes::declaring_method` says declares `m`, which means "before *one of* these", because which one depends on a
  receiver type not known until this declaration has been elaborated.

A soft back edge is dropped. A hard back edge is `DefinitionCycle` only when its cycle is hard the whole way round, and
otherwise the deepest soft edge inside that cycle is dropped instead — the refusal belongs to the cycle, not to the edge
that closed it. Hard edges point only at definitions and soft edges only at instances, so every cycle through an
instance holds a soft edge and no instance is ever caught in a `DefinitionCycle`.

What a dropped edge costs is `NoMethodForType` **at the call**, which is what a genuine mutual dependency between a
definition and a dictionary deserves — an instance is never `rec`, so a cycle through one has nowhere to go. Each kind
comes back in the order the document wrote it.

### What a use is

A use is `Shape::Def`, one node whatever the definition holds, and δ in `eval.rs` folds it into a `Head::Def` neutral
that *carries* the `Arc<Value>` the declaration computed — [Definitional equality](#definitional-equality) is the
semantics that reference obeys. The `Defined` behind it stores two `Value`s and no `Term`, which is what keeps this free
of the `Arc` cycle `family/` needed a declaration context to avoid.

Visibility is 136a's mechanism unchanged, filtered in `elab/name.rs`'s name resolution after binders and declarations
and before the host's registry. Laws in `crates/musa-calculus/tests/suite/program_laws.rs`.

**Owes.** Prompt 141o assembles the group out of a document's lowered declarations; prompt 142 is the first *pass* that
hands it a piece.

## The constructor rule

> §2's constructor rule: `C a⃗ ⇐ N p⃗`, with the family's parameters read off the expected type rather than written.

`musa-calculus` · **implemented** · `02-core-calculus.md` §2

Implemented in `check` and nowhere else, as the raw-term twin of `family::realize`: the value side has supplied a
δ-rule's parameters since prompt 141b, and this makes the two doors into canonical data agree rather than deciding
anything new.

It fires where the expected type is an element of a family, the head of the written spine names one of that family's
constructors, and no more arguments are written than the constructor has fields — so everything else, including the
fully written `Option.Some Nat 0`, falls through to `Switch` exactly as before, and the widening accepts programs
without refusing any.

§1.3's bare-word rule is folded in rather than left beside it: a word standing alone is qualified against the expected
type's family, and only after a binder and a declaration have both declined it.

**There are no indices to read.** §1.1 gives a family parameters and nothing else, so the expected type's arguments
*are* the parameters, and there is no position at which a constructor could have chosen a value that reading the
expected type would have to assume.

**Not by marking the parameter `Filling::Parameter`** — trialled and rejected on measurement: 28 of `musa-calculus`'s
163 laws fail, because `infer(Option.Some)` stops answering the closed term that 141b's builtin vocabulary requires.

**Owes.** Prompt 141g's lowering reads `Some(x)` out of source and hands it here; prompt 142 is the first caller with a
real program.

## Inductive families and match

> Parameterized inductive families with strict positivity, generated **dependent** eliminators, `match` compiled through
> case trees with coverage, and the structural termination rule.

`musa-calculus` · **implemented** · `02-core-calculus.md` §1.1, §6.2

Core complete: mutual `data` groups, positivity over the whole group, generated motives and methods, case-tree
compilation with column splitting, unreachability and incompleteness reporting, and recursion admitted by rewriting each
call into its branch's induction hypothesis — with the binders after the recursive argument moved inside the arms first,
so the motive generalizes them and an accumulating traversal gets a hypothesis that is a function of the accumulator
rather than a value at the branch's own copy of it. A call that changes an argument *before* the recursive one is
refused by name.

### Positivity admits §1.1's whole rule

Not merely the direct case. Each `Group` records, per parameter, whether that parameter occurs only strictly positively
in the fields the group stores — computed at `declare_data` from the constructors just built, with the group's own
families taken optimistically so `Cons : A → List A → List A` terminates. A later declaration's field may therefore
*contain* itself at a parameter its container was found positive in, so `Body(items : List<StaffRead>)` declares.

`Constructor::recursive` stays the direct occurrences alone, so such a field carries **no induction hypothesis** and a
fold *through* a container is not yet writable; that gap is asserted as a law rather than left to be discovered. The
arrow rule is untouched at any depth and under any container. Laws in
`crates/musa-calculus/tests/suite/nesting_laws.rs`.

### The motive is dependent; there are still no indices

Prompt 155. The generated eliminator's motive is a **family** — `R_j : (t : N_j p⃗) → Type ℓ` — so a method's result is
`R_j (c p⃗ a⃗)` and an induction hypothesis is `R_j a`, and a `match` arm is checked at the goal *refined by the pattern
it matched*. `case.rs` builds the motive by abstracting the subject out of the goal, which it does by rebinding the
subject's binder and re-evaluating (`Env::rebinding`): substitution in the semantic domain, because §1 gives the core
none on terms. A `match` that refines nothing gets the constant family `λ_. G` and is the term it always was.

The case tree is a value (`kernel/case_tree.rs`): `Answer`, `Split`, `Impossible`, with §6.2's emission to nested
eliminator applications, and coverage asked of the finished tree against the declaration group rather than of the loop
that built the branches. `Impossible` has no producer until indices arrive.

What is still absent is **indices**: a family is completely described by its parameters (`family/mod.rs`), so there is
nothing for unification to refute and no forced pattern to elaborate. Note 50 deleted the general indexed machine and
the dependent motive together, and prompt 142d rebuilt a *different* mechanism — an erased index sort — in their place.
Prompt 143 reversed that; the motive came back at 155 and the indices belong to
[Inductive families with indices](#inductive-families-with-indices).

Mutual recursion between _definitions_ is deferred.

### Column selection

Maranget's necessity condition specialized to an ordered `match`: the first row decides whether to test at all, and the
column is the leftmost that row tests. An arm's body is still elaborated once per leaf it reaches.

**Owes.** Prompts 136–141 build records, enums, traits, `Syntax<Cat>`, and the collections on top of it as library code.
Prompt 165 owes the arm-body hoist and its measurement (note 44 §2). Prompt 169 owes the metatheory matrix.

## Base types and builtins

> Base types, their literals, and the four families of compiler-owned builtin over them.

`musa-calculus` · **implemented** · `02-core-calculus.md` §5.8

Implemented as a mechanism the host fills. `Base` is a name and a kind; `Literal` is a type and an opaque `Payload` the
host implements; `Builtin` carries its family and a `fn` δ-rule — a function pointer, so D3's purity is the type rather
than a promise — and an immutable `Registry` rides on `Cx` beside `classes`, so a context with no registry names no base
type and every earlier suite is unchanged.

δ fires in the same arm of `apply` that fires ι, at the moment the last argument arrives; short of that a builtin is a
rigid neutral head. Registration checks the half of D1–D4 a signature makes visible — one name one meaning, every
argument and result type finite data, every base type it mentions registered inert — and D2/D3/D4 stay with the host's
table, where prompt 127ca's suite samples them.

### D1's *or* is `Datum`

A δ-rule is `fn(&[Datum]) -> Option<Answer>`, and a `Datum` is either a literal or a constructor named `Family.Case`
carrying its fields in declaration order — so a builtin may be handed `Some(3)` and may answer one, which the 92-entry
table needs for the 38 δ-builtins that answer `Option`, `Result`, or `List`.

### A rule has three answers because elaboration has three outcomes (§4)

- `Answer::Reduced` is the datum it computed.
- `Answer::Refused` is *the program is wrong*, and carries the sentence to say.
- `None` keeps its old meaning — arguments the rule's own signature does not admit, which is the host's table
  disagreeing with its own rule, and stays `Malformed::BuiltinStuck`.

A refusal becomes `CoreError::Refused` at the application's origin and reaches `lower/refusals.rs` like every other
refusal: the rule says the sentence and the core says where, because a rule sees data rather than terms and has no
origin to name. Widening the answer to `Result<Option<Datum>, Refusal>` instead would hand every host rule the core's
refusal vocabulary and its origin discipline, which is a wide interface for one narrow need.

A refusal fires when the *rule* does — at conversion or normalization, not at `check`, since a rule only looks at its
arguments once they arrive — so `int_div 6 0 : Int` is a true typing judgment whose reduction is refused. Structural
rewrites are untouched: a traversal rewrites a term and does not judge a program.

The finite-data check is positive rather than a search for arrows: a type is finite data when its head is a registered
base type or a declared family and every argument is finite data in turn, so a Π anywhere is still `HigherOrderDelta`
and everything else is `NotFiniteData` by name.

### Reading needs no type; writing is type-directed

The two directions are deliberately asymmetric, because a constructor value carries its family's parameters before its
fields and ι reads them by position.

**Reading needs no type** — canonical data is canonical, so a forced argument is a literal, a saturated constructor, or
the builtin does not fire. The same reading is public as `musa_calculus::canonical`, at a *term* rather than a value: a
consumer of `check` holds a normal form, and a count written as a `Nat` or a list of pairs is a constructor spine that
`read_back`'s one-`Shape::Lit` question refuses. It needs no context and cannot fail — a constructor spine carries its
own `Constant`, which carries the group that declared it, so where the fields begin is already in the term — and the
parameter-count rule is shared with the value-side reading rather than copied, so the two cannot disagree about a family
whose parameters change. `read_back` is deliberately *not* restated over it: `read_back` borrows a `&T` out of the term
and `canonical` owns a data tree, so routing one through the other would clone a whole `VoiceTrack` datum on the path
every voice takes.

**Writing is type-directed.** The answer is realized against the builtin's own result type, instantiated at the
arguments actually applied, which is where the parameters come from; the constructor is found by name in that family,
its stored field types are read in `Group::declarations` extended by the parameters and then by each field as it is
built, and a name that is not a constructor of that family, or one at the wrong arity, is one `MisfitAnswer`.

**Both directions walk an explicit stack and charge steps, since prompt 165b.** Reading a value into a `Datum`
(`eval.rs`'s `canonical`), reading a *term* into one (`family::datum`'s `read`, behind the public `canonical`), and
building a `Datum` back out (`family::datum`'s `realize`) were three recursions on the host stack, and the first and
third charged one `Metric::Nesting` level per level of the data. The depth of a list a δ-rule is handed is the size of
one argument, which is neither of the two descents §4.1 derives the metric from, and a six-hundred-element list was six
hundred levels of a limit of 320. All three are loops over a frame stack now, and the two that are metered charge one
step a node. `budget_laws.rs` states it from both ends: a list two thousand long is built and read on a 2 MiB thread,
and the charge is one per node counted by hand.

**The core learns no musical type**: the table is `musa-compiler`'s `BUILTIN_OWNERSHIP`, and the worked registry in
`base_laws.rs` is `Int` and `Text` on purpose.

**There is no literal pattern**: a base type has no eliminator, so a base-typed column admits only a catch-all and a
destructuring pattern at one is refused by name; matching *against* a literal is decidable equality, which desugars in
the host.

### The second family: structural eliminators

`Builtin::structural`. A `Rewrite` answers a *term* rather than a literal, which is what ι already does, and fires on
one declared target argument once that argument is a literal — so a traversal's algebra may be a λ or a neutral and is
passed through untouched, never forced. The answer is read in the environment of the spine's arguments, so the rewrite
names them by index and the core evaluates what it wrote; the rewrite is handed its own `Builtin` because a traversal
recurses and a `fn` pointer cannot capture one.

Registration checks that the target names an argument and that its type is headed by a registered base type — a rewrite
over a declared family would be a second ι-rule. Descent stays with the host, like D2 and D4; the meter is the backstop,
so a rewrite that does not descend is refused rather than run forever.

A rewrite may also name what it *builds*: `Builtin::structural_with` takes a **vocabulary** of closed terms the host
resolved from its own context — `List.Empty`, `List.Cons`, a family constant — and `Builtin::vocabulary` hands them back
through the `&Builtin` the rewrite already holds. Nothing about D3 moves, because the terms are values fixed at
registration rather than a lookup performed at reduction, and `Builtin::structural` stays as the empty case. Closedness
is the host's obligation, stated where the laws can check it.

### The fourth family's reduction shape is none at all

`Builtin::constructor` registers a name, a type, and a `Family` and nothing else, because
[`../../rules/across-stages/03-machine-calculus.md`](../../rules/across-stages/03-machine-calculus.md) §2 gives its
forms typing rules and no reductions — such a builtin's saturated application *is* its value, and two of them are
convertible exactly when their spines are.

It is not a rule that always answers `None`. There is no rule, so `Malformed::BuiltinStuck` cannot misfire, §5.8's
inertness argument covers it the way it covers a base type, and `Registry::new`'s two checks pass over it — the δ check
because the family is not `Delta`, the target check because there is no structural rule — which `base_laws.rs` states as
an absence of two questions rather than an exemption from them.

Nor is it a weaker `Builtin::new`: every machine form is polymorphic in its ports, so a type stands on every spine and
`canonical` answers `None` at a universe, and a δ-rule registered there could never have fired.

**Owes.** Prompt 142 writes the desugaring and cuts the compiler over; prompt 164 collapses the table behind traits.

## Numerals at a counting family

> A number written as a number, at any family shaped to count.

`musa-calculus` · **implemented** · `02-core-calculus.md` §5.10

Implemented in `family/` and threaded through every walker. A **counting family** — no parameters, exactly two
constructors, one with no fields and one whose single field is the family itself — is recognized at `declare_data` from
the declaration's own shape, so `Nat` is not privileged and nothing in the host nominates it.

`Counting { floor, step }` rides on `Declared`; `Shape::Numeral`/`Form::Numeral` hold a `Numeral { family, count }` and
are **leaves**, one node whatever the count.

### One canonical form, and the collapse runs toward it

`Constant::value` turns the floor into numeral zero and `family::stepped` turns the step applied to a numeral into
numeral+1, both called from `eval`'s own arms — so no second path can produce a spine the canonicity claim says does not
exist, and conversion at a counting family is a `u64` comparison rather than a walk.

The tower reappears **one level per elimination**: `family::ready` unfolds a numeral major premise into the floor's
method or the step's method with the field bound to numeral−1, which is `case.rs`'s route too since a `match` compiles
to nested recursors. So a fold over `n` costs the `n` steps it names, and *writing* the number costs one.

### Across the δ boundary

A count crosses as `Datum::Count { family, count }`, not as a `Datum` tower: a tower would cost a node per unit **and**
recurse on `Drop`, and `canonical`'s per-level nesting charge capped it at 256, so the numeral would have been a hole in
§4.1's budget. (That charge is gone as of prompt 165b — see above — but the tower's other two costs are not, so a count
still crosses as a count.) `realize_count` reads the element type off the signature and refuses a count at a family that
does not count.

Overflow past `u64::MAX` neither saturates nor refuses — the step stays an ordinary blocked spine, `module-design` rule
5, and unreachable at 2⁶⁴ steps anyway. `Refusal::NotANumeralFamily` names the first condition of the counting rule the
written family fails, in the order an author reads a declaration in.

Laws in `crates/musa-calculus/tests/suite/numeral_laws.rs`: convertibility with the hand-built tower and agreement under
a `match` at every count the tower can still be written at; a nesting cost independent of the count across three orders
of magnitude; and fifty thousand neither overflowing the stack nor deepening the term — on both sides of the data
boundary.

**Owes.** Prompt 142 writes numerals from the surface; prompt 164 adds arithmetic over them; prompt 169 audits §5.10
with the rest of §5.

## The compiler's own domains and operations

> The compiler's own domains and operations, registered in the core's terms.

`musa-compiler` · **implemented, unreached** · `02-core-calculus.md` §5.8

Implemented for the δ half. `prelude.rs` declares `Bool`, `Nat`, `Option`, `List`, `Result`, `Scope`, and `RowFault` —
`Scope` is *declared* rather than registered because it is finite data with three cases and nothing hidden behind them,
while an `Origin` beside it is a base type, since no program takes one apart and the Origin view reads a compiled
projection a stage down.

`registry.rs` registers 41 inert domains as base types over one generic `Payload` wrapper, translates `Shape` to a core
`Term` in one recursion rather than retyping 117 signatures, and assembles a `Cx` in four stages: structural
declarations, base types, the families that name one, then the builtins.

### What each module registers

- `registry/rules.rs` carries 106 `fn` rules — the 92 δ entries of `BUILTIN_OWNERSHIP` and the 14 δ builders of
  `SYNTAX_OWNERSHIP`.
- `registry/traversal.rs` registers `recurse_syntax` and `syntax_fold_from_leaves` as §5.8 *structural eliminators* over
  `Syntax`, each with the vocabulary its rewrite may write.
- `registry/track.rs` registers the eight track builtins as §5.8's *third* family over an `EventTrack` base type — seven
  with δ rules written from `WrittenPitch::transpose`, `ScoreFact::stretched`, and `musa_events::together` rather than
  from the old evaluator's arms, which build a deferred `MusicOperation` and compute no track at all, and
  `map_note_pitches` with a *structural* reduction, because its mapper is a function and a δ rule never sees one.
- `registry/machine.rs` registers §5.8's *fourth* family.
- `registry/notation.rs` adds the five words a notated block is built out of.

### Refuse, or answer with a value

All eight track builtins answer a track, and the four that can fail — `stretch` at a factor of zero or below, `shift`
outside its track, `play` and `sounded` at a length a chord or a fact cannot sound for — *refuse the program* through
prompt 141m's channel rather than handing the composer's own mistake back as a value.

That is the criterion applied: `Option` is a musical answer a program branches on, `Result τ Text` is a diagnostic
wearing a value's clothes, and none of those four sentences is something a caller can do anything with but stop.

The arithmetic, duration, and position rules in `registry/rules.rs` are the same judgment and still answer a `Result`,
because `BUILTIN_OWNERSHIP` is one table read by two checkers and narrowing them rewrites the twenty corpus sites that
read them — 141m's survey table lists the eleven rules and the twenty sites, and prompt 142 carries the move. `Row12Of`
and `checked_expression` keep theirs for good: a `RowFault` names which positions repeat and which classes are missing,
and the syntax gate exists so a transformer can decide what to say about a tree it built badly.

### Registrations past both ownership tables

`play` is the only builtin that *constructs*, so §5.7's clause is discharged there: its `Origin` and `Scope` are
arguments, since a source span is not something a `fn` pointer can invent.

- `set_note_pitches` is registered in neither ownership table, for `instantiate_quote`'s reason — it is the putting-back
  half of the controlled transform, and a source word for it would be the control removed.
- `instanced` is the ninth registration and the second past both tables, on the same argument about provenance rather
  than pitch: it prepends the expansion path of the `Origin` it is handed onto every fact of a track, which is how a
  `make` says which site produced its notes, and a source word for it would let a program claim its own notes were
  generated by a template that never made them.
- `spliced` is the tenth registration and the third past both tables. It takes an `EventsTerm` — a new inert base type
  holding a checked `musa_events::Term` whole, beside `Template` and for `Template`'s reason, since `01-surface.md` §7
  gives a quote's body no eliminator and no spelling beyond the form that writes it — and a list of tracks, binds the
  *i*th track to the *i*th hole the reading named, and evaluates. Past both tables because
  `events EventTrack[WrittenTime, ScoreFact] { … }` already spells the whole operation, so a row would invent a second,
  *called* spelling over a term no expression can build; `instantiate_quote` is out of both tables for that reason one
  stage up. It needs neither an `Origin` nor a `Scope` argument, because `crate::lower::event track` stamps both at read
  time — a quote is a written form, so its span, its scope, and its splice step are fixed before any material arrives —
  and the material a hole brings is stamped by `instanced`, whose one operation serves an instance site and a `${…}`
  alike.

### The five notation words

Which the eight did not have: `sounded` puts *one* fact of any kind over `[0, held]`, `follow` places one track after
another, `tied` marks what a `~` was written on, `joined` is where a tie stops existing, and `nothing` is the empty
track.

`tied` and `joined` are two words rather than one because roadmap §6.3's tie is a property of *two* facts: a notehead
can only say "I continue", and `sounded` builds one fact, so it writes no tie and never could. `joined` is applied once
per voice — by `lower/piece.rs`, around the whole of one — because both of its refusals are questions about a whole
voice: a tie at the end of a `repeat` body continues into what follows the block, so a merge done inside one would
report a dangling tie at every nesting level, and "this tie has nothing to tie to" is only answerable where there is
nothing after.

Its fact argument is `prelude.rs`'s `Fact`, a *declared* family mirroring `elaborate/fact.rs`'s nineteen `FactKind`
cases one for one — declared and not registered because the cases are the score's own vocabulary and `07-analysis.md`
wants to `match` on one, while the eleven payload domains it names (`Mode`, `Clef`, `NotatedDuration`, `FreeDuration`,
`Mark`, `MarkArgument`, `DynamicMark`, `Progress`, `Metronome`, `Ramp`, `ChordSymbol`) go the other way, since a case
that spelled a `NotatedDuration` out of a `Ratio` and a `Text` would let a program build one whose spelling and value
disagree.

`sounded`, `follow`, `tied`, and `joined` are registered in neither ownership table for `set_note_pitches`'s reason — a
source `sounded` would let a program put a `Fact.Key` in the middle of a voice, which `00-semantics.md` §3 forbids, and
a source `tied`/`joined` pair would let one mark material it did not write as continuing, or join two noteheads a
composer wrote as two. `nothing` is registered at all: a δ-rule fires when its last argument arrives and a
`Builtin::constructor` does not reduce, so a word of no arguments is neither, and it is a literal at
`EventTrack ⟨written⟩` that the lowering embeds. Not a `Definition` either, which `run_syntax_step` is only because a λ
has no inferable type; a literal at a base type carries its own.

`sounded` admits a span of no length where `play` refuses one, because a mark, a grace note, and a dynamic are point
occurrences while a chord sounding for no time is not a sounding.

`run_syntax_step` is a `Definition` — a type and a checked term — rather than a registration, because a projection hides
nothing and its target is a declared family the core would refuse. `Coordinate` and `Cat` are base types whose values
are literals rather than declared families, because a `fn` rule cannot capture a context and `duration_of` has to
*write* `Duration ⟨written⟩` itself.

### The machine forms, and the two checks deliberately not made

`registry/machine.rs` registers `03-machine-calculus.md` §2's eight grammatical forms as `Builtin::constructor`s over
`Machine` and `Primitive`, two base types at `Type 0 → Type 0 → Type 0 → Type 0`, with every type argument a
`Filling::Parameter` binder written by `Term::parameter_pi`, because §2 names four of the eight rather than applying
them and `identity` is a machine rather than a function to one — a parameter is what a use site does not write, so a
nullary form's ports are solved from the position it stands in. Its ports are `Pair` and `Unit`, declared in
`prelude.rs` as families rather than spelled as prompt 136 record types, because §2's pairs are positional wiring and
field names for them would make the surface read a wiring diagram as a record.

Storability *is* enforced, and by the signature rather than by a pass looking for arrows. `scheme` writes
`02-core-calculus.md` §1.2's `Storable` constraint with `requiring_storable` over exactly the binders §2's rules write
`data A` above — `machine`'s two ports, `identity`'s one, `feedback`'s stored value — and elaboration discharges it.
There is no instance for an arrow, so `Machine ⟨step⟩ (Nat → Nat) Nat` is refused where it is written. `connect` and
`beside` carry no constraint, because their rules carry no `data` premise and inventing one would refuse a program §2
admits.

Two things §2 asks are deliberately **not** checked there, and are named where the omission is visible:

- `primitive` is registered nowhere, because its type is read out of the build-local registry — the written name and
  version select a descriptor supplying the step, both ports, and the configuration argument's type, so it is a
  different type per registered pair rather than one Π short of writable.
- The step position is not checked to hold a step *tag*: `Machine Nat A B` type-checks here, because a tag is a host
  notion and this crate's registry is the only thing that knows the list. `lower/types.rs` refuses it where the position
  is, against `machine.rs`'s own `is_step_tag`.

§5's preparation refuses what survives both ("a well-typed machine may still fail preparation").

### The accounting, and why nothing calls it

The 10 rows left are the eight collection eliminators (141c's argued exclusion), `primitive`, and `run_syntax_step`,
counted from the tables so the accounting cannot drift.

Nothing calls it: both modules carry `#[cfg_attr(not(test), expect(dead_code, …))]`, so prompt 142 wiring the elaborator
makes the expectation unfulfilled and the compiler says so.

The laws are unit tests in `registry/laws.rs`, `registry/traversal/laws.rs`, and `registry/track/laws.rs`, because the
agreement laws need the old evaluator and `eval_builtin`, `expand_region`, and `Syntax` are private to this crate.

- `core/oracle.rs` samples each δ signature and answers with the old `eval_builtin`, encoding its values into `Datum` by
  a second hand-written path, so a broken encoder cannot make agreement pass.
- The traversal laws rebuild one real region node for node under both evaluators and compare whole `Syntax` values.
- The track laws ask each of the eight twice — once against the pure operation it was written from, on a hand-built bar
  of two noteheads and a slur, and once through the core at its own registered signature, which is the only way to ask
  `map_note_pitches` at all.
- `registry/notation/laws.rs` states the `Fact`/`FactKind` mirroring from both ends at once — an exhaustive `match` over
  `FactKind`, so a twentieth kind stops the crate compiling, and the constructors read off `prelude.rs`'s own
  declaration, so a twentieth case with no kind behind it fails too — and runs all nineteen through `sounded` against
  hand-built expected kinds, which is what checks the field data the two ends cannot see: a field read at the wrong
  offset, out of the wrong domain, or in the wrong order answers a kind the comparison names.

**Owes.** Prompt 142 elaborates the surface into this registry and deletes the old checker and evaluator; prompt 164
collapses the table behind traits.

## The CST read as a raw core term

> The surface CST read as a raw core term, and a core refusal restated at the span that caused it.

`musa-compiler` · **implemented, unreached** · `02-core-calculus.md` §2, §7

`lower.rs` holds `Sites` and the CST helpers, and six modules hold the reading: `types.rs` for written types,
`values.rs` for expressions and patterns, `items.rs` for declarations, `quotes.rs` for both quotation forms,
`notation.rs` for a notated block, `refusals.rs` for the way back.

### One direction, and the core has the other

Nothing here resolves a name to an index, unifies, inserts an implicit, or checks coverage, positivity, or termination —
so an unknown name is *written through* as a `Raw::var` and the core answers `UnknownName` at the origin this module
gave the node.

A literal's base type is the surface node's — `3` is a `Nat` wherever it is written, and `chord c# minor` supplies a
`NoteName` because a `ChordExpr` is what it is — which is §2's "a literal infers", and is also the one source change
prompt 142's migration owes.

### Desugaring is the operator table the core never learns

§1.5 decides which of two shapes each row takes. `x == y` is `Eq::equal(x, y)` and `xs[i]` is `Index::at(xs, i)` — the
*qualified* spelling, because §5 requires an operator to resolve "at a known head **or** under a `where`" and method
syntax gives only the first: a generic receiver has no head an instance is filed under, so `x == y` inside
`fn same<A>(…) where Eq<A>` would be `MethodOnVariable`.

`p up M2` and `p step n` stay `RawShape::Method`, because their receivers are concrete and their operations are inherent
items in a type's namespace, which is exactly the case §6's exact-receiver lookup is for. `if` is a `match` on `Bool`.

### A `::` path is read by capitalization and nothing else (§1.5)

Lowercase segments are modules and are dropped; the first capitalized segment and the one item after it are joined with
`.` — the core's own spelling, `Nat.Succ`, `Eq.equal` — and the result is written through as a variable, so a path
naming nothing is `UnknownName` like every other name.

The one thing a *reading* refuses is more than one segment after the capitalized one (`Code::QualifiedPath`), because
that is a claim about the written text. `phase_literal` is asked from the path reading and from the dotted `NameExpr`
reading alike, so `TokenKind::Comma` and `TokenKind.Comma` reach one vocabulary rather than two that agree by accident.
A pattern's path is read the same way, from the flat token run the grammar writes it as — `Tying::Untied` was a
constructor named `Tying` binding a variable named `Untied` until this reading told the two apart.

A `record` lowers to a *definition* whose value is a record type, because `01-surface.md` §1.2 makes a record
structural, while `data` and `enum` are nominal and each becomes one `RawData`; a type parameter is explicit on a
declaration and implicit on a `fn`, because `List<Nat>` writes its argument and `same(x, y)` does not.

`Sites` numbers a node per reading and is deliberately not deduplicated: two nodes with the same span are two nodes.
`refusals.rs` names all fifty-three `Refusal` variants once, so a variant added to the core fails to compile here until
somebody says which `Code` it is, and a refusal carrying `Origin::UNKNOWN` — a registered signature's, which nobody
wrote in a file — points nowhere rather than at node one.

### A notated block is a left fold

`notation.rs` reads one as `follow(follow(nothing, s₁), s₂)` over the statements in written order, seeded with the
`nothing` literal, with no cursor anywhere — `00-semantics.md` §3 deleted it, and a fold has none to delete.

One statement table from `SyntaxKind` to the call it makes, rather than nineteen methods, because 141j's mirroring law
exists to protect exactly that correspondence:

- A note, and each pitch of a bracketed simultaneity, is `sounded` with a `Note` fact, folded with `together`.
- Every annotation is `sounded` with its own `Fact` case.
- `stack` is the one statement that reaches `play`, since `play` takes a `Voicing`, a `Voicing` takes a `ChordClass`,
  and a written `[c4 c#4]` names none.
- `use e;` is `e` itself.
- A transformation block is the matching track builtin applied to the fold of its body, which is `00-semantics.md` §3's
  implementation theorem rather than a duplicated convention.

**A block asks nothing**: every constructor a statement reaches for is total (141m), so the reading writes no `?` of its
own and drains none, and a block denotes `EventTrack ⟨written⟩` — which is what makes `use e;` the sentence §2 says it
is, since `follow` demands a track and a saved fragment now is one.

`play` is the one call whose arguments are not exactly what was written: a source `play(v, d)` reads as
`play(⟨origin⟩, ⟨scope⟩, v, d)`, because §5.7 requires a constructed fact to carry both and a composer has neither to
give. One entry in the reading rather than a mechanism, and unambiguous by arity — the registered `play` takes four.

### The reading context travels down only

Scope, the lexical scale, and the origin path travel *down* and never back up. `in scale` moves the pitch a `step` reads
and emits no fact; a `step` with no collection in force is refused where it is written rather than defaulted to C major;
and a key, meter, tempo, clef, part, or voice change inside a `music` value is *misplaced* rather than unsupported,
because the same statement in a score is legal and these are permanent answers.

Which of the two a block gets is the reading's third bit, `placed`, and not its scope: a free `music { … }` value reads
at `Scope::Piece` and so does a piece header, and what separates them is whether there is a single *here* — so where a
block is placed the same four statements are ordinary points, at the scope `lower/piece.rs` picks for each.

`motif` is the `fn` §2 says it is and `fragment` the `let`, both without a written return type: §2 spells one, and
141j's fallible vocabulary makes it stale, which prompt 142 reconciles along with the source.

### What is refused at the node

Two forms, with the prompt that owns them named: an events quote (142, which teaches the source to spell the track 141h
gave a core shape) and an anonymous product (142's records).

A `where` clause on a free `fn`, a `record`, or an `enum` is no longer among them — prompt 141i gave it the constraint
binder `01-surface.md` §1.4 always described, so the clause is read through `written_constraints` and folded into the
declared type. What is refused there now is *misplaced* rather than unsupported: a `where` on an impl method, which
takes its type from the dictionary field it fills, and a `where` on an unannotated `fn`, which has no signature to
constrain.

Nothing outside `document.rs` calls it, and that is unreached too: both modules carry
`#[cfg_attr(not(test), expect(dead_code, …))]`, so prompt 142 wiring them makes the expectations unfulfilled and the
compiler says so.

**Owes.** Prompt 141h gave `EventTrack` its core shape and prompt 141ha gave `Machine` its own; prompt 142 is the first
caller and owns the source changes the node-decides-the-domain rule implies.

## A whole document, elaborated

> Which declarations a document holds, gathered and handed to the core as one group.

`musa-compiler` · **implemented, unreached** · `02-core-calculus.md` §2.4, `01-surface.md` §1.2

In `document.rs`. A document is a list of `Source`s — a node whose children are declarations, plus the one bit lowering
cannot read off the node: whether §5.9's phase vocabulary is spellable inside it. Import *order* is `crate::imports`',
because a walk that learned about import graphs would be resolving names a second time.

### There is one door, and the order behind it is computed

Every declaration — `data` groups, `record`s, definitions, and what `impl` blocks leave behind — goes through
`declare_program` in one call, and the order is the dependency order that call computes.

It was three doors in a forced order until prompt 162ba, and the order was wrong rather than merely coarse. "Families
first, because a field is a type" is true and is half of it: a field is a type, but an *index* is a term. §1 makes a
family's index "any term of the index's type — a call, a projection, a value the program computed", so
`data Vect<A>(n : Nat) { Cons(…) : (m + 1) }` names `Nat.add`, and a walk that had declared no definition yet could only
report that as a name nobody wrote. The dependency runs both ways, so no order fixed by kind answers it.

There was a fourth door before that, and prompt 141r removed it for the same shape of reason: a `declare_impl` loop
running after the definitions made every definition blind to its own document's instances, which reads as
`NoMethodForType` at a call whose `impl` is twenty lines above it.

The limit 141o recorded — a family whose field names a `record`, since §1.2 makes a record a definition — is gone with
the ordering that caused it, and `document/laws.rs` writes one.

### The edges are the core's own reading

Not the document's: `declare_program` walks the `Raw` shapes it owns, under a binder stack, for a `data` group exactly
as it already did for a definition. That replaces a token pass over the CST, which over-approximated safely while the
graph held only type names and would not have while it holds every definition's name too — a `fn` named after a
`record`'s field is an ordinary program and was one spurious edge away from a refused document.

A cycle is `Refusal::DefinitionCycle`, restated as `Code::DependencyCycle` at the declaration that closes it, wherever
the cycle runs. A `data` group on one is refused with the rest: the core's mutual-recursion door is one group with
shared parameters, and two written declarations share none.

### Two questions and no others

`Document` answers a name's normal form at its own type, through `infer` and `normalize` so the reading is the one a
source term gets, and the site table that restates the resulting refusal — which is §7's arrangement, with the core a
leaf that never learns what a file is.

The **survey** in `document/laws.rs` is the deliverable rather than a check on it: `stdlib/`'s sixteen non-adapter
libraries elaborate as one document with exactly two faults, `Music` and an anonymous product, and each adapter
elaborates in phase scope with one and two — every one of them a row in prompt 142's Target. The lists are exact the way
`registry::rules::UNREGISTERED` is exact, so a corpus file that starts failing for a new reason fails the build.

**Owes.** Prompt 142 owns the readback out of normal forms, the four passes that call this, and the deletion of the
checker it replaces.

## A written piece as the track it denotes

> A written piece as the track it denotes, and the structure that gives "from here onward" a place.

`musa-compiler` · **implemented, unreached** · `00-semantics.md` §3,
[`../../rules/events/03-denotational-semantics.md`](../../rules/events/03-denotational-semantics.md) D3

In `lower/piece.rs`. §3's *second* composition equation, over a score's voices, where `notation.rs` is the first over a
block's statements: `together(v₁, together(v₂, … together(vₙ, context)))` and nothing else.

**A part is not a thing in the term** — it is the scope its voices' facts are constructed at — so there is no third
combinator and no pass.

### What the structure is for is scope

`notation.rs` refuses seven statements: four as misplaced, because "from here onward" has no unique meaning in a value
usable at several places, and three at a node labelled "a voice to belong to". Both refusals are one missing thing.

Supplying it **widened the reading rather than the statement table**: the distinguishing bit is `Reading::placed` and
not the scope, because a free `music { … }` value reads at `Scope::Piece` and so does a piece header — what separates
them is whether there is a single *here*.

Reading a voice is `notated(node, Reading::at(Scope::Voice { … }))`, and the four context statements that were
`misplaced` unconditionally now pick a scope where one is placed: a `key`, a `meter`, or a `tempo` written among a
voice's items is the *piece's* from there — per-voice meter is polymeter, which is a part's own header — and a `clef` is
the *part's*, because the reader whose hands change staff is one player.

### Numbering, extent, and provenance

Numbering is positional: a part's id is a running count over the score, taken only once its name is accepted, and a
voice's is its position among `PartDecl::items` *including* the `make` sites this prompt refuses, so the number a
written voice gets does not move when prompt 142 teaches the reading to expand its neighbour.

A header fact covers **D3's `max(d, e)` over the parts** and not `notation::extent` of the piece, because that function
sums — a block is a fold and a fold's extent is a sum — while `together(M, N) = (max(d, e), E ⊎ F)` inserts nothing into
the shorter track.

An unwritten meter is still a meter and carries `Origin::UNKNOWN`, which `provenance_at` already reads as "nowhere of
its own to point"; every written one carries its own statement's origin rather than the piece's, so a composer asking
where the 7/8 came from gets the line that says it.

A part's own clef, meter, and tempo are read through `resolve::part_facts` rather than off the nodes — the split
`part_context` was made out of, since that reading already carries the refusals a second `clef` and an unreadable meter
need — and `Lowering::heard` is the adapter that turns a reporting reading into the `Option` everything in `lower`
speaks.

The **survey** in `lower/piece/laws.rs` is the deliverable rather than a check on it: all fifty-four pieces in
`examples/`, read and checked with `document/laws.rs`'s standard-library corpus in scope, with exactly ten faults in six
classes left, every one of them a row in prompt 142's Target — and one of them, a notation statement whose argument is a
bound name, was a row 142 did not have until this survey found it.

**Owes.** Prompt 142 owns bar structure, the instance sites, the parameterized notation statement, and the pass that
calls this.

## Records and enums as surface syntax

> `record`/`enum` declarations, construction by field name, projection, `with` update along a path, record patterns, and
> `Type::Case` constructor paths.

`musa-syntax` · **implemented**

Keywords, CST nodes, typed AST wrappers, parser, formatter layout, semantic highlighting, keyword completion, and the
tree-sitter grammar and its six query files, held to the lexer by the drift law.

`musa-compiler` still reads the old `data`; a path update reaches its checker and is refused by name as an unimplemented
stage.

**Owes.** Prompt 137 adds traits and operators over the same forms; prompt 142 cuts the compiler over.

## The private marker

> One keyword before a `let`, `fn`, `record`, `enum`, `data`, or `structure`, and before an enum's cases.

`musa-syntax` · **implemented**

Implemented as a token inside the declaration's own node, so every accessor that declaration had reads the same node and
`is_private` is one child lookup.

Parser dispatch looks past the marker by one significant token. The marker on a non-declaration and the redundant marker
inside a `structure` body are each refused by their own message, and a refused program still round-trips.

Formatter, highlighting, keyword documentation, the generated UI tables, and the tree-sitter grammar and queries all
carry it.

**Owes.** Prompt 142 marks real packages; prompt 137 decides what the marker means on a `trait` or an `impl`.

## Module visibility

> One visibility boundary — the module — as a resolution filter: `private` on a declared family or on its cases, refused
> by name outside the module that declares it.

`musa-calculus` · **implemented**

Implemented over an opaque `ModuleId` the caller assigns, carried on a `Cx` and stamped on a declared group, compared
only for equality. A context naming no module is inside every module, and a declaration written in none hides from
nobody — which is what leaves every existing caller unchanged.

Three refusals: the private name; an enum marking some cases and not others; and a `match` outside the module on a
family whose cases are private — the last raised at the split rather than at the `match`, since an arm that only binds
never takes the type apart. The generated recursor is hidden with the cases.

**Owes.** Prompt 142 supplies the first real module identities; prompts 166–167 measure whether the marker earned its
keep.

## Records and enums, elaborated

> A record as a dependent record type with η, an enum as its own family with namespaced constructors, and `with` as one
> `let` and one literal per path segment.

`musa-calculus` · **implemented**

Implemented, including record patterns in case trees and a bare constructor read against the checked type by
[The constructor rule](#the-constructor-rule). Written where no type says which family it builds, it is refused naming
every family that declares the case.

**Owes.** Prompt 142 is the first caller.

## Traits and instances

> Traits and instances as dictionaries: `trait`/`impl` elaborated to a dependent record type and its values, coherence
> and the orphan rule checked at the declaration, `where` constraints on definitions, and §4's flat two-step resolution
> — an enclosing dictionary first, then one keyed table read.

`musa-calculus` · **implemented** · `10-traits.md`

A trait is the closed term `λp⃗. { … }`, so `Eq τ` is an application the existing evaluator β-reduces — no new `Term`
shape and no trait-specific conversion rule.

One `HashMap` keyed by (trait, head), where the head is rigid or a local type variable and a metavariable head is
deliberately not representable, so lookup cannot become a search. An instance's parameters are recovered by one hole per
parameter, solved by matching the instance's written head against the constraint being answered — `convert.rs`'s solving
mode, not a second matcher.

### Resolution is flat, and terminates because it does not recurse

§1's supertrait is `Refusal::SuperClass` at the trait's own declaration and an `impl`'s `where` clause is
`Refusal::ConstrainedInstance` at its, so no instance carries a constraint that resolving it could raise — and §4's
termination measure has nothing left to measure and is gone with the search it bounded.

Nothing is postponed either:

- A constraint whose head is a hole is `Refusal::Unsolved` at the site.
- One whose head is a canonical former no `impl` could ever key on is `Refusal::UnkeyedConstraint`.
- One at a bare type variable with no enclosing `where` is `Refusal::UnconstrainedVariable`.

Seventeen refusals, each with a program in `trait_laws.rs` that reaches it.

### A `where` on a definition rides on the binder

A `where` on a free definition, a `record`, or an `enum` is the third position `Constraint`'s doc always named, and it
rides on the binder rather than in a name-keyed table, because a definition is a value that can be passed, stored, and
returned with only its type in hand.

`Filling::Constraint(Arc<Constraint>)` marks the Π binder, no core rule reads it, and elaboration asks the question at
the two places it already asks how a binder is filled — filling one by §4's resolution rather than by a metavariable,
and abstracting over one by writing the λ *and* discharging the key, so §4 step 1 finds the binder standing where it was
written.

A family's constraints are ordinary parameters appended after the written ones, so none of `Group::params`'s arithmetic
moves and `Storable` is unaffected — a dictionary is a parameter, never a stored field.

**Owes.** Prompt 137a spells it on the surface; prompt 142 is the first caller; prompt 164 collapses the builtin
registry into it.

## Method resolution

> Exact-receiver method resolution: `x.m(…)` resolved from the head of the receiver's type against the traits that
> declare `m` and have a dictionary at that head.

`musa-calculus` · **implemented** · `10-traits.md` §6

Implemented as one raw shape holding the receiver and the method and no arguments, so `x.m(y)` is that applied through
the ordinary application rule and a resolved call is *the same term* as `Class.m(x, y)` — the law is convertibility, not
a snapshot.

Candidates come from a method-name index rebuilt with the trait map, intersected with the same keyed instance read §4
step 2 already was, so neither operand is a scan.

Candidates come from **two tables**, both read at every call: the definition `Head.m` in scope, and the field `m` where
the receiver's type is a one-constructor family that names one. A field reading is the accessor term
`record.rs::projection` builds, factored out and shared, so `x.m(y)` and `(x.m)(y)` are the same term for the same
reason `x.m(y)` and `Head::m(x, y)` are.

Four refusals: a receiver whose type has no rigid head, no candidate, two candidates in one table, and one candidate in
each.

**Owes.** Inherent `impl T { … }` blocks are prompt 142's; the surface spelling is
[Traits and operators as surface syntax](#traits-and-operators-as-surface-syntax).

## Traits and operators as surface syntax

> `trait`/`impl`/`where`, §1's six precedence levels for `== < + - * /`, `x.m(y)`, and `xs[i]`.

`musa-syntax` · **implemented** · `01-surface.md` §1

Implemented as three keywords, four new tokens, and one operator climb whose table is a constant per level, so there is
no place for a user-defined symbol to be added.

A note statement reads its pitch through a separate entry point at levels 1, 4, and 5, which is why `c5 up 2 /4` is
still a transposed quarter note rather than a division.

`a.b(c)` at a name receiver stays a `NameExpr` plus an application — the parser cannot know whether `low` is a module —
so `MethodCallExpr` exists only for receivers no name can spell.

**Owes.** Prompt 142 is the first caller; the elaboration that reads these nodes is prompt 142's.

## Storable

> The `Storable` constraint, generated rather than written.

`musa-calculus` · **implemented** · `02-core-calculus.md` §1.2

Implemented as **one structural predicate, not a trait**. Note 50 replaced the generated instance table with
`storable::is_storable`, a walk over the *value* a type evaluates to, run where the language actually needs the fact — a
machine port's signature (§2.3) and nowhere else.

`requiring_storable` is how a host writes the constraint into a machine constructor's scheme, as a constrained Π over
the reserved name whose dictionary is the empty record, and `dictionary.rs` discharges that constraint by computing the
walk rather than by keying a table.

The walk asks the same two questions of each stored field — is it finite at every depth (no Π and no universe anywhere
inside), and is its head a family that is storable in turn — and a family already being asked assumes itself storable,
which is §1.2's "checked once per declaration group" read coinductively.

There is no `Storable` entry in any `Classes`, no generated instance, and no `Cx::declaring` fixpoint. A source
`trait Storable` is `Refusal::ReservedClass` and a source `impl Storable` is `Refusal::HandWrittenStorable`, behind
every spelling.

**Owes.** Prompt 164 retires `musa-compiler`'s `d` type-variable class against it; prompt 169 owes the payload-boundary
re-derivation.

## The syntax index

> The syntax index `Syntax<Cat>`, and the phase's kind and delimiter as real types.

`musa-compiler` · **implemented** · `11-quotation.md` §1, §4

Implemented in the expansion phase's own checker and evaluator. Two categories, with `Expr` reached only by
`as_expression` and `checked_expression` — the two operations that run the real parser — and forgetting as one
directional arm in `reconcile` rather than an operation an author writes, since `unify` is symmetric and the reverse is
the uncertified splice the index exists to refuse.

`TokenKind`'s case set is generated from `musa-syntax`'s kind table by a `stringify!` macro, so the name and the kind
cannot disagree, and a drift test holds the two sets equal; `Delimiter` is the four the fixed grouper knows. Their
constants are read as the flat names `TokenKind.Integer` and `Delimiter.Braces` that `qualified_name` already builds, so
no namespacing feature was added to a checker prompt 142 deletes.

A syntax value cannot say which category it has — the claim is erased — so `infer::admits` treats any two as equal.
`Derived { origin, quotation, path }` names what `syntax_built` already computed.

**Owes.** Prompt 139's quote is the replacement for the seven construction operations and prompt 166 is where the last
caller stops using them; prompt 164 collapses `token_kind_equal` and `delimiter_equal` into `Eq`; the entry-by-entry
survey is [note 45](../../notes/research/language-design-closure/45-phase-registry-survey.md).

## Quotation as a written form

> `quote at here { … }` with `$x`, `${ e }`, and `$..xs`, and provenance minted rather than allocated.

`musa-syntax`/`musa-compiler` · **implemented** · `11-quotation.md` §2, §3

The body is read by the real parser — the splice forms are three productions in the ordinary expression grammar, gated
to quote bodies by a parser flag, so there is no template dialect and no second parser.

The phase elaborates a quote into a construction over the same builders an author no longer writes: a checking form
only, refused where nothing says which category it builds; a splice checked at the position against `Syntax<Expr>`, with
`$..xs` admitted only where the grammar has a repetition and the separators supplied by the position rather than by the
author.

Hygiene is renaming — a quoted binder and its quoted uses are one generated name, a spliced name keeps the binding it
arrived with, and a quote that writes a spelling the printer could generate is refused where it is written.

Every literal node gets `Derived { origin, quotation, path }` from the anchor, a per-site quotation number, and its
position in the quote's own tree; a spliced node keeps its arrival identity. `path` is never empty: both walks start one
step in, since an empty path would restate the origin and put every site's outermost node at one address. The
construction is charged to the expansion budget where it is built.

**Done.** Prompt 166 rewrote `stdlib/src/adapters/staff.musa` on it and measured: the emitting section fell from 260
lines to 156, and the fifty-six `syntax_built` calls and twenty-seven role integers are gone. The trial's construction
program compiles today as `tests/fixtures/staff-construction.musa`; the measurement is
[note 60](../../notes/research/language-design-closure/60-the-staff-rewrite-measured.md) §§1–2.

## Quote patterns

> `quote { … }` as a pattern.

`musa-syntax`/`musa-compiler` · **implemented** · `11-quotation.md` §4

One grammar serves both directions: the pattern form is the quote-body production without the anchor, because a pattern
derives no identity — it builds nothing for one to be derived from — and `${ e }` is refused in the elaborator rather
than by a second grammar that would have to be kept in step.

A pattern is read at the scrutinee's category, so the category has to be known and a type variable is refused there;
`$x` binds `Syntax<Cat>` and `$..xs` binds `List<Syntax<Cat>>`, and every other word is matched as written, with a name
the arm then uses reported as the missing `$` rather than as an undeclared name.

### Matching is modulo three things and nothing else

- Trivia, which a read region keeps and a template dropped.
- The parser's unary layout bracketing, peeled on both sides, since `Layout[x, +, y]` is real shape and `Layout[x]` is
  not.
- A comma-separated position's own separators, which the position supplies rather than the author.

Provenance is not shape at all, so a node a composer wrote and a node a quote built match one pattern — the property
adapter expansion rests on, and the reason there is no way to ask which.

A spread may stand among any group's children, which is *wider* than construction, where one needs the comma its
position supplies: reading a run needs only a run to bind. At most one per group, since two would leave the split
between them a search.

Coverage is `Shape`, keyed on the template — like a literal it constrains without enumerating, so a match of shapes
still needs the arm that says what the adapter reads, and two identical shapes are one arm.

**Done, and the one section that got bigger.** Prompt 166 rewrote `stdlib/src/adapters/staff.musa` and measured: the
reader's three sections went 513 lines to 612, because the evaluator charges every level of an `if` chain to every token
that reaches it and the file now classifies in two stages to keep the page inside its budget —
[note 60](../../notes/research/language-design-closure/60-the-staff-rewrite-measured.md) §3 is the cost model and the
finding. The trial's dispatch program compiles today as `tests/fixtures/staff-dispatch.musa`, and it contains no quote
pattern at all — which is [note 43](../../notes/research/language-design-closure/43-dependent-language-trial.md) §2's
own finding about where the form belongs, since a pattern is written in Musa and staff notation is not.

## Quotation in the core

> A quotation as a core term: the template a literal, instantiation and matching δ-rules.

`musa-compiler` · **implemented** · `11-quotation.md` §2–§4, `02-core-calculus.md` §5.8

A `crate::quote::Template` is a **literal of an inert base type**, which is §5.8's D1 answered the way `Syntax` itself
was: no eliminator, no source program takes one apart, and two agree exactly when the host says they do.

The quotation counter rides on the literal beside the body, because it is a property of the construction *site* — two
quotes with identical bodies at one anchor must still build distinguishable nodes (§3).

Four δ-builtins in neither ownership table, since those two are the old checker's and a row in `SYNTAX_OWNERSHIP` would
make `instantiate_quote` a word an adapter could write: `instantiate_quote` calls `crate::quote::instantiate` and the
three pattern operations call `crate::quote::matched`, so derived identity is computed once and reached a second way
rather than reimplemented — which is §5's second-path audit applied where two answers would be worst.

The splices arrive as a list of lists because a hole is a run or a node and the core has no sum of the two a lowering
could write.

The pattern side is `match_quote`/`quote_hole`/`quote_holes` and a chain of `Bool` matches rather than one `Option` of a
list of lists, for two reasons: `Registry::check_finite_data` admits a base type at a *literal* index and refuses one at
a variable, so the holes are read at `⟨tokentree⟩`; and `Elaborator::definition` infers an unannotated `let` where a
`match` has no inference rule, so a destructuring's continuation could not be bound to a name and would be copied into
every coverage hole §6.2 demands.

`lower/quotes.rs` carries the body walk the old checker still has its own copy of, and the law beside it is what holds
the two together: five bodies, lowered and normalized, build the *same* `Syntax` the old checker and evaluator build,
path for path.

**Reached, and the two prompts that reached it.** Prompt 142 deleted `ExprKind::SyntaxQuote` and the checker's copy of
the walk — `ExprKind` has one mention left in the crate, inside a comment about its own removal. It also supplied §1's
forgetting as an *acceptance* rule, which prompt 159 then deleted as the subtyping it was: forgetting is a `SyntaxOp`
row spelled `forget`, written at each site where a category is dropped, and `lower/quotes.rs` writes it for
`match_quote`'s scrutinee where the lowering used to have it inserted behind its back.

## A diagnostic carried whole

> A diagnostic about a document the composer did not write, carried whole.

`musa-compiler` · **implemented** · [`../../rules/desktop/05-states.md`](../../rules/desktop/05-states.md) §5

Implemented as `Cause` — document key, code, message, labels, note, help — and `Diagnostic::causes`, attached by
`level_of`'s `Broken` arm through all three of its callers.

Spans in a cause's labels are spans in its own document and in no other file, which is what keeps `remap_spans` correct
without a runtime check: the source map moves the composer's own text, and a cause is not in it.

A cause carries no fixes, for the reason `remap_spans` already gives about generated text, and holds no causes, because
the only producer is a module read and a module cannot import. Nothing is spliced into the wrapper's message; the six
codes only ever raised inside an adapter module now reach their author with the note and the help they were written
with.

**Owes.** Prompt 165 owns the wording. `Code::Import`'s `` `{path}` does not compile `` is the same shape on the library
path and is named in [note 47](../../notes/research/language-design-closure/47-diagnostics-about-another-document.md) §4
as the next application.

## Causes restated for the session

> Causes restated for the session, with line and column in *their* document.

`musa-project` · **implemented**

Implemented as `Cause` and `CauseLabel` beside the existing restatement, resolved by `Diagnostic::from_compiler` out of
the `ImportSources` the session already holds — so the layering rule is unchanged: the compiler measures bytes, the
session counts lines, and a `Position` is still never derived by the frontend.

A `CauseLabel` deliberately carries `at`/`to` and no span, because a byte range into a file the frontend does not hold
has no use and one misuse. A cause whose document is not in the map keeps its label texts without positions rather than
being dropped, and `ProjectSnapshot::cause_source` hands the text to a renderer that wants a caret.

**Owes.** Prompt 142 supplies the first real adapter packages to fail this way.

## Causes as related information

> Causes as related information at their own URI.

`musa-lsp` · **implemented**

Implemented over `DiagnosticRelatedInformation`, whose `Location` already carried a URI — the only reason it was always
this document's is that nothing upstream could say otherwise.

A resolved key that is a real path becomes a `file://` URI with the cause's message, note, and help folded in the way
help is already folded into the primary's. A key with no file behind it, and a cause with no labels at all, fold into
the primary message instead of being dropped, because a composer who cannot click through still has to be told.

**Owes.** Prompt 165 owns the wording.

## Causes in the problems list

> Causes in the problems list.

`apps/musa-desktop/ui` · **implemented** · `05-states.md` §5

Causes list under the diagnostic they caused, indented past its own detail, each with the document's last path segment
where the location goes and the position within it beside it.

Not a navigation target and no fix control, because the composer cannot edit that file — the shape `08-elaboration.md`
already fixes for text a composer does not own.

**Owes.** A read-only package viewer is deliberately absent and stays absent until a prompt argues for one.

## Collections

> `List`, `Buildable`/`Iterable`, `map`/`filter`/`fold`/`collect`, and total indexing.

`musa-syntax`/`musa-calculus` · **partial** · `01-surface.md` §1.6

The surface half — `[a, b, c]` in the grammar, CST, AST, formatter, highlighting, and the tree-sitter grammar with its
drift test — predates the language pass and is unchanged.

The library half is written *as a program*, in `collection_laws.rs`: `List`, `Option`, and `Bool` as ordinary
declarations; `Buildable<C, A>` and `Iterable<C, A>` as traits whose `map`, `filter`, and `collect` are derived methods
over one shared fold body; and `Index<C, I, R>` answering in an `Option` rather than acquiring a partial operator.

`musa-calculus` ships none of them: base types and builtins are registered by the host rather than declared here, and
`Storable` is pre-declared only because the check is the evidence.

`push` on a list is snoc, which is what makes `collect` the identity rather than a reversal, and what makes it
quadratic.

**`Vec A n` does not ship** — no program asks for a length in a type;
[note 46](../../notes/research/language-design-closure/46-collections-and-the-vec-answer.md) records the answer and the
condition that re-opens it.

**Owes.** Prompt 142 writes the same declarations in `.musa` and hands them to authors; prompt 166 measures what a
forward-accumulating traversal is worth against the reversed reading algorithm it replaces.

## Per-term provenance

> An origin on every core term, preserved by evaluation and quotation and invisible to conversion.

`musa-calculus` · **implemented** · `02-core-calculus.md` §7

Implemented as an opaque `Origin` on `Term`, on values, and on context assumptions; carried through elaboration and
named by both sides of a conversion mismatch.

**Owes.** Prompt 138 relates it to the compiler's `Derived` graph.

## User-defined nominal data

> User-defined nominal data, private constructors, and abstract type members.

`musa-compiler` · **absent**

Absent here. The first two are spelled in `musa-syntax` and enforced in `musa-calculus` by
[The private marker](#the-private-marker) and [Module visibility](#module-visibility), and what is missing is the
compiler wire-up that gives a module a real identity.

An abstract *type member* — a type whose definition is hidden — remains a separate feature with no program asking for
it.

**Owes.** Prompt 142 cuts the compiler over and supplies the identities.

## Inductive families with indices

> A type constructor whose constructors choose their indices, eliminated by a dependent motive, with `match` compiled to
> a case tree and refinement decided by unification.

`musa-calculus` · **absent** · `02-core-calculus.md` §1.1

What exists is the non-indexed half: `family/` declares parameterized families and generates eliminators, and since
prompt 155 the motive `family/assemble.rs` builds is a *family* — so a `match` refines by its pattern, and what is
missing is only the index for a constructor to choose. `base.rs` already applies a base type to arguments — `Syntax` to
a category literal, `EventTrack` to its coordinate — which is the shape an index takes, without the refinement.

The erased index stratum that stood here is **reversed**. `index.rs` and its decision procedure were built at prompt
142d against `02-core-calculus.md` §1.5, and prompt 143's amendment retired both: the erasure made §3's conversion rule
false about the implementation, and the arithmetic the stratum uniquely bought had no user in any committed `.musa`
file. The row that described it is [`../prompts/142c-index-amendment.md`](../prompts/142c-index-amendment.md), which now
carries a reversal banner.

**Owes.** Prompt 151 deletes `index.rs`, the `Indexed` shape, and the conversion hook, turning an indexed type into an
ordinary applied type constructor. Prompt 155 reified the case tree and made the motive dependent; prompt 155a makes a
tree a definition body; prompt 156 gives constructors their indices and the unification that decides which branches are
reachable. Prompt 164 discharges the seventeen `pc12_*`/`row12_*` builtins the original amendment was granted on.

---

## Recommended implementation order

1. Carry out the clean break of prompts 127b–127d, 142, and 171–174: the event-track rename and coordinate index, the
   deletion of the contextual `music` type, machines as core values, and `schedule`.
2. Finish or reject the small source-language design for theory-owned data. Do not implement it while stable package
   selection remains undefined.
3. Add the gesture event track using the now-implemented payload admission rule.
4. Implement audio preparation with the complete argument list.
5. Replace caller-buffer-based feedback with the whole-machine step order and the one-frame step.
6. Add the versioned representation registry and complete origin paths.
7. Run the repaired audio and whole-language conformance prompts before public release.

None of these steps requires call-by-push-value, dependent track types, first-class “worlds,” or one common musical data
model.
