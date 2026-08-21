---
id: 117
slug: analysis-service
status: done
depends_on: [42, 99, 116]
phase: 3
---

# A Typed Observation Service for Musical Analysis

## Task

Build the narrow observation boundary that tonal and contrapuntal analyses consume. Analyses run on a closed
`Timeline<ScoreFact>`/score projection, return typed findings with evidence and assumptions, and never construct or
rewrite `music`. Expose one caller-oriented operation through project/CLI so later algorithms have real users without
publishing compiler passes or raw fact traversal.

## Read

- `docs/rules/language/00-semantics.md` and `05-verification.md`; prompt 42's snapshot interface and prompt 44's event
  track queries.
- Current compiler/project/CLI/LSP facades and all callers of `ScoreSnapshot` facts.
- OMT `104-analyzing-with-set-theory-or-not.md` and `107-analyzing-with-modes-scales-and-collections.md`: analysis
  choices and musical meaning are contextual, not merely a computation over labels.
- Peyton Jones (1987) Chapter 22, *Strictness Analysis*, for the shape a program analysis should have: §22.1's abstract
  interpretation — an abstract domain, an abstract version of each operation, and a stated relationship to the concrete
  semantics — and §22.3 on why the obvious treatments of recursion are wrong. Musa needs no strictness analysis; it
  needs the discipline.

## Design

Compare two boundaries in the prompt's repair notes: a new public analysis crate versus a private compiler analysis
subsystem reached through one stable request/report operation. Choose the latter unless a non-compiler consumer truly
needs to own the algorithms. The likely public surface is one immutable `AnalysisRequest`/`AnalysisReport` pair and
`analyze`, with project as the normal facade for CLI/LSP/desktop; internal segmenters, candidate graphs, indexes, and
theory values stay private.

**State every analysis as an abstract interpretation, not as a traversal.** Each analysis kind declares: the abstract
domain its findings live in, the abstraction map from the concrete `Timeline<ScoreFact>` projection to that domain, and
the soundness claim relating them — what a finding does and does not license a caller to conclude. An analysis whose
soundness claim cannot be written is an analysis that must not ship, because a finding with no stated relationship to
the score is exactly the false claim `docs/rules/language/03-musical-domains.md` §6 forbids. This is what makes the
fact/candidate/conflict classification below mean something precise — candidates are concrete interpretations the
abstraction cannot separate — rather than being severity labels chosen by feel, and it is what prompt 118 inherits.

A request names an analysis kind, score scope/voices, exact time window, and explicit assumptions/policies. A finding
has a stable code, typed summary data, severity/classification (fact, candidate, conflict), evidence spans/fact ids, and
source references. Do not emit a fake probability: where several interpretations survive, return ordered or unordered
candidates with the stated deterministic ranking evidence. Reports are separate from compiler diagnostics unless the
user explicitly requested an assertion/check.

Add `musa analyze <file> --kind facts` as an architectural smoke: it reports selected pitch/chord/context facts and
their exact source evidence, proving request/window/scope and text/JSON rendering without implementing prompt 118's
tonal judgments. LSP and desktop plumbing wait until prompt 122/116.

## Target

- Private compiler analysis subsystem and the smallest stable request/report boundary justified by project/CLI callers.
- Project operation and `musa analyze --kind facts --format text|json` with deterministic ordering.
- Tests for scope/window selection, evidence/source mapping, invalid request diagnostics, deterministic rendering,
  analysis not changing semantic hash/snapshot, and public-surface audit.
- `docs/rules/language/07-analysis.md`: boundary, finding/evidence semantics, algorithm admission rules, and extension
  guide. Its admission rules must require an abstract domain, an abstraction map, and a soundness claim per analysis
  kind, and must cite the source of that discipline.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa -- -D warnings
cargo fmt --check
cargo run -p musa -- analyze examples/annotated.musa --kind facts --format text
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-compiler
```

Commit as `Add the musical analysis service boundary`.

## Stop

- No tonal, cadence, modulation, counterpoint, or voice-leading algorithm yet.
- No new crate without the independent consumer evidence required above.
- No raw public occurrence iterator added for analysis, no pass-through project wrapper, and no mutable report.
- No diagnostics/lints or source changes from an analysis request.
