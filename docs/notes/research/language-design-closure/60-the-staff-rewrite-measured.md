# 60. The staff rewrite, measured against the prediction it was gated on

**Status: governs nothing.** `../../../plan/prompts/166-staff-rewrite.md` is the prompt and
`43-dependent-language-trial.md` §12 is the prediction; this page is the measurement that closes the pair. Taken at
prompt 166, after the rewrite, on the tree the commit lands on.

Prompt 128's amendment was granted on one piece of evidence: `stdlib/src/adapters/staff.musa` was what an underpowered
language looks like. Prompt 132 then predicted, in public and before any of the language existed, what the same file
would cost once the language was built. This is the comparison.

## 1. The whole file

|  | Lines | Bytes | Non-comment, non-blank |
| --- | ---: | ---: | ---: |
| prompt 127dcfb, the trial's baseline | 2,404 | 93,252 | 1,814 |
| prompt 142, migrated to the new checker — the actual starting point | 2,515 | 98,555 | 1,909 |
| **after the rewrite** | **2,149** | **80,374** | **1,597** |
| prompt 132's prediction | 2,050 ± 100 | 79,000 ± 4,000 | — |

**Both land inside the band**, at its upper edge: 2,149 against a ceiling of 2,150, and 80,374 against a ceiling of
83,000. Against the file this prompt actually started from, that is −366 lines and −18,181 bytes, or −14.6% and −18.4%;
against the trial's baseline, −255 lines and −12,878 bytes.

The trial predicted the shape of that as well as the size — "bytes fall further than lines because the deleted material
is unusually wide" — and it is right: bytes fall 3.8 points faster than lines.

**The trial's own second gate, which it proposed and prompt 166's Design carries five of:**

| Gate | 127dcfb | Required | Now |
| --- | ---: | ---: | ---: |
| `callN`-style construction helpers | 7 | 0 | **0** |
| hand-allocated role integers | 27 | 0 | **0** |
| `syntax_built` calls | 56 | 0 | **0** |
| eight-field destructures of `Pending` | 14 | 0 | **0** |
| positional `Reading(…)` rebuilds | 20 | 0 | **0** |
| `text_equal` on a token kind or delimiter | 13 | 0 | **0** |
| `text_equal` on a notation keyword | 8 | ≤ 2, inside lookups | **0** |
| phase operations named by the file | 8 | ≤ 5 | **6** |
| lines | 2,404 | ≤ 2,150 | **2,149** |

One row misses. The file names six phase operations — `recurse_syntax`, `run_syntax_step`, `syntax_at`, `syntax_anchor`,
`syntax_number`, `as_expression` — where the trial asked for five. The trial's own §5 list of what survives the
deletions names *seven*, so the ≤ 5 was an aspiration written beside a seven-item list rather than a derivation, and
nothing in the rewrite wanted a sixth removed: `syntax_at` and `as_expression` are the two halves of reading a written
value out of a numeral, and collapsing them would put a parse inside a projection. Recorded as a miss rather than argued
away.

## 2. Where the lines went

The two files divide into sections by their own banner comments, and the sections are not the same sections — the
rewrite re-factored as well as shrank, which is why a row-by-row subtraction is not available. Both listings in full:

| Before (2,515) | Lines |  | After (2,149) | Lines |
| --- | ---: | --- | --- | ---: |
| head: imports, `data`, package | 162 |  | head: imports, `data`, package | 97 |
| reading | 195 |  | reading | 185 |
| emitting | 260 |  | the words | 119 |
| pieces | 126 |  | emitting | 156 |
| placing | 185 |  | placing | 177 |
| what is held | 169 |  | the forms | 165 |
| the forms | 393 |  | the header | 270 |
| the header | 157 |  | the flush | 123 |
| the reader | 513 |  | the reader | 233 |
| the document | 113 |  | the groups | 260 |
| writing | 243 |  | the document | 132 |
|  |  |  | writing | 233 |

Read as the four parts prompt 132 predicted against:

- **Construction** — the emitting section and its call sites, 260 lines becoming 156, and the head's declarations 162
  becoming 97. This is where the quote did its work: fifty-six `syntax_built` calls and twenty-seven role integers are
  gone, and what replaced them is `quote at here { … }`. The trial measured this piece directly at −164 and it came in
  near that.
- **Dispatch** — the reader, 513 lines becoming 233, with a new 119-line "the words" section and a new 260-line "the
  groups" section carved out of it. Net across the three, 513 → 612: **dispatch got bigger.** The trial predicted the
  keyword table would not shrink ("fifteen notation keywords are fifteen keywords") and it did not; what happened on top
  of that is §3 below. This is the section that got worse, and it is the interesting one.
- **`Pending`** — "what is held", 169 lines, folded into the record and the new "the flush" section at 123. The fourteen
  eight-field destructures are gone.
- **`document_read`** — "the document", 113 lines becoming 132, and "the forms" 393 becoming 165. `call7` is gone; the
  form table is a `match` over an `enum`.

The printer is unchanged in size — 243 → 233, −4% — exactly as the trial predicted at −6%. No mechanism in prompts
129–131 was aimed at it, and none reached it.

## 3. What the rewrite actually cost, and it was not lines

`59-the-staff-wall-is-the-evaluators.md` took two of the three orders of magnitude off the staff budget and prompt 165b
implemented them, leaving a factor of 1.9: `examples/staff-page.musa` expanding at **381,055** reduction steps against a
budget of 200,000. Closing that, not the line count, was the work.

**Result: 191,586 steps.** Measured with note 59's probe — the two `Spend`s in `phase/transform.rs` printed rather than
only summed — reverted afterwards.

Getting there needed a cost model the specification does not state, and it is worth writing down because the next
adapter author will need it too. All figures are for the staff-page region: 243 lexical items, about 195 tokens and 24
groups.

**The evaluator's case trees are eager, and `if` is a case tree.** `Bool`'s constructors carry nothing, so a case tree's
method for each of them *is a value*, and evaluating the tree evaluates every method. An `if`/`else if` chain therefore
evaluates every condition in the chain and both branch values, whichever one is chosen. Two consequences, and the second
is the one that shaped the rewrite:

- **Reordering an `if` chain is worthless.** Putting the common case first was tried and measured: it made the file
  *slower*, 15,981 → 16,716 steps on the twenty-item differential, because the reorder lengthened the chain the uncommon
  cases walk without shortening anything.
- **Only shortening it helps**, and every level costs 45–64 steps *for every token that reaches the chain*.

A `match` arm that binds an argument becomes a lambda and *is* lazy. Text-match — `match text { "a" -> … }` — evaluates
all its comparisons at about 60 steps each, but its arm bodies are lazy.

**The measured costs, per token of the page:**

|  | Steps |
| --- | ---: |
| the recursor's own floor, per token, doing nothing | 198 |
| building a `Said` record and matching `State` | ~199 |
| the old nine-level `kinded` chain, plus `body_token` | ~402 |
| **a trivial ignored token (a comma), all in, before** | **799** |
| an unknown identifier, all in | ~1,592 |
| of which `knows` | ~700 |
| `knows`, per arm | ~67 |
| a record `with` update, seven fields | ~46 |
| a note, three tokens | ~4,811 |
| a nesting form (`bar`), all in | ~10,600 |
| a header field | ~5,600 |

**So the winning move was to ask "is this token read at all?" before anything else** — on the raw `TokenKind`, before a
`Said` record exists and before the kind chain runs. Of the page's 195 tokens, 111 are whitespace, the slash between a
pitch and its written value, or the comma between two numbers. A three-level `sighted` chain in front of the nine-level
`kinded` one takes those 111 from 799 steps each to about 440. That is `data Sighted`, and it is the reason the file has
a two-stage classifier where one would read better.

Two more, smaller:

- **Identifiers bypass classification entirely.** `crates/musa-compiler/src/registry/traversal.rs`'s recursor has a
  separate `identifier` branch from `token`, so a word is known to be one before any kind is compared. The adapter took
  it.
- **`knows` ran twice per word** — once to dispatch and once to recover the word. Caching the `Knows` value in
  `Started::Worded` runs it once, at about 700 steps a word.

**This is a finding about the language, not about the adapter.** Every one of these is an author writing around the
evaluator's strictness, and none of them makes the file read better; the `Sighted`/`Kinded` split in particular is a
performance artifact wearing a domain name. An arm for a nullary constructor that is not evaluated when it is not chosen
would delete all three.

## 4. "More obviously correct" — the evidence

The prompt asks for this stated rather than claimed.

- **Every behavioural test passes unchanged.** The twenty-five `staff_expansion_laws` and the three
  `staff_writing_laws`, none of them edited: 28 run, 28 pass. They were ignored for the wall this commit closes and are
  un-ignored here; the slowest runs in three seconds.
- **The anchors land on the same nodes.** An anchor number is a node's position in the region's reading order, the
  rewrite changes no reading order, and `staff_writing_laws.rs`'s edit law — which asserts the patch lands on the node
  the command named and moves no other byte — passes on its original assertions.
- **The printer round-trip holds at the same conformance level**, by the same test, and the loss sentences are the same
  sentences.
- **The oracle did not move.** `tests/fixtures/elaboration-compatibility.txt`, the rendered corpus, and every `.snap`
  are byte-identical, checked rather than assumed: `cargo insta test --workspace --unreferenced=reject` accepts nothing
  new and the manifest test passes. This is the prediction the Design section made — no `origin=` line carries a derived
  path and no oracle fixture expands a syntax region — and it held.

## 5. Was the gate the right gate?

The trial said in advance that it would not be: "the line count was chosen in prompt 128's amendment as the evidence
that the language was underpowered, and it was the right evidence for *that* claim … It is the wrong evidence for
whether the replacement worked, because most of the file was never about the language."

That reads correctly from the other side. The file came in at 14.6% shorter, inside the predicted band and nowhere near
the 25% the trial named as "dramatic". But the seven mechanical rows above all went to zero, including the two the
amendment quoted — 119 lines of `callN` and 27 unchecked role integers — and the section the language was actually aimed
at, construction, fell by 40%. What did not move is the printer, which no mechanism was aimed at, and dispatch, which
got *bigger* for a reason that is the evaluator's and not the language's.

The honest summary is that the prediction was accurate and the gate was coarse. The number to carry forward is not 2,149
lines; it is that the rewrite spent its effort on §3's cost model rather than on §1's line count, which is where the
next measurement should be pointed.

## 6. What un-ignoring the class exposed: the budget is not deterministic in one process

Twenty-five of these laws had carried `#[ignore]` since prompt 127dcfa. Running them for the first time turned up
something that is not the adapter's and not this prompt's, and it is worth writing down before somebody meets it as a
flake.

**`cargo nextest run --workspace` is green, 1,895 of 1,895. `cargo test -p musa-compiler --test suite` fails 25 of the
same tests.** The difference is not the runner's opinion: nextest gives each test its own *process*, and libtest runs
654 tests as threads in one. Single-threaded — `cargo test … -- --test-threads=1` — all 653 pass. The same expansion
therefore succeeds or crosses the budget depending on what else the process happens to be compiling at the time.

Measured with note 59's probe. The staff adapter's module read costs a fixed 75,377 steps, and reading one small staff
region costs 55,241 more, so 130,618 of 200,000 — comfortable. Under sixteen concurrent tests the same region's run
needs more than 124,623 and the refusal reports `attempted 200001`.

The cause is one line. `crates/musa-calculus/src/kernel/meta.rs`'s `SOLUTIONS` is a **process-global** `AtomicU64`, and
it is the invalidation stamp guarding the δ-unfolding memo prompt 165b added:

> equal stamps mean no solution arrived in between … it is conservative in the safe direction because a solution
> anywhere invalidates every memo rather than only the ones that mention it.

"A solution anywhere" is a solution in *another compilation on another thread*. Every metavariable any concurrent
elaboration solves invalidates every memo in this one, 165b's graph update stops firing, and the spend roughly doubles.
Confirmed by experiment: replacing the global with a `thread_local!` cell — nothing else changed — makes all 653 pass
multithreaded, in 28 seconds.

That is a determinism defect, not a test-harness quirk. `../../../rules/language/02-core-calculus.md` §4's budget is
supposed to accept and refuse exactly the same programs everywhere, and `phase/transform.rs`'s doc says as much of its
two hosts. A desktop session compiling two documents at once is the same shape as the test binary. The stamp belongs to
the elaboration context that owns the metavariables, not to the process; prompt
[166b](../../../plan/prompts/166b-per-context-memo-stamp.md) is the repair.
