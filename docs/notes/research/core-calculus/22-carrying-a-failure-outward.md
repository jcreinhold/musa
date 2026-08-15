# Carrying a failure outward without a staircase

Prompt [127dcfad](../../../plan/prompts/127dcfad-result-question.md) added postfix `?`. This note records what it was
measured against, what it does *not* mean, and the one place the implementation had to decide something the prompt left
open.

## The staircase that motivated it

`document_read` in `stdlib/src/adapters/staff.musa` asks six questions in a row — was the region refused, does a tie
hang, is there a head, does the clef name a clef, and two more — and before this prompt every answer that was not a
failure indented the next question one frame further. The failure arms were all the same line written six times, and the
actual work sat at the rightmost leaf.

|  | before | after |
| --- | --- | --- |
| `document_read`, lines | 115 | 61 |
| `document_read`, deepest indent | 56 columns | 32 columns |
| `match` expressions inside it | 16 | 4 |
| `staff.musa`, lines | 2041 | 1987 |

The four `match` expressions that remain are the ones that *decide* something rather than propagate: they read a head,
and they choose between two shapes. The twelve that went are the failure arms and the success arms that existed only to
bind a name for the next question.

`stated_field` changed type as part of this — from an `Option`-shaped answer to `Result<Syntax, (Syntax, Text)>` — so
that its absence carries the node it is absent from, which is what prompt 127dcb makes part of an adapter's answer. That
retyping is why the six questions became askable at all; a `?` needs a `Result`, and a missing field that says only
"missing" is not one.

These numbers are recorded here so prompts 127dcfae and 127dcfag can attribute what they measure to the traversal change
rather than to this one. Note 39 §7.2 assigns five of the staff adapter's six pain causes to ergonomics; this is the
third of the three ergonomics repairs, and after it the remaining pain is the fold.

## The elaboration

Writing `C` for the answer the question was asked inside of:

```text
⟦C[e?]⟧  =  match ⟦e⟧ { Ok(x) → ⟦C⟧[x], Err(y) → Err(y) }
```

`x` and `y` are binders no source can name — they are spelled with a leading space, which no identifier may start with —
so the erasure needs no renaming side condition and a right-hand side written near a `?` cannot capture them.

No core term, typing rule, reduction, normalization case, or cost-table entry was added. The evidence for that is a
test, not the claim: `a_question_is_the_result_match_it_elaborates_to` compiles a chain of two questions and the nested
match an author would have written by hand, and asserts the two agree on every value *and* on `charged_nodes`. Equal
charge against a match whose scrutinee is visibly evaluated once is what says the question evaluates its subject once
too — a second evaluation of that test's eight-member-list subject would be several nodes wide and the meter would say
so.

## Where a question may be asked, and why that is not an annotation requirement

`?` needs somewhere to carry a failure *to*. Musa has no `return`, so the place is not "the function" in general but the
expression whose value **is** the function's answer: a function or lambda body, and a match-arm or `if`-branch body in
tail position within one. A `?` written anywhere else — inside an argument to a call whose result is the answer, say —
is refused, because elaborating it would need an early exit and this prompt forbids one.

That is a smaller allowance than it first looks, and it was tested rather than assumed: `document_read`'s six questions
are all in tail position, because propagation is what tail position is for. A question written to be *handled* rather
than propagated is a `match`, which the language already had.

The enclosing answer's type may be inferred. `?` unifies that answer with `Result<β, E>` for its subject's `E` and lets
the unifier discharge it like any other constraint, so an unannotated named function and an unannotated lambda both
infer through a question. This matters beyond convenience: "principal rank-1 inference, annotations optional" is a
language-wide sentence, and a construct that demanded an annotation would have made it false.

The four refusals each say which of the four things went wrong:

- the subject is not a `Result` — and if it is an `Option`, the diagnostic says so by name, because that is the mistake
  a reader of another language will make;
- the answer around the question is no `Result` — "this answers `Nat`, so a failure has no way out of it";
- the answer fails a different way — "a failure of `Result<Nat, Text>` cannot leave an answer of `Result<Nat, Nat>`",
  naming both, because a coercion is exactly what is being declined;
- the question is not in the answer's tail — "`?` has nowhere to carry a failure to".

Each points at the question itself, not at a match nobody wrote.

## What this is not

**Not a monad, and not evidence for one.** Note 39 §6.1 says the evidence names one constructor used twice, not one
algorithm run over two constructors. Adopting `bind` because `Result` happens to be a monad is the analogy-first move
`constitution.md` §9 forbids, and it would need higher-kinded variables the language does not have. A second constructor
that later needs propagation earns its own operation, or supplies the evidence note 39 §6.4 asks for.

**Not an error-type conversion.** The two error types must be identical. A function that produces a different failure
maps it at the call site, which is what keeps the failures a function can produce readable from its signature.

**Not an exception, a handler, an effect, or an early return.** The elaboration is local and the language stays an
expression language.

**Not available on `Option`.** An `Option` has no error to carry outward, so there is nothing for `?` to mean there.
Converting is a named operation, which forces the author to say what the missing case means.

## One deviation from the prompt's Design

The prompt's Design says `?` is rejected "only when the enclosing result cannot unify with `Result`". The implementation
rejects one further case: a `?` whose enclosing branch is not in the answer's tail. The prompt's own **Stop** forbids an
early-return statement, and without one there is no way to elaborate a question in a non-tail position — so the extra
refusal is what the Stop list implies rather than an addition to it. It is named here because the two sentences read
differently, and a later reader should not have to rediscover why.
