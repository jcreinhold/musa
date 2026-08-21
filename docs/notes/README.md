# The notes

**Status: governs nothing.** Nothing here decides what musa is or what gets built.

| Directory | What it is |
| --- | --- |
| [`research/`](research/README.md) | Decision records: one page per decision that a governing or directive document cites |
| [`toolchain/`](toolchain/README.md) | Notes on the machine: a gate that misbehaves rather than fails, a tool that needs configuring once |

The two are different kinds of not-governing. `research/` is about musa: a record there is the reason a rule in
[`../rules/`](../rules/README.md) says what it says. `toolchain/` is not about musa at all — it cannot contradict a
specification, because it is describing macOS or Prettier rather than music. It is kept because a trap that cost someone
a day should cost the next person a minute.

Neither is an archive, and `research/` in particular is not a notebook. A candidate still being weighed and a draft
under review live in a branch until the decision they support is made; then one record of it arrives here, and it stays
only while something binding cites it. Superseded designs are deleted — git history is where a replaced design belongs,
and [`../README.md`](../README.md) says the same thing about every other directory.
