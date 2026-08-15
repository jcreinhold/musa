# The staff adapter on the repaired interface, measured per repair

## Purpose

Five prompts repaired the elaboration language between prompt 127dcfa, which wrote
[`stdlib/src/adapters/staff.musa`](../../../../stdlib/src/adapters/staff.musa), and this one, which rewrites it against
them. This note records what each repair actually removed from that file, so that prompt 127dd freezes evidence rather
than a summary. The rewrite preserves behaviour exactly: same values, same diagnostics at the same spans, same paths,
byte-identical fixtures, no movement in the compatibility oracle.

Nothing here amends `docs/rules/`, and nothing here is a language proposal.

## Verdict

**The interface is now complete enough to write the adapter the notation wants, and the adapter got bigger.** Both
halves are the finding.

1. **The hole closed for `( … )` and stayed open for `[ … ]`.** Note 40 §3.3 showed the fold made a re-descending reader
   *unwritable*: `group_read → taken_piece → group_read` is a dependency cycle. Under `recurse_syntax` a group met
   inside parentheses is now read exactly like the numerals around it. A group met inside brackets is still not a pitch
   — but for an unrelated reason, given in §4.
2. **`Pending` lost nothing.** All seven of its argument slots survive. Note 40 finding 2 predicted this; the prompt's
   own Design ("slots that were only ever unclaimed arguments go") did not. §3 says which slots are the notation's and
   which were the interface's, and the answer is that none of them were the interface's.
3. **The trial's line count does not survive at full size.** On the simplified slice, traversal-specific lines went 44 →
   34. At full size they go 149 → 187. §2 gives the table and §5 says why the trial could not have seen it.
4. **A single pass costs more constructed cells than two passes did.** On the smallest region the adapter accepts,
   expansion charges 7,217 constructed value nodes against 3,378 before — 2.1×, and 7% of the 100,000 the language
   budget allows. §6 explains the arithmetic and why it is not worth removing.

## 1. What each repair removed

Measured on `stdlib/src/adapters/staff.musa` at each commit that touched it, against the same file at the commit before.
Line counts are `total / non-comment non-blank`.

| Prompt | Commit | Lines | Δ code | What it removed from this file |
| --- | --- | ---: | ---: | --- |
| 127dcfa (as written) | `e3ddd0f` | 2142 / 1737 | — | — |
| 127dcfaa — `list_fold_from_end` | `82b71bb` | 2128 / 1730 | −7 | the `from_the_end` closure chain: one closure and one extra application per element, and the reason the smallest region could not be expanded at all |
| 127dcfab — expression `if` | `3234a68` | 2116 / 1718 | −12 | `clef_named`'s four-deep `match text_equal(…)` staircase, and three like it |
| 127dcfac — record update | `a0028d2` | 2041 / 1645 | −73 | five of seven `fn holding_*` setters; explicit `Pending(…)` constructions 13 → 5 |
| 127dcfad — `?` | `c6b05ef` | 1992 / 1591 | −54 | `document_read` 115 → 61 lines, deepest indent 56 → 32 columns, sixteen nested `match` expressions → four |
| 127dcfaf — the recursor | `64b97f6` | 1992 / 1591 | 0 | nothing: the prompt landed the recursor in the *language* and only renamed `syntax_fold` at this call site |
| 127dcfag — this prompt | — | 2158 / 1652 | **+61** | `data Read`, the second traversal, and the nested-group hole for `( … )` |

Two things are worth reading off this table before §2.

**The ergonomic repairs did the shrinking.** 127dcfac and 127dcfad together removed 127 code lines; the traversal
replacement added 61. An aggregate improvement credited to the recursor would have been off by more than the recursor's
own sign. This is the reason prompt 127dd's evidence had to be gathered per repair, and it is the one place where the
plan's ordering paid off in a way a single measurement would have hidden.

**The recursor's cost is not the ergonomics' cost.** What the recursor bought is in §4, not in a line count.

## 2. Fold against recursor, counting only what one version has

Counting, as note 40 §3.3 did, only the definitions one version has and the other does not:

|  | fold (`64b97f6`) | recursor (this prompt) |
| --- | --- | --- |
| definitions the other version lacks | 149 lines: `data Read`, `taken_piece`, `read_body`, `stated_numbers`, `stated_exact`, `body_of` | 187 lines: `data Place`, `data State`, and sixteen `fn`s |
| constructor arms written for a shadow tree | 10 | 0 |
| traversals over the same children | 2 | 1 |
| explicit `Pending(…)` constructions | 5 | 1 |
| `with { … }` record updates | 8 | 23 |
| deepest indent, whole file | 44 columns, in `stated_numbers` | 44 columns, in `bar_read`, which this prompt did not touch |
| deepest indent, traversal-specific | 44 columns | 40 columns, in `body_token` — the fold's `token_read` verbatim |
| `let expand` | 33 lines, five result arms and an inline trivia filter | 20 lines, two result arms |
| nested group inside `( … )` | dropped | read |
| nested group inside `[ … ]` | dropped | dropped, for the reason in §4 |
| constructed value nodes, smallest accepted region | 3,378 | 7,217 |
| evaluation steps, same region | 5,040 | 6,688 |

Where a job exists in both versions, the counterparts are:

| job | fold | recursor |
| --- | ---: | ---: |
| what a token means in a body | `token_read` 51 | `body_token` 51 — the same text |
| what a token means between parentheses | `stated_numbers` 49 + `stated_exact` 34 | `stated_token` 45 + `one_more_number` 20 |
| what a group means | `group_read` 41 | `entered` 15 + `parens_pending` 18 + `brackets_pending` 30 |
| a body between braces | `read_body` 32 | `body_reading` 23 |

So the reading of the notation itself got smaller (134 → 116, and that is `?` and record update, not the recursor); the
traversal got larger. `data Read` and its ten constructor arms went away and were replaced by `data State`, a
`data Place`, a place dispatch in each of the three step functions, and three separate ways to run a group's children —
threaded from the end for a body, threaded from the start between parentheses, mapped between brackets.

That is the honest shape of the trade. The fold's first pass was cheap because it built leaves that meant nothing; the
recursor has no such pass, so every node is met by a reader that already has to know where it stands.

## 3. Which parts of `Pending` were interface cost

None of them.

`Pending` has eight fields: `read`, the `Reading` built so far, and seven argument slots — `length`, `dots`, `tying`,
`numbers`, `taken`, `voiced`, `words`. Every one of the seven is an argument that arrived before the word that says what
it was: `bar (4, 4) { … }` is read from its end, so the braces arrive, then the parentheses, then `bar`. That is forced
twice over — once by the notation, and once by the data this adapter emits, which is right-nested
(`Sounded(anchor, event, after)`), so what a tie reaches must already be in hand when the tie is met. A hand-written
recursive-descent reader over the same notation producing the same data would hold the same seven things.

What the recursor removed is the *shadow tree*: `data Read`, whose five constructors existed only so that a bottom-up
pass could hand a top-down pass something to walk again, and `taken_piece`, which did that walking. Note 40's finding 2
is confirmed at full size; the prompt's Design expectation is refuted.

**The prompt's `C` is also not what its Design said.** The Design proposed `C` = "the inherited meter and open-form
state". The meter is not inherited here and must not become so: the notation says the meter on every bar on purpose, and
`no_note_takes_its_register_or_its_value_from_the_note_before_it` is the law that would break if a bar took its length
from the page. What the inherited context is actually spent on is smaller and more local — which of three places a node
was written in:

```musa
data Place { InBody, InParens, InBrackets }
data State { Unopened, Opened(place: Place, pending: Pending) }
```

`Unopened` is not a design flourish. `recurse_syntax`'s initial context has to be a `State`, a `Pending` needs a
`Reading`, and a `Reading`'s items are built from a `NodePath` — and no operation makes a `NodePath` before a node is
entered. So the initial context is the one state that means "nothing has been entered yet", and `place_of`,
`pending_of`, and `reopened` are the three lines that keep it from appearing in any step function.

**The specific claim under test passes.** `A` is not `Context -> Result<…>`. `C = A = State`, forced by the threading
idiom `list_fold_from_end(seed, fn (kid, later) { run_syntax_step(later, kid) }, kids)`, which uses the accumulator as
the context. No closure chain was reintroduced under another name, and no continuation encoding appears.

## 4. The hole, half closed

Note 40 §3.3 required this prompt to close the nested-group hole and to say that it did.

**Between parentheses it is closed.** The numbers a form states are *threaded*, so a nested group is read by handing its
children the state that reached it:

```musa
InParens -> list_fold_from_start(
    state,
    fn (kid, earlier) { run_syntax_step(earlier, kid) },
    kids,
),
```

This is exactly the program the fold rejected as `group_read → taken_piece → group_read`. It is not a cycle here,
because `run_syntax_step` names no function: the child's own step carries the traversal, and re-entering the traversal
is an application of a sealed value rather than a static call. That is the whole of what sealing buys, and it is the
single most important thing this rewrite demonstrates about the interface.

**Between brackets it is not closed, and the traversal is not why.** A chord's pitches must come out as a `List` to hand
to `syntax_group`, one written pitch per child, so `voiced_inside` is a `map` over the group's own children — and a
`map` is length-preserving. A nested group might hold several pitches, and the phase language has no operation that
builds a list or joins two. So a group written inside `[ … ]` still contributes no pitch.

This is attributable to "a list cannot be built", which the file's own header has listed as a limitation since prompt
127dcfa, and not to `recurse_syntax`. It would close if the adapter could concatenate; it does not close by any
rearrangement of the traversal. Recorded here rather than repaired, because repairing it is a language change and this
prompt's Stop forbids one.

## 5. Divergences from the paper trial

Note 40 §3 wrote a simplified staff reader on paper and measured 44 → 34 traversal-specific lines. At full size the same
count goes 149 → 187. Three reasons, and none of them is a defect in the trial's *interface* conclusions:

1. **The trial had one place-sensitive construct; the file has three.** Its `Place` distinguished a body from a numeral
   position. The real adapter distinguishes a body, a form's parentheses, and a chord's brackets, and the three are run
   three different ways — from the end, from the start, and mapped. Two of those three ways did not exist in the trial,
   and they are `body_reading`, `stated_inside`, and `voiced_inside`.
2. **The trial's fold version was allowed to be incomplete.** Its 44 lines include a `group_read` that gives up on
   nested groups. The recursor version's 187 includes the code that does not give up. Comparing a complete program to an
   incomplete one understates the complete one.
3. **The trial had no trivia.** The real lexer emits `Whitespace`, `LineComment`, and `BlockComment`, and the fold
   filtered them into a `Trivia` constructor of the shadow tree — one arm, free, because the shadow tree existed anyway.
   With no shadow tree the filter becomes an explicit `trivial` predicate gating all three step functions. This is a
   small cost and it is a real one, and it is the first thing that broke when the rewrite was first run: all 24 laws
   refused with "this staff does not know this word", pointing at the newline after `syntax staff {`.

The trial's *findings* hold: the fold is incomplete rather than awkward (§4 above), `Pending` survives (§3 above), and
the recursor needs no failure result, no higher rank, and no owner check. What does not hold is the line count, and a
trial on a simplified slice was never in a position to produce one. Recorded as a finding about the trial, as prompt
127dcfag's Read section asks.

## 6. The construction charge, re-run

Prompt 127dcec recorded that on the smallest region this adapter accepts — six header lines and one bar — expansion was
**stopped at 104,016 constructed value nodes against a limit of 100,000**. That was the `from_the_end` closure chain,
and prompt 127dcfaa removed it.

Re-run on the same region (`instrument`, `transposing`, `clef`, `key`, `time`, `spelling`, then `bar (4, 4) { c5/1 }`),
with the meter read directly:

|  | constructed value nodes | evaluation steps |
| --- | ---: | ---: |
| 127dcec, as recorded | 104,016 — **refused**, limit 100,000 | — |
| fold, `64b97f6` | 3,378 | 5,040 |
| recursor, this prompt | 7,217 | 6,688 |

The region that could not be expanded at all now expands at 7% of the node limit and 3% of the step limit.

The 2.1× against the fold is real and is the recursor's. Charges are shallow — `charged_shape` charges an aggregate one
cell plus one field per part, never its parts again — so this is not quadratic; it is simply more cells. Each step now
wraps its result in `Opened(place, pending)`, and each parenthesised group and each bracketed child starts from a
`nothing_pending(here)` that builds a whole blank `Reading`. Under the fold those nodes produced a three-field `Piece`
and the `Pending` was built once per body.

It is left alone. Halving it would mean carrying the place outside the accumulator, and there is nowhere outside the
accumulator for it to go: `recurse_syntax` takes one set of step functions for the whole traversal, so the place has to
ride in `C`. Fourteen times the headroom is not worth contorting the program for, and a note that says so is better
evidence for prompt 127dd than a program bent to a number.

## 7. What this says for the freeze

- The recursor is **necessary**: the parenthesis case in §4 is a program the fold could not express, and it is a program
  the notation wants.
- The recursor is **not** an ergonomic win on its own. The ergonomic wins in §1 came from `?`, record update, and
  expression `if`, and they are larger.
- The one language limitation this rewrite still hits is the missing list constructor (§4), which is a candidate for
  127dd to weigh and not a repair anyone should make quietly.
- `Pending` is the notation's, not the interface's (§3). A future interface change that promises to remove it is
  promising something the domain will not allow.
