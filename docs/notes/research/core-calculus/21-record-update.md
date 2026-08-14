# Rebuilding a record by naming only what changed

**Status: research. These notes do not set Musa's rules.**

Prompt 127dcfac added `subject with { field = expr, … }`, an immutable update of the named fields of a nominal record.
This note records what the standard library looked like without it, what it looks like with it, and the two places where
the implementation differs from what the prompt's **Design** predicted.

The short version: **the update adds no core term, and it removed five of the seven functions that existed only because
it was missing.**

## 1. What was wrong

`stdlib/src/adapters/staff.musa` declares `data Pending` with eight fields and then wrote seven functions —
`holding_body`, `holding_length`, `holding_dots`, `holding_tie`, `holding_numbers`, `holding_voices`, `holding_word` —
whose entire body was a match destructuring all eight fields and a constructor naming all eight, one of which differed.
Each was between six and twenty lines, and none of them said anything a reader could not have derived from its name.

That is the cost Ousterhout ch. 8 describes, itemized: a convenience the core does not offer is paid once per author,
and here one file paid it seven times. It is also invisible from the type system's side — every one of those functions
type-checks, and nothing about `Pending` is wrong. What was missing was a way to *say* "this one, with that field
replaced."

## 2. The measurement

Counted against the commit that landed prompt 127dcfab, so the `if` conversion is not credited here.

|  | before | after |
| --- | --- | --- |
| `fn holding_*` in `staff.musa` | 7 | 2 |
| explicit `Pending(…)` constructions | 13 | 5 |
| lines in `staff.musa` | 2116 | 2041 |
| `with { … }` updates | 0 | 8 |

The prompt's **Task** said seven fields and twenty-four constructions. The declaration has eight fields, and the honest
construction count — excluding the declaration itself and the patterns that merely spell `Pending(…)` on the left of an
arrow — was 13. The prompt was counting textual occurrences. The measurement is recorded here rather than the claim,
which is why prompts 127dcfae and 127dcfag can attribute their own improvements to the right cause.

Two `holding_*` functions survive because they still earn their names. `holding_length` keeps its deliberate use of
`faulting` — a second length is a diagnostic, not an overwrite, and that is behavior rather than boilerplate.
`holding_word` chooses between `OneWord` and `TwoWords` depending on what is already there. Both bodies are now a match
on the one field they care about with an update in each arm, and neither destructures anything it does not use.

## 3. It elaborates to a constructor application

`p with { f = e }` becomes a one-arm `match` on `p`. The pattern is the sole constructor of `p`'s declaration binding
every field to a fresh binder; the body is that same constructor applied to `e` in `f`'s position and the corresponding
binder in every other. No core term, typing rule, reduction, normalization case, or cost-table entry was added.

Three properties fall out of the shape rather than out of a rule stated on top of it. The subject appears once, as the
scrutinee, so it is evaluated once however many fields are carried over. Exactly one constructor introduction appears,
so an update charges exactly one construction, which is what prompt 127dcec's meter requires. And the binders are
spelled `" field 0"`, `" field 1"`, … — a leading space is not a valid identifier, so no source name can capture them,
and a right-hand side naming `dots` reads the `dots` in the surrounding scope rather than the field of that name. The
second part of the semantic contract is therefore true by construction, and needs no renaming side condition in the
erasure.

## 4. Refused, and why the refusals held

**Row polymorphism and structural records.** Both make "a record with at least these fields" into a type, which is a
different language and a different inference problem. The update here is nominal throughout: the subject's type
determines the declaration, the declaration fixes the field set, and an unknown name is a resolution error that names
the declaration and lists what it does have.

**Generated per-field setters.** A generated `with_dots` is the `holding_*` pattern moved from the library into the
compiler, where it multiplies with the field count instead of disappearing. It would have made the measurement in §2
look identical while making the problem worse.

**Lenses.** Composition over a field-access value needs a type this language cannot write, and would have reopened the
container question note 39 §3.3 closed.

**Mutation, nested-path update, and update through a sum.** A record with more than one constructor is taken apart with
`match`, which names the case, and rebuilt inside the arm — the case analysis is the information, and an update that
skipped it would have to fail at run time in a language that has no run-time failure. `p.a.b = …` is written as nested
updates, which is one more line and no new rule.

## 5. Two deviations from the prompt's Design

**Right-hand sides evaluate in declaration order, not left to right.** The prompt fixed "left to right"; the
implementation checks each right-hand side where it is written — so a type error points at the line the author wrote —
and evaluates in the declaration's field order, because that is the order a constructor written with named arguments
already evaluates in. Nothing observes the difference: the language is total and its expressions have no effects, so no
program can tell the two orders apart. Making update the one form with its own evaluation order, in order to match a
sentence, would have been the drift this repository forbids in the other direction.

**The elaboration binds by pattern, not by projection.** The prompt's **Design** says the constructor is applied "to
projections of the subject for the rest." There are no projections:
[`02-core-calculus.md`](../../../rules/language/02-core-calculus.md) §5 states that products have introduction but no
surface projection. Pattern binding *is* how a field is read in this language today, so the elaboration uses the
eliminator that exists. If a projection form is ever added, the elaboration could be restated with it and would mean the
same thing.

## 6. The spelling

`with` was already a keyword: `use theme() with { note 3 = a5; }` is the music statement's occurrence-override clause,
and `examples/variation.musa` uses it. Rather than spell the update differently, both parsers were taught which claim
wins. The hand-written parser carries a `with_is_spoken_for` flag that `use_stmt` sets while reading its own expression
and that `expr` takes at entry, so only the *top level* of a `use` value stands down and an update written inside an
argument is an ordinary update. The tree-sitter grammar says the same thing by declaring
`[$.expression, $.record_update_expression]` in `conflicts` and letting the GLR parser resolve it — precedence numbers
were tried first and could not express it, because the ambiguity is about which enclosing rule the `with` belongs to
rather than about how tightly it binds.

The reading a statement's `with` already had is the one it keeps. That is the whole argument for the disambiguation
going this direction: an author's eye puts `with { note 3 = a5; }` on the `use`, and the parser should agree.

## 7. What would reverse this

A declaration with two constructors that authors genuinely want to update without naming the case. Today that is a
`match`, and the case analysis is information the author should be made to write. If a pattern emerges where the case is
already known from context and restating it is pure noise, the question is worth asking again — but the answer would
more likely be a better way to *name* the known case than a way to skip it.
