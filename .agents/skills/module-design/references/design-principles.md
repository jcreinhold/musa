# Design Principles

Use this when the short skill needs a design argument.

## Module Depth

Interface cost matters more than implementation size. A module is deep when a small API hides enough work to serve several use cases. A shallow module exposes nearly all of its work through methods, types, and configuration.

- Count public items and the use cases they support; a ratio is only a prompt to inspect, not a target.
- A small wrapper is valid only when it hides policy, representation, retries, caching, or another real concern.
- Split only when the resulting boundaries hide independent decisions. Merge code that shares state or forces callers to cross both boundaries for ordinary work.

## Information Hiding

Hide decisions likely to change: storage, format, policy, ordering, and optimization. Use history to find them. A public interface leaks when callers must know its representation or reconstruct data it already had.

## Simplicity

Keep independent concerns separate. Familiarity is not simplicity. A type that combines storage, policy, and presentation makes changes interact; divide it where those concerns vary independently.

## Interface Rules

- Put common defaults inside the module.
- Define edge cases as normal results when callers do not need recovery.
- Use caller-neutral operations, but add functionality only for current needs.
- Do not add a trait, builder, generic type, or public item until a second use requires it.
- Write comments for invariants, reasons, and surprises; do not restate signatures.

## Review Signals

Look for one logical change touching many sites, callers holding too many details, or safe-looking changes breaking distant code. These show a missing boundary or a leaked decision.
