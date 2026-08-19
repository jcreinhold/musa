# The anchor's place, without a name supply

**Status: records a decision; governs nothing.** Written at the head of prompt 145, when the prompt's own mechanism for
the two-argument `syntax_anchor` met the registry it would run in and did not survive the meeting. The prompt's Design
is repaired to match this note; the decision itself is argued here once rather than restated there.

## The question

`docs/rules/language/11-quotation.md` §5 retires `syntax_anchor`'s third argument — the place its answer stands at — and
prompt 139 deferred the deletion to the prompt that removes the last caller, which is prompt 145. The answer token still
needs a `Derived` triple (`11-quotation.md` §3): the `origin` is the node the anchor is about (the second argument), the
`path` is `[0]` (a site that builds exactly one node), and the question is the middle component — which construction
site, when the operation no longer receives one.

## What the prompt's Design said, and what the code said back

Prompt 145's Design, as written, drew the site index from `Resolver::next_quotation`, "the same counter `syntax_quote`
draws from." That sentence was written against the old checker, which owns the counter and could see a phase call.
Implementation found two facts against it:

1. **The anchor executes as a δ rule, and a δ rule is a function pointer.** `musa_core::Rule` is
   `fn(&[Datum]) -> Option<Answer>` — values in, answer out. `registry/rules.rs`'s module documentation carries the
   reason on its face: "a rule cannot capture the compiler — no meter, no span, no diagnostic sink, no interning table.
   §5.8's D3 asks that 'the result is a function of the argument values alone', and that is a property of the type
   here." A rule has no counter to draw from and cannot be given one without changing the core calculus's own type —
   which is `docs/rules/across-stages/02-core-calculus.md` §5.8, amendment territory, and out of this prompt's reach.
2. **The adapter does not pass through the code that owns the counter.** Since prompt 142, adapter modules elaborate
   through the document path into musa-core; the compiler's lowering — the only places `next_quotation` is drawn
   (`core.rs`'s old `syntax_quote`, `lower/quotes.rs`'s new one) — never sees the adapter's `syntax_anchor` call. The
   counter the Design named is behind a door the caller no longer opens.

## What the references say

**Peyton Jones, ch. 9 (the type-checker).** Fresh-name generation is *explicit threaded state*: the name supply goes in
as an argument and comes back depleted, and where threading one supply is intolerable, disjointness is derived
*structurally* — his `split`/`deplete` makes the two halves of a split supply disjoint by construction (`+2` per step,
even and odd chains), not by a discipline anyone maintains. The lesson cuts directly: a counter you cannot thread is a
counter you cannot use, and the honest substitute is disjointness inherited from structure you already hold.

**Idris2.** Fresh names live in `UST.nextName` inside the `Core` monad — elaboration-time state. `Core.Normalise`, the
evaluator, has no name supply and mints nothing; synthesized terms carry positions the *elaborator* assigned (the FC
context), never positions the evaluator derived. Musa already follows this division exactly: quote site indices are
drawn at lowering time, from the resolver, in both checkers. The anchor is the one construction operation that runs on
the wrong side of that line — at evaluation, in `Normalise`'s seat, where there is no supply by design.

**Open Music Theory.** Not operative here. This change moves no notation semantics — the anchor is provenance plumbing,
and which node a bar line is stays exactly what it was. The theory reference earns its keep in the rewrite proper, where
the reading algorithm's correctness is the question.

## The options, weighed

**(a) Thread a call site through `musa_core::Rule`.** Change the signature to hand each δ call its origin, then derive
the index from that. Rejected: D3 is stated as a property of the type — "the result is a function of the argument values
alone" — and a call-site channel is a function of the *call*, not the values. One builtin's convenience is not worth a
hole in the core's determinism statement, and the change is the constitution's to make, not a prompt's.

**(b) Special-case the call at elaboration.** Teach musa-core's elaborator to recognize `syntax_anchor` and rewrite it
into an internal three-argument form with a site index baked in. Rejected: a hidden third argument is the
hidden-information failure the registry exists to name, and it builds a second, invisible mechanism beside quotation for
the same job. The trial's own falsifier — "a hand-written provenance path, or a role integer by another name — no" —
fires on a mechanism the author cannot see just as much as on one they must maintain.

**(c) Derive the place from the arguments alone — adopted.** The anchored node's own path is already a unique,
elaboration-time identity: no two nodes of a region share a reading-order path, by the same argument that makes anchor
*numbers* unique. So:

```text
Derived { origin: <the anchored node's path>, quotation: u32::MAX, path: [0] }
```

The `quotation` component is a **reservation**, not an allocation: the quote counters count upward from zero (`core.rs`
and `lower/quotes.rs` both), and `u32::MAX` is the value no quote can draw before the counter itself gives out.
Disjointness from quotes holds by construction — Peyton Jones's `split`, with the whole upper endpoint as one half. The
reservation is named once, in `syntax.rs` beside `Derived`, which owns the namespace, and both evaluator arms (the old
checker's and the registry rule's) build the place through it.

## The collision analysis, stated rather than assumed

Three pairs matter:

- **Two anchors of one node** produce one `Derived` — and collide, exactly as the prompt's Design requires: the
  one-call-per-anchor obligation stays, reported by `check_expression`'s duplicate-path gate in the same words. The
  obligation is now stated on the *anchored node* rather than the call site, which is the stronger and more honest form:
  the staff adapter's discipline is one `anchored` call per node read, and two sites anchoring one node were never a
  case it could use.
- **An anchor and a quote at one node** cannot collide: the quote's site index is drawn from a counter that starts at
  zero and increments; the anchor's is `u32::MAX`. Reaching it would take four billion quote sites in one module, at
  which point the module has worse problems than provenance.
- **An anchor and an input node** cannot collide: `Derived` paths carry only `PathStep::Built` steps, disjoint from the
  `Child` steps of input paths — the invariant `syntax.rs` already states at `Derived::path`.

What changes for an author: the workaround recorded in `tests/fixtures/staff-construction.musa`'s header — pass `here`
for the place, sound only for one call per node — becomes the law of the operation rather than a discipline around it,
and the twenty-seven hand-allocated role integers the staff adapter kept apart by hand are gone with the argument that
needed them.

What does not change: the anchor *number*. It is the node's position in the region's reading order, computed by the
reader, and this change touches nothing about reading order. Every anchor the staff adapter emits is the number it
emitted before — which is what `staff_writing_laws.rs` asserts, and why those tests survive unchanged. Prompt 145's
oracle prediction stands unmodified: no `origin=` line carries a derived path, no fixture expands a syntax region, no
`.snap` prints one, so the manifest, the rendered corpus, and every snapshot are byte-identical. It is checked, not
assumed — the Check section runs the comparison.

One Target item in prompt 145 falls out: `PhaseFamily::Builder`'s doc comment — "a function of its displayed arguments
and nothing else — no counter, no clock, no compiler state" — was to be repaired because the Design's counter made it
false of the anchor. Under this decision it is **true of the anchor again**, and stands. The design that needed no
repair to the family's defining property is the right size for the operation; that is evidence for (c), not a
coincidence to note and move past.

## The performance contract this enables

The arity change itself is step-neutral — same arguments minus one, same token built. Its performance content is what it
unblocks: the staff rewrite's `anchored` is the form trial note 43 §1.1 measured, and the rewrite is the deletion of the
call count 141u's instrumentation recorded. The measured baseline, restated so the closing measurement has something to
answer to:

- `examples/staff-page.musa`, whole-compile on the migrated checker: 1,605,182,361 reduction steps; the declaration path
  alone after 141u: 226,873. The spend is the adapter's expansion — the `callN` chains and re-folded reading the rewrite
  removes.
- The staff class fails at `reduction steps at 200001 of 200000` — the budget the rewrite must fit, not by raising it
  (144's table owns that number) but by spending less.

The closing measurement, owed by prompt 145's Check: the staff-page step count after the rewrite, reported beside the
line and byte counts, with the class's tests green at the unchanged budget. If the rewrite fits the file gate but not
the step budget, the finding is about the evaluation model and goes to 144, not into a quieter number here.
