# 07 — Analysis

Status: **candidate**, with `docs/language/05-verification.md` §3 governing over it.

`05-verification.md` divides theory work into three strengths: constructor invariants, explicit assertions, and
interpretive analyses. This document is the third one, elaborated. It fixes what an analysis may say, what a caller
receives, and — the part that matters most — what a new analysis kind must prove before it may ship.

## 1. What an analysis is

An analysis **observes a compiled score and reports what it saw**. It never constructs music, never rewrites source, and
never raises a diagnostic. Those are not conventions; they are the shape of the operation:

```rust
pub fn analyze(snapshot: &ScoreSnapshot, request: &AnalysisRequest) -> Result<AnalysisReport, AnalysisError>
```

The snapshot is borrowed and the report is a value. There is no path from an analysis back into the resolver, which is
what keeps an analysis from becoming a lint with extra steps — prompt 83 drew that line and this does not move it.

The exit code of `musa analyze` says whether the *request* could be answered, never what the report contains. A piece
with nothing in the requested window is a piece with nothing in the requested window; it is not a failure.

Consequently:

- **A bad request is an error.** A misspelled part, a voice that is not in the part named, a window that ends before it
  starts. Each is something the caller can fix by asking differently, which is the test for belonging in `AnalysisError`
  at all.
- **An empty answer is a report.** Zero findings is a well-formed observation.

## 2. The admission rule

> **No analysis kind ships without an abstract domain, an abstraction map, and a soundness claim.**

The discipline is Peyton Jones (1987) Chapter 22, *Strictness Analysis*, §22.1: a program analysis is an abstract
interpretation, which is an abstract domain, an abstract version of each operation, and a stated relationship to the
concrete semantics. Musa needs no strictness analysis. It needs that shape.

Each kind states, in the doc comment of the module that implements it:

1. **The abstract domain.** What its findings *are*, as a set. Not "chord labels" — the actual objects, with the
   coordinates they carry.
2. **The abstraction map α.** How the concrete score projection (`ScoreSnapshot`, itself a projection of a
   `Timeline<ScoreFact>`) maps into that domain. Every finding must be the image of something under α.
3. **The soundness claim.** What γ, the concretization, admits: the set of concrete scores a report is consistent with.
   This is what a finding licenses a reader to conclude — and, equally, what it does not.

This is not documentation for its own sake. It is what makes §3's classification mean anything. Without a stated α, a
"candidate" is a severity label chosen by feel, and a finding with no stated relationship to the score is exactly the
false claim `03-musical-domains.md` §5 forbids. **An analysis whose soundness claim cannot be written must not ship.**

§22.3 of the same chapter is the standing warning about the other half: the obvious treatments of recursion are wrong.
An analysis that walks a structure with self-reference — a form graph, a motivic derivation, a reduction tree — owes a
fixed-point argument, not a recursion that happens to terminate on the corpus.

## 3. Findings: fact, candidate, conflict

A finding carries a stable code, a typed observation, a standing, and evidence. The standing is a position relative to
the kind's own abstraction, never a severity:

| Standing | What it means |
| --- | --- |
| `fact` | α determines it: every concrete score the report is consistent with agrees about it. |
| `candidate` | Several concrete readings survive α and this is one of them. |
| `conflict` | Two readings α says cannot both hold, reported together. |

A candidate is never reported alone: the readings it competes with are in the same report. Their order is the kind's
**stated deterministic ranking**, and it is not a probability. Musa does not emit a confidence number it cannot derive —
"73% likely a half cadence" is a claim about a corpus nobody named.

There is no privileged "the analysis". Two kinds may return different well-typed readings of one passage; that is data,
not malformed music. Analysis results enter kernel payloads only when an author explicitly writes an annotation derived
from one, and the provenance records that it was their choice.

## 4. Evidence

Evidence answers "where can I see this", and it has three shapes because there are three genuinely different answers:

| Evidence | When | What a caller can do with it |
| --- | --- | --- |
| `Event` | the finding is about a note, chord, or rest | select it on the page; the span is the statement that *spells* it — the note inside the motif, not the `use` that placed it |
| `Annotation` | the finding is about something written above the staff | reveal it in the source |
| `InForce` | the finding is about a value in force from an instant | nothing to reveal: a context track records where a value takes force, which is a place in the piece rather than a place in a file |

Collapsing these into one optional span would make every consumer rediscover which case it was holding.

Reports are **totally ordered and deduplicated by `analyze` itself**, not by whoever prints them: by instant, then by a
fixed rank per observation kind, then by part, voice, and event. Two runs of one kind over one score produce identical
bytes — otherwise nobody can diff a report against yesterday's and act on the difference.

## 5. The boundary

The public surface is one request, one report, one function, and the types they are made of. Segmenters, indexes,
candidate graphs, and theory values stay private to `crates/musa-compiler/src/analysis.rs`.

A public `musa-analysis` crate was considered and rejected. Its only caller would be `musa-compiler` — every consumer
reaches analysis through `musa-project` — while its public surface would be exactly the pass details that later kinds
change. A new crate is justified only by a consumer that genuinely needs to *own* the algorithms, and there is none.

Callers reach it through `ProjectSession::analyze`, which resolves the report into display-ready facts: names instead of
ids, bars and beats instead of rationals, lines instead of byte offsets, and one sentence per finding. That is not a
pass-through — `docs/interface/03-interaction.md` §7 lists what a frontend may compute, and nothing musical is on it, so
the resolution has to happen somewhere below the frontend and this is where.

## 6. Adding a kind

1. Write the abstract domain, the abstraction map, and the soundness claim, in the module doc, before the code.
2. Add the variant to `AnalysisKind` and give it a `method` line and a non-empty `assumptions` list. An empty
   assumptions list is itself a claim — that the reading depends on nothing — so no kind may leave it empty.
3. Implement it as a private submodule taking the resolved lanes and window.
4. Prove the soundness claim in tests: what the kind reports, what it refuses to report, and — for a kind that emits
   candidates — that competing readings arrive together and in the stated order.
5. Name it honestly. `05-verification.md` §2's rule holds here too: `species.first_above(cantus)` is honest,
   `valid_counterpoint(cantus)` is not.

## 7. The kinds

### `facts`

**Abstract domain.** The finite set of *pointed statements* of a score: a sounding written pitch with its exact span, a
silence with its exact span, a chord symbol at an instant, and a key or meter in force from an instant — each paired
with where in the score it can be seen.

**Abstraction map.** α restricts the snapshot to the requested lanes and window and reads every surviving score event,
harmony annotation, and key and meter stretch into exactly one element of that set. A chord event becomes one `Sounding`
per tone: a chord at this layer is simultaneous notes and nothing more (`00-semantics.md` §3).

**Soundness.** α is *exact on what it reports*. For every finding there is a statement of the score with precisely those
coordinates, and every statement of the score inside the scope and window has a finding. Therefore every finding is a
`fact`: α separates every pair of concrete scores that differ on anything it reports, so no two readings survive it and
there is nothing to be a candidate about.

A finding licenses exactly "the score states this here". It licenses nothing about material the request excluded: γ of a
report is every score agreeing with it inside the scope and window, and that set is not a singleton. A reader who
concludes "the piece is in C major" from one `key-in-force` finding over one bar has read something α does not say.

**What it does not do.** It does not interpret. A chord symbol is reported as written and no notes are derived from it,
because `crates/musa-compiler/src/harmony.rs` opens by saying a symbol is recorded and never interpreted. Spelling
survives: `d#4` is reported as D-sharp and never as E-flat.

Later kinds — tonal function, cadence, modulation, voice leading — arrive in prompts 118 and 119, each under §2's rule.
