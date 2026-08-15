---
id: 131
slug: quotation-spec
status: done
depends_on: [130]
phase: 3
---

# Specify Typed Quotation, Splicing, and Syntax Patterns

## Task

Add `docs/rules/language/11-quotation.md`: `Syntax<Cat>` as an indexed inductive family, `quote at here { … }` with
value and sequence splicing, provenance that the elaborator computes instead of the author allocating by hand, and the
inverse form — quotation as a pattern — for taking syntax apart. This is the piece that decides whether Musa is
homoiconic in practice or only in principle.

## Read

- `stdlib/src/adapters/staff.musa`, the whole `// ---- writing` section and `call1`–`call7`. Every line of it is the
  specification's motivation: a nested `syntax_group`/`syntax_token` tree written by hand where the author meant
  `Sounded(anchor, event, items)`, and `syntax_built(here, 53, 0)` with about fifty hand-allocated role integers whose
  uniqueness nothing checks.
- `docs/rules/language/00-semantics.md` §2 — the phase environment `Σφ`, the phase-local types, and the rule that
  descent into a syntax value happens in exactly one place. A pattern form is a second way in, so that rule is one this
  prompt has to either amend or satisfy, and saying which is half the work.
- `docs/rules/language/01-surface.md` §7 — the typed-antiquotation precedent, and specifically its four writer's rules:
  a quote is commented like the file around it, a raw payload says what the material is and not where it goes, the
  quotation locus is where a hole is *instantiated*, and time operations move occurrences rather than rewriting
  payloads. The first and third generalize; the second and fourth are kernel-specific.
- Prompt [127da](127da-path-aware-syntax.md) (derivable paths and pure builders), [127db](127db-derivation-graph.md)
  (origins as a grafted finite graph), and [127dcfaf](127dcfaf-syntax-step-recursor.md) (the inherited-context
  recursor). This prompt keeps the recursor and deletes the builders' hand-written call sites, so it has to say exactly
  which of their laws survive.
- `docs/rules/across-stages/04-identity-and-realization.md` — derived identity is an across-stage contract, not an
  adapter convenience, and a quotation that mints provenance has to mint it in that vocabulary.
- `docs/rules/language/02-core-calculus.md` (as rewritten by 129) §1 for inductive families and §7 for term provenance,
  and `10-traits.md` (as added by 130) for how `Cat` is constrained.

## Design

**`Syntax<Cat>` is an indexed inductive family.** `Cat` ranges over `Expr`, `Item`, `Pattern`, and `TokenTree`. Today
`Syntax` is one untyped type and every adapter re-checks by hand that what it built in an expression position is an
expression; the index makes that a type error instead of a runtime refusal. This is the first real use of 129's indexed
families and is deliberately chosen as such: if the index does not pay for itself here, the paper trial in 132 will say
so before any code is written.

**`quote at here { … }`.** The quote's body is Musa surface syntax of the category the expected type demands, and it
elaborates to a `Syntax<Cat>` construction. `$x` splices a `Syntax<Cat'>` value at a position whose category is `Cat'`;
`$..xs` splices a sequence where a sequence is grammatical. Category mismatch is a compile-time error naming both
categories. The quote is parsed by the real parser — not a template dialect, not a string — because a quotation that
does not share the parser is the "sublanguage by subtraction" root `AGENTS.md` forbids.

**Provenance stops being an author's job.** Spliced syntax keeps the identity it arrived with. Syntax written literally
inside a quote is `Derived { origin, quotation, path }`, where `origin` is the `at here` node, `quotation` identifies
the quote, and `path` is the literal position within the quote's own tree. All three are computed by the elaborator.
That deletes the role integer entirely: uniqueness becomes structural rather than a convention, two occurrences of the
same shape inside one quote are distinguishable because their paths differ, and a rewritten adapter can no longer
collide two derived nodes by miscounting. State the identity law: two quotations at the same origin with the same path
are the same derived node, and nothing else is.

**Quotation as a pattern.** `match node { quote { $a + $b } => … }` binds `a` and `b` to sub-syntax. This is the inverse
of construction and it is what removes the string-dispatch table — matching against a quoted `TokenTree` decides a token
kind by its shape, not by `text_equal(kind, "PitchLiteral")`. Two rules keep it honest: a pattern quote binds only
splice variables (no literal-token capture by accident), and matching is on the syntactic shape, not on provenance, so a
derived node and a source node with the same shape match the same pattern.

**What survives of the sealed-step recursor, stated as a list.** `recurse_syntax` and `run_syntax_step` stay: generic
traversal over unknown syntax is a different job from matching a known shape, and prompt 127dcfaf's inherited-context
design is the right answer to it. `SyntaxStep` stays non-storable. What does not survive: `call1`–`call7` and every
hand-written `syntax_group`/`syntax_token` assembly, the role-integer argument to `syntax_built`, and the string
dispatch on token kinds and delimiters. Prompt 138 replaces the last of those with typed `TokenKind` and `Delimiter`;
this document says the surface no longer has a reason to look at either.

**Two quotations, one discipline, and they are not merged.** `kernel T { … }` builds an event-track term in the
elaboration stage; `quote at here { … }` builds `Syntax<Cat>` in the expansion phase. `02-core-calculus.md` §6.1 keeps
those two stages apart, so this document states the shared rules once — typed holes, no capture between quoted and host
identifiers, the completed quote must close and check before it is used, the locus is where a hole is instantiated — and
says explicitly that the two constructs share those rules and not their type. A later reader who tries to unify them
should find the refusal already written down with its reason.

**`00-semantics.md` §2 needs one amendment**, and it belongs in this commit because it is the same decision: descent
into a syntax value now happens in two places, `recurse_syntax` for unknown shape and a pattern quote for known shape.
Amend the sentence, keep the reason it existed — the phase must not grow a second uncontrolled traversal — and state
what makes the pattern form controlled: it destructures one level of a shape written in the source grammar, and it
cannot observe provenance.

## Target

- `docs/rules/language/11-quotation.md`, new: `Syntax<Cat>` and its index; the construction form and its category rules;
  value and sequence splicing; the derived-identity law and its uniqueness argument; the pattern form and its two rules;
  the surviving-recursor list and the deleted-mechanism list; the shared-discipline section that keeps kernel quotes and
  syntax quotes distinct; and the diagnostics each failure mode owes.
- `docs/rules/language/00-semantics.md` §2: the single-descent sentence amended as above, with the phase-local type
  `Syntax` restated as `Syntax<Cat>`.
- `docs/rules/language/01-surface.md` §7: a forward reference to `11-quotation.md` for the shared rules, and nothing
  else — §7's kernel-specific rules stay where they are.
- `docs/rules/language/README.md`'s document map, and `citations.md` for the new claims (typed quotation, hygiene,
  splicing categories).

## Check

```sh
python3 scripts/renumber-prompts.py audit
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
/Users/jcreinhold/.cargo/bin/mdwright check docs/rules docs/plan
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
git diff --check
```

Commit as `Specify typed quotation and syntax patterns`.

## Stop

- No code, no grammar, no fixture, no `stdlib/` change.
- No merging of `kernel T { … }` with `quote at here { … }`, and no third quotation form.
- No unquoted-string macro, no procedural macro over token streams, no runtime `eval`, and no way to observe provenance
  from a pattern. Each of those is a hole in the phase boundary `00-semantics.md` §2 exists to hold.
- No typed `TokenKind` or `Delimiter` definition. Prompt 138 owns the phase API's types; this document may name them.
- No change to `docs/rules/across-stages/04-identity-and-realization.md`. If derived identity as specified there cannot
  express `Derived { origin, quotation, path }`, that is a governing-document repair and stop condition 4, not a quiet
  edit inside a language prompt.
