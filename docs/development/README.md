# Working on musa

**Status: operational. Governs nothing.** These pages are about the toolchain and the machine, not about what Musa
means. Nothing here constrains a design decision; if one of these notes and a specification appear to disagree, they are
talking about different things.

What belongs here is the knowledge that is otherwise only in someone's head: a gate that misbehaves rather than fails, a
tool that needs configuring once per checkout, a measurement that explains why something is slow. The test is whether a
new contributor would lose an afternoon rediscovering it.

| Page | What it answers |
| --- | --- |
| [macos-gatekeeper.md](macos-gatekeeper.md) | Why the test suite appears to hang on macOS after any change to `musa-compiler`. Diagnosis measured; **no fix confirmed yet** |

What does *not* belong here: anything that decides semantics (that is `../governance/`, `../spec/`, and the per-stage
specifications), anything about which crate implements what (`../architecture/`), and anything a composer would read
(`../book/`). The commands themselves are in [`../../AGENTS.md`](../../AGENTS.md); these pages explain the ones that
surprise you.

A note here earns its place by being reproducible. State what was measured, on what, and what the numbers were, so the
next reader can tell whether it still applies to their machine.
