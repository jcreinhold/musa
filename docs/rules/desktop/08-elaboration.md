# 08 — The Elaboration Language on Screen

Status: **governing**.

The language grew a middle. A piece no longer only spells notes and expands motifs: it calls functions, instantiates
templates, imports modules, asserts theory, and quotes the kernel, and `docs/rules/language/02-core-calculus.md` §5.8
says those are a proved conservative extension rather than a bolt-on. This document fixes what that means for a screen —
what a composer is shown about a term, where generated music says it came from, how an advisory reading appears without
being mistaken for a mistake, and what a raw kernel document looks like.

One rule stands above the rest and is the reason this file exists: **the interface has no theory of the language.**
Every sentence on this page is one the core wrote. The frontend chooses where it goes.

## 1. Two readers, one fact

A musician asks *what does this do to the music*. A language implementor asks *what is its type, and where does it come
from*. Both questions have one answer, and it is one fact — never two explanations that could disagree.

So each term is shown as a **first sentence and a disclosure**:

```
triad(root: NoteName) -> ChordClass

Builds the three notes of a triad on a root.

▸ language detail
```

The first two lines are the musician's: the signature as the source spells it, and the summary written above the
declaration. The disclosure holds what only an implementor wants — the distinction line that separates the types a
person confuses (`NoteName` against `Pc12`, `Key` against `Scale`, `ChordClass` against `Voicing`), the document the
declaration lives in, and whether it is writable. It is closed by default, it stays closed until asked, and it is the
same fact told at a different depth.

The disclosure is never where the answer is. A term whose only useful sentence is a type name has failed at the source,
and the repair is in the declaration's comment, not on this screen.

## 2. Terms in Source

- **Hover** on a name shows its record: signature, summary, parameters with their types and defaults, the deprecation
  line when the declaration carries one, and the disclosure of §1. A name the compiler did not resolve shows nothing — a
  tooltip that guessed would be the interface having a theory.
- **Completion** offers the names in scope, each with its signature beside it and its summary underneath. The list is
  the core's; it is not filtered, ranked, or re-worded here.
- **Definition** moves the caret to the declaration when it is in this document, and opens the library document when it
  is not (§3).
- **References** selects every resolved use of the name, in the order the resolver met them. Uses, not text matches: two
  different `root`s are two names.
- Both hover and completion answer from the **last valid compile**. While the source does not compile they keep
  answering, from the revision the score is already showing, under the stale rule of `05-states.md` §4. A term panel
  that emptied itself on a half-typed line would be blank exactly while it is being used.

## 3. Library documents

A bundled module has no file on disk. It is opened in a document of the application's own making, and it says so:

- The document is **read-only**, and the source column says so in one line above the text: *Standard library —
  read-only.* Not a lock glyph alone; colour and iconography are never the only signal (`03-interaction.md` §5).
- Its name is the module's own path, as the language spells it: `std::pitch`, not a file name and not a URI.
- Editing keys do nothing in it, and no diagnostic, fix, gesture, or structured edit is offered against it. A composer
  who wants different behaviour writes their own declaration; the library is not a scratch pad.
- Closing it returns to the piece. It is never listed as project material in the contents (`07-the-volume.md`), because
  it is not part of the volume.

## 4. Origin, extended

`04-provenance.md` §3 fixes the Origin row. The elaboration language adds steps to its path, and every step now carries
what kind of step it is and — when it is a place in the source — where it is written.

| Step | Reads | Opens |
| --- | --- | --- |
| **occurrence** | `sigh()` | the call, and the declaration it names |
| **instance** | `make Upper` | the instance, not the template: two instances are two places |
| **transform** | `transpose down P5` | nothing on its own; the block it modifies is the step beside it |
| **specialization** | `with note 3 = a4` | the `with` clause that respelled this note |
| **assertion** | `assert authentic cadence` | the claim, where it is written |
| **splice** | `kernel quote` | the splice that put the material here |

Four rules:

- **A step that is not a place has nothing to reveal.** A transform is an argument to a block and a `repeat` iteration
  is a count. Such a step is still a control — it still selects the music it produced, which is what
  [`04-provenance.md`](04-provenance.md) §3 already made every segment do — but the interface does not invent a span for
  it, and nothing in the source column moves when it is used.
- **The kind is named by the core, not read off the label.** The frontend never parses `transpose down P5` to decide
  what it is looking at. A new kind of step arrives as a new name, not as an "other".
- **Plural origins stay plural.** Music that two expansions produced — a note inside a template inside a repeat, a
  passage a `use` and an `assert` both cover — lists every step. The interface never picks the most convenient one to
  show, and never collapses two into a summary.
- **Quoted kernel material is navigable and not editable.** A score gesture may follow a splice step to the quotation
  that produced it; it may not synthesize an edit inside the quoted term. The offer is simply not made — see §6.

## 5. Assertions and advisory readings

Two different things share a screen, and the whole design is keeping them apart.

**An assertion is a statement the composer wrote.** It is shown where it is written. When it fails, it fails like any
other diagnostic — in the problems list, in the compiler's own words, with its certain fix offered as a control when
there is exactly one (`05-states.md` §5). Nothing new is invented for it.

**An analysis is a reading.** It is asked for, never volunteered, and it appears as findings — never as diagnostics,
never in red, never as a mark on the engraving that reads like an error. `docs/rules/language/07-analysis.md` is the
reason: a cadence with two of its three kinds of evidence is a finding with an absence recorded, not a mistake.

A findings panel shows, per finding and in this order:

1. **Where** — bar and beat, as `Position` sets them everywhere else.
2. **What** — the summary sentence, the core's.
3. **How it stands** — the finding's own standing word: *supported*, *partial*, *contested*. A word, not a colour and
   not a percentage. A number would claim a precision the reading does not have.
4. **The grounds** — each criterion, whether it was satisfied, and what it cites. An unsatisfied ground is shown, not
   hidden: the absence is the information.
5. **The evidence** — an event, a passage, an annotation, or a value in force. The first three are selectable and reveal
   in the source; the fourth is not a place and says so.

Above the findings, the report states its **method**, its **profile** when it ran under one, and its **assumptions**, in
the core's words. A reader who does not accept the assumptions can stop there, which is the point of printing them
first.

A report with no findings says so as an answer — *Nothing found* — and not as an empty pane. Absence is the message only
when nobody asked a question (`05-states.md` §2); here somebody did.

## 6. Raw kernel documents and quoted regions

A kernel document is an interchange format, not a piece. It is shown as itself: the text, plainly labelled *Kernel term
— interchange format*, with no engraving, no transport, and no entry. A quoted region inside a piece carries the same
label where it is quoted.

The rule underneath both: **the application never writes into material it did not spell.** Included, quoted, generated,
and analysis-derived facts are navigable and not editable, and the way that is expressed is that the edit is not
offered. A disabled control the composer keeps reaching for teaches them the application is broken; no control at all
teaches them where the writing happens.

## 7. Keyboard

Everything in this document is reachable without a pointer, and nothing here introduces a new modifier.

- The term disclosure is a `<button>` in the tab order and toggles on `Enter` or `Space`.
- Origin steps are buttons, in path order, left to right — the order they are read in.
- Findings are a list of buttons; `↑`/`↓` move within it, `Enter` selects the evidence, and the selection is the one
  selection the application has (`03-interaction.md` §1).
- A fix offered on a failed assertion is reached and applied exactly like every other fix, and `⌘Z` reverses it.
- Every control here has an accessible name that is its visible text. Where a control's visible text is a symbol, the
  name is the word.

## 8. States

| State | What is shown |
| --- | --- |
| **Nothing selected** | No term panel and no findings panel. The inspector shows what it already shows. |
| **A name that never resolved** | Nothing. Not "unknown": the compiler will have said so in the problems list, and a second sentence in a second place is a second voice. |
| **Analysis running** | The previous report stays on screen. No spinner below one second, as everywhere (`05-states.md` §3). |
| **Analysis refused** | The core's own message, in the top margin in `--chalk`, and the previous report stays. A refusal is not an empty report: a reader shown zero findings would conclude the music is clean. |
| **Stale terms** | Terms keep answering from the revision the score is showing, and the leaf's `--chalk` edge already says the source has moved past it. Nothing here adds a second indicator for that. |
| **A reading of an older score** | A different fact, and one nothing else on screen states: the piece may compile perfectly and the report still be from before the last change. The report carries the revision it read, stays where it is, and says in words that it was read before the last change. It is never emptied and never recomputed on its own — asking is the composer's act (§5). |
| **A library document with no such module** | The document is not opened, and the failure is stated where the request was made. A blank library page would read as an empty module. |

## 9. Restraint

- No node graph, no property grid, no visual programming surface, and no second type or theory engine in TypeScript.
- No new lens. Origin view is still the only one (`04-provenance.md` §5), and the steps this document adds are steps in
  the path it already draws.
- No automatic analysis, no analysis on a timer, and no analysis badge on the page.
- Nothing in this document changes the engraving, the layout, or the motion budget.
