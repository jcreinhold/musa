# The event-track specification

**Status: governing since prompt 12; amended at prompt 127a.** Bound by [`../constitution.md`](../constitution.md),
[`../obligations.md`](../obligations.md), and [`../across-stages/`](../across-stages/README.md); binds every consumer of
the semantic core. Where the built compiler and a page here disagree, either the code is wrong or the page needs a
deliberate repair.

This directory specifies the **finite event track**: one of the two core values of `../constitution.md` §3 and §4, and
the thing every surface construct elaborates into. An event track answers one question — *what facts exist, and where do
they exist in this coordinate of musical time* — over exact rational time, with typed occurrences and the operations
`empty`, `event`, `follow`, `together`, `map_payloads`, and `duration`. How those facts were produced, how they are
displayed, and how they sound belong to layers above or beside it, never inside it.

The other core value, the machine, is specified in
[`../across-stages/03-machine-calculus.md`](../across-stages/03-machine-calculus.md), and the checked scheduler that
connects the two is specified there as well.

**Read [`00-purpose.md`](00-purpose.md) first.** It argues why the core is this small, what it refuses to hold, and how
the pieces below fit together; the rest of the directory is written assuming it.

## Document map

| File | Contents |
| --- | --- |
| [`00-purpose.md`](00-purpose.md) | Why the core exists, what it refuses, and the pipeline it sits in. |
| [`01-grammar.md`](01-grammar.md) | The interchange syntax (not the musician-facing syntax). |
| [`02-static-semantics.md`](02-static-semantics.md) | Well-formedness rules. |
| [`03-denotational-semantics.md`](03-denotational-semantics.md) | The denotation `(d, E)` and every operation's definition. |
| [`04-algebraic-laws.md`](04-algebraic-laws.md) | The laws, formally, cross-referenced to their property tests. |
| [`05-normalization.md`](05-normalization.md) | Canonical normal form, semantic equality, serialization. |
| [`06-surface-elaboration.md`](06-surface-elaboration.md) | How the surface constructs elaborate. |
| [`07-backend-contract.md`](07-backend-contract.md) | What downstream consumers may assume. |
| [`08-open-questions.md`](08-open-questions.md) | What is deliberately undecided. |
| [`09-pipeline-baseline.md`](09-pipeline-baseline.md) | The measured cost of the core path, prompt by prompt. Descriptive; decides nothing. |
| [`10-term-calculus.md`](10-term-calculus.md) | The term calculus: syntax, evaluation, soundness theorems. |
| [`11-realization.md`](11-realization.md) | Seeded finite realization and its reproducibility laws. |
| [`12-payload-admission.md`](12-payload-admission.md) | Payload equality, schema ownership, law transport, and exact identity framing. |

The numeric prefixes are the reading order.
