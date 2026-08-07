---
name: module-design
description: 'Use for designing Rust modules, crates, or APIs: public/private boundaries, refactoring surface area, facade crates, information hiding, and single-implementor traits.'
---

# Deep Module Design

Keep the interface small and concrete. Hide work, layout, and choices that callers should not need.

Read `references/rust-patterns.md` for language patterns.

## Defaults to Resist

- Do not make an item public, generic, configurable, or trait-based without a real caller that needs it.
- Do not add wrappers or facades that only forward calls.
- Do not split code by file, layer, or feature before checking what must change together.
- Do not add an error case unless a caller can recover from it.
- Do not stop at a green build. Read callers and check the public surface.
- For a non-trivial public boundary, compare two designs with different boundaries or invariants.
- Use recent history to find volatile decisions:

  ```bash
  git log --oneline -10 -- <path>
  git log -p -5 -- <path> | head -200
  ```

## Orient First

| Pressure | Test |
| --- | --- |
| Shallow module | The interface exposes nearly as much as the implementation. |
| Leaky abstraction | Callers need internal facts. |
| Mixed concerns | Independent concerns share one type or module. |
| Growing surface | Public items grow without new capability. |
| Temporal coupling | Callers must follow an order that types do not enforce. |
| Information loss | Callers must rebuild information the module had. |

If unclear, run `bash .agents/skills/module-design/scripts/audit-module.sh <path>` from the repo root.

## Rules

Apply these in order when they conflict.

1. **Keep independent concerns apart.** If they change independently, do not make callers manage them through one
   interface.
2. **Make modules deep.** A small public surface should serve several use cases and hide substantial work.
3. **Hide volatile decisions.** Group code by data format, policy, or storage choice, not by the current sequence of
   steps.
4. **Put defaults inside.** Remove parameters that every caller gives the same value.
5. **Remove needless errors.** If a normal result can safely cover an edge case, use it.
6. **Generalize the interface, not unused functionality.** Use terms that fit several current callers; do not add knobs
   for imagined ones.
7. **Make each layer add an abstraction.** Merge a pass-through wrapper, or give it real work to hide.
8. **Split and combine by information sharing.** Combine code that shares state or yields a narrower interface; separate
   code that changes on its own.

## Audit

Stop at the first failed question.

1. Does the public surface hide more than it exposes?
2. Can each concern change without forcing unrelated changes?
3. Can internals change without breaking callers?
4. Did the change make callers simpler?
5. Can normal semantics remove an error path?
6. Does each public operation serve more than one real use case?

Treat dead surface as dead-code removal, not redesign.

## By Context

### New module or boundary

- State the public API and its invariants before the implementation.
- Compare two genuinely different designs for a non-trivial boundary.
- Keep items private until a caller needs them.

### Refactor

- Find the boundary that hides the most volatile detail.
- Merge modules that always change together.
- Keep a long method when splitting would force callers to pass state between pieces.

### Library boundary

- Add a crate, module, or facade only to hide a stable internal structure.
- A facade must curate a narrower API; do not re-export everything.

### Review

Look for change amplification, too much caller knowledge, and hidden dependencies.

## Failure Smells

| Smell | Fix |
| --- | --- |
| Public fields expose layout | Make fields private; expose operations. |
| Wrapper only delegates | Merge it or make it hide real work. |
| Call order enforced by convention | Use types, a builder, or a scoped API. |
| Storage, policy, and presentation share a module | Split by independent change. |
| Every caller passes one parameter value | Move the default inside. |
| One-implementor trait or class | Use the concrete type. |
| Public item “for future use” | Keep it private. |
| Error no caller can handle | Redefine the normal operation. |
| Public interface mirrors storage | Expose a caller-oriented operation. |
| Module cannot be named in one sentence | Split or simplify the concern. |

## Before Declaring Done

- [ ] Each new public item, parameter, trait, and error case has a real caller.
- [ ] No new pass-through method or mixed public concern.
- [ ] Run `bash .agents/skills/module-design/scripts/audit-module.sh <path>`.
- [ ] Read two or three callers; they are simpler or the added capability justifies the cost.
- [ ] Run `cargo clippy -p <crate>` and `cargo nextest run -p <crate>`.

## Dispatch Agents

- `agents/pre-design-audit.md` — establish constraints before substantial design work.
- `agents/post-design-verify.md` — check the changed surface and callers after implementation.

## References

- `references/design-principles.md` — theory and tradeoffs.
- `references/rust-patterns.md` — Rust patterns.

## Related Skills

- `rust-testing` for tests coupled to internals.
- `rust-performance` when boundary changes are driven by hot-path allocation or layout.
