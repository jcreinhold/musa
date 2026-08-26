# Traits, methods, and namespaces — retired

**Status: retired.** This document specified the trait system: coherence, the orphan rule, instance lookup, dictionary
elaboration, the operator table, and method resolution. It is retired outright and has **no successor**. The file is
kept so that the references to it in `docs/plan/prompts/` and `docs/notes/` still resolve.

**What retired it.** The amendment at [prompt 143](../../plan/prompts/143-one-theory-amendment.md) removes traits rather
than narrowing them, on a measurement: six traits, 82 call sites, and **zero** trait-constrained signatures — not one
function in `stdlib/` or `examples/` is polymorphic over a trait — with `Eq`'s five instance bodies being five builtins
with no λ around them. A dispatch mechanism with nothing to dispatch on is a name-resolution mechanism wearing a
costume. [Prompt 146](../../plan/prompts/146-delete-the-trait-system.md) deleted the mechanism and put type-directed
name disambiguation in its place.

**Where each part went.**

| What this document held | Where it is now |
| --- | --- |
| a trait is a record of methods | a structure **is** a record, and an instance is an ordinary value: [`01-surface.md`](01-surface.md) §1.2 |
| coherence, and the orphan rule that made it checkable | nothing. Coherence was a property *of the trait system*; with no instances there is nothing for two of them to disagree about |
| instance lookup, in three steps | type-directed disambiguation: candidates by name, filtered by the expected type, one diagnostic listing what ruled each out ([`01-surface.md`](01-surface.md) §1.5) |
| dictionary elaboration | nothing. There is no dictionary to pass |
| the operator table | [`01-surface.md`](01-surface.md) §1.5: operators are surface syntax for named functions, disambiguated by type |
| the collection traits `Iterable` and `Buildable` | the fold over a family **is** its generated eliminator ([`02-core-calculus.md`](02-core-calculus.md) §1.1); the rest are ordinary library functions ([`01-surface.md`](01-surface.md) §1.6) |
| "a structure's carrier is its first parameter" | [`../style-guide.md`](../style-guide.md) §6, as a field-order convention rather than a resolution key |
| §7's two equalities to tell apart | [`02-core-calculus.md`](02-core-calculus.md) §3 and [`../obligations.md`](../obligations.md) §17: acceptance is decided by one relation |

**One consequence a later prompt depends on.** This document stated coherence as "unique up to conversion… decided by η
at `quote`", and that sentence was the **sole** reason record η was required by anything. Retiring it removes η's only
client, which is why [prompt 157](../../plan/prompts/157-records-leave-the-core.md) can make a record a one-constructor
inductive family without losing something that was load-bearing.
