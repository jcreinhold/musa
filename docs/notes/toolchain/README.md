# Working on musa

**Status: operational. Governs nothing.** These pages are about the toolchain and the machine, not about what Musa
means. Nothing here constrains a design decision; if one of these notes and a specification appear to disagree, they are
talking about different things.

What belongs here is the knowledge that is otherwise only in someone's head: a gate that misbehaves rather than fails, a
tool that needs configuring once per checkout, a measurement that explains why something is slow. The test is whether a
new contributor would lose an afternoon rediscovering it.

| Page | What it answers |
| --- | --- |
| [generated-files.md](generated-files.md) | Why a currency test says a fixture is stale when nobody changed the generator, and which formatter owns which files |
| [nextest-fail-fast.md](nextest-fail-fast.md) | Why `--run-ignored all` reports a handful of failures instead of the whole list, and why the count moves between runs |
| [playwright-under-recursive-pnpm.md](playwright-under-recursive-pnpm.md) | Why `pnpm -r test` fails a screen test that passes when the UI package runs on its own |
| [slow-test-suite.md](slow-test-suite.md) | Why the test suite appears to hang on macOS at 0% CPU, and why `cargo clean` fixes it |
| [tracing-in-tests.md](tracing-in-tests.md) | Why a logging law fails under `cargo test` but passes under `cargo nextest run`, and what to install instead of `with_default` |

What does *not* belong here: anything that decides semantics (that is `../../rules/constitution.md`,
`../../rules/across-stages/`, and the per-stage specifications), anything about which crate implements what
(`../../plan/code-map/`), and anything a composer would read (`../../book/`). The commands themselves are in
[`../../../AGENTS.md`](../../../AGENTS.md); these pages explain the ones that surprise you.

A note here earns its place by being reproducible. State what was measured, on what, and what the numbers were, so the
next reader can tell whether it still applies to their machine.
