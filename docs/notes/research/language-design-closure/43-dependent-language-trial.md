# Trialling the dependent language before any code implements it

## Purpose

Prompts 129–131 specified a dependent core, records, namespaced enums, traits, operators, methods, `Syntax<Cat>`,
quotation, and syntax patterns. None of it had been written against a program. This note writes it against ten, on
paper, records what each specified mechanism is exercised by, answers the six falsifiers prompt 132 names one by one,
and predicts the size of prompt 145's staff rewrite.

The precedent is [`40-sealed-step-recursor-trial.md`](40-sealed-step-recursor-trial.md): complete programs, no ellipses,
a mechanism-to-program table, and findings permitted to fail the interface. What changes is the scope — that trial
questioned one operation, this one questions a language.

Nothing here amends [`docs/rules/constitution.md`](../../../rules/constitution.md) or
[`docs/rules/obligations.md`](../../../rules/obligations.md). It repairs the four candidate documents of
[`docs/rules/language/`](../../../rules/language/README.md) where a program contradicted them, and each repair is named
in §13 with the program that forced it.

## Verdict

**The design survives the trial and eight things change.** No falsifier fired. Every one of the eight changes makes the
specification smaller or more exact, which is what a trial before implementation is for.

1. **Quotation is the largest single win, and it is larger than prompt 131 claimed.** `call1`–`call7` and every
   hand-written `syntax_group`/`syntax_token` assembly go — 119 lines of helper and twenty nested call sites become
   twenty one-line quotes — and with them go **seven of the fourteen phase operations** (§9), not just the role
   integers. §1.
2. **The dispatch table has three halves, not two.** Shapes become patterns and token kinds become `TokenKind`, as
   `11-quotation.md` §4 says; the fifteen *notation keywords* are neither, and the answer for them is a declared enum
   and one lookup, not a quote pattern. §2.
3. **`Syntax<Cat>` pays for itself only at the splice boundary, and needs a third introduction form.** Under quotation,
   construction is sound because the parser read it, so the index buys nothing there. What it buys is that a *spliced
   source node* must be certified — which needs `as_expression`, a checked parse, and which moves the "this is not a
   pitch" diagnostic from a compiler-defect gate to the line the composer's mistake is on. §1.2, §13 R6.
4. **`Cat` should have two cases.** No program in the trial constructs a `Syntax<Item>` or a `Syntax<Pattern>`. Two
   unused cases carry two thirds of prompt 138's index-soundness obligation. §9.
5. **K is not exercised and should not be admitted.** No program in the trial unifies an index: `Cat` appears only at
   closed constructors, and no program needs `Vec A n`. Every type the ten programs declare has decidable equality, so
   Hedberg gives K as a theorem where it is wanted. `02-core-calculus.md` §1.4 nominated this prompt for exactly this
   decision. §10.
6. **Three specified spellings are unwritable in the grammar that specifies them**, and a program hit each: a
   multi-parameter function type (`Iterable`'s own declaration), a named-field enum case in literal and pattern position
   (`Refusal::Refused { node, why }`), and a splice holding a computed expression (`document_read`). §13 R2–R4.
7. **The bracket hole closes.** Note 41 §4 left a nested group inside `[ … ]` contributing no pitch, attributable to "a
   list cannot be built". `Buildable` closes it, and the closure is one fold. §2.3.
8. **`Pending` survives a third time.** Note 40 predicted it, note 41 confirmed it against the fold, and it survives
   records too: seven argument slots are what the notation costs. What records remove is the *fourteen destructures*,
   not the seven fields. §3.

One gap is outside the pass. **Hidden constructors have no spelling**, and no prompt from 133 to 149 adds one — three of
note 28's five programs rely on them. §14 says what that costs and what the next prompt is.

## 0. How the programs were checked

Every program below was read against `02-core-calculus.md`, `01-surface.md`, `10-traits.md`, and `11-quotation.md` **as
prompts 129–131 left them**, not as intended. Where a program only worked under a charitable reading, that is recorded
as a finding rather than smoothed over; §13 is the list.

Nothing was type-checked by a compiler, because none of this language exists yet. That is a real limitation and it is
the difference between this trial and note 40, which could stand types in and run `musa check`. What can be checked on
paper is what note 40 §2 checked with stand-ins minus the parts that needed a checker: arity, argument order, pattern
coverage, fold direction, the scope of `with`, whether a spelling exists in the grammar, and whether a constraint is
resolvable at a known head. What cannot be checked is conversion, termination, and coverage in the presence of indices.
Prompts 133–137 owe that evidence and §7 says which mechanism owes which.

The measurements are against `stdlib/src/adapters/staff.musa` at prompt 127dcfb's commit: **2,404 lines, 93,252 bytes,
1,814 non-comment non-blank lines**. Note 41 measured 2,158 lines before the `// ---- writing` section existed; that is
not the number prompt 145 is gated on.

**A note on prompt 132's own arithmetic.** Its item 1 names the `// ---- writing` section, which in the file is the
printer; `call1`–`call7` are in `// --------- emitting`. Both are trialled in §1, because the question item 1 asks is
about the second and the section it names is the first, and leaving either out would be a rewrite prompt 145 was
surprised by. Its items 5–8 allot four slots to note 28's five programs, "minus any the corpus has since retired". None
has been retired — what changed is that `Music` was deleted by prompt 127a and every program that named it now names
`EventTrack[WrittenTime, ScoreFact]` — so this trial writes ten programs rather than nine.

## 1. Program one — staff construction

### 1.1 The emitting section

The current section is lines 353–588. `named`, `call1`, `call2`, `call3`, `call4`, `call5`, and `call7` are 119 lines of
one idea repeated at seven arities: a layout group holding an identifier beside a parenthesis group, with `Comma` tokens
written out and a child number threaded through every `syntax_built`. Rewritten:

```musa
// --------------------------------------------------------------- emitting

// Nothing said yet.
let no_head = Head {
    instrument = Given::Absent,
    shift = Given::Absent,
    written_clef = Given::Absent,
    tonality = Given::Absent,
    beats = Given::Absent,
    spelled = Given::Absent,
};

// Nothing read at all.
fn nothing_read(here: NodePath) -> Reading {
    Reading {
        items = quote at here { NoItems },
        span = 0,
        opens = Sound::Silent,
        hangs = Hanging::Closed,
        head = no_head,
        refusal = Refusal::NoRefusal,
    }
}

// A reader that has read nothing, ready to be threaded through children.
fn nothing_pending(here: NodePath) -> Pending {
    Pending {
        read = nothing_read(here),
        length = Length::NoLength,
        dots = Dots::NoDots,
        tying = Tying::Untied,
        numbers = Numbers::NoNumbers,
        taken = Taken::NoBody,
        voiced = Voiced::NoVoices,
        words = Words::NoWords,
    }
}

// The anchor of the node at `here`, or a zero where the compiler has no range.
// An anchor is a number whose meaning is the compiler's table: this adapter
// hands one over and can read nothing from it.
fn anchored(region: Syntax<TokenTree>, here: NodePath) -> Syntax<Expr> {
    match syntax_anchor(region, here) {
        Some(node) -> node,
        None -> quote at here { 0 },
    }
}

// `Tie`, as the package spells it.
fn tie_of(here: NodePath, tying: Tying) -> Syntax<Expr> {
    match tying {
        Tying::Untied -> quote at here { Untied },
        Tying::Tied { node } -> quote at here { TiedOn },
    }
}

// How many dots the emitted value carries, as the numeral it is written with.
fn dot_count(here: NodePath, dots: Dots) -> Syntax<Expr> {
    match dots {
        Dots::NoDots -> quote at here { 0 },
        Dots::OneDot -> quote at here { 1 },
        Dots::TwoDots -> quote at here { 2 },
        Dots::ThreeDots -> quote at here { 3 },
    }
}

// `WrittenDuration`, as the package spells it.
fn written_duration(here: NodePath, length: Length) -> Option<Syntax<Expr>> {
    match length {
        Length::NoLength -> None,
        Length::Divided { numeral, dots, span } -> Some(quote at here {
            NoteValue($numeral, ${ dot_count(here, dots) })
        }),
        Length::Spanned { numeral, span } -> Some(quote at here { ExactSpan($numeral) }),
    }
}

// The exact span a stated written value covers.
fn stated_span(length: Length) -> Option<Ratio> {
    match length {
        Length::NoLength -> None,
        Length::Divided { numeral, dots, span } -> Some(span),
        Length::Spanned { numeral, span } -> Some(span),
    }
}

// `Beats(count, unit)`, from the two numbers a form stated.
fn beats_of(here: NodePath, numbers: Numbers) -> Option<Syntax<Expr>> {
    match numbers {
        Numbers::NoNumbers -> None,
        Numbers::OneNumber { first, value } -> None,
        Numbers::TwoNumbers { first, count, second, unit } ->
            Some(quote at here { Beats($first, $second) }),
    }
}

// What a stated meter measures, in whole notes.
fn beats_span(numbers: Numbers) -> Option<Ratio> {
    match numbers {
        Numbers::NoNumbers -> None,
        Numbers::OneNumber { first, value } -> None,
        Numbers::TwoNumbers { first, count, second, unit } -> match count / unit {
            Ok(span) -> Some(span),
            Err(why) -> None,
        },
    }
}
```

Four things in that block are the whole finding.

**`named` and `call1`–`call7` are gone and nothing replaces them.** There is no `callN` under another name, no helper
that assembles a call, and no arity to add an eighth of. A call is written as a call.

**Every role integer is gone.** The block above contains no `syntax_built`, so it allocates nothing; the identity of
each literal node is `Derived { origin, quotation, path }`, computed (`11-quotation.md` §3). The whole file's
twenty-seven distinct hand-allocated values go with it.

**A computed spelling becomes a `match` over quotes, and that is an improvement.** `tie_of` and `dot_count` used to
build an identifier or an integer token out of a `Text` chosen by a `match`; now the `match` chooses between quotes, and
each spelling is read by the real parser rather than trusted. A misspelt `"TiedOnn"` was a token nobody checked; a
misspelt `quote at here { TiedOnn }` is an identifier that fails to resolve where the expansion lands.

**Two sites need a splice holding an expression.** `${ dot_count(here, dots) }` and, in §1.2, `${ anchored(region, here)
}` are values computed at the splice, and a function body is a block holding exactly one expression (`01-surface.md` §1)
so there is nowhere to bind them. `11-quotation.md` §2 says `$x` "splices one value" and its examples are all bound
names; the grammar of the splice has to admit the expression. §13 R4.

### 1.2 The twenty call sites, and what the index is actually for

The file's twenty `callN` sites become twenty quotes. Three are representative and the third is the finding.

```musa
// call3(here, 1, "Sounded", anchored(region, here), event, items)
quote at here { Sounded(${ anchored(region, here) }, $event, $items) }

// call1(spot, 3, "Rest", value)
quote at spot { Rest($value) }

// call5(spot, 12, "Tuplet", anchored(region, spot), played, against, items, after)
quote at spot { Tuplet(${ anchored(region, spot) }, $played, $against, $items, $after) }
```

The third kind is `note_read`'s, and it does not go through:

```musa
// call3(spot, 2, "Note",
//   syntax_token(syntax_built(spot, 49, 0), "PitchLiteral", spelling),
//   value,
//   tie_of(spot, tying))
```

The first argument rebuilds the composer's pitch literal out of its spelling text. Quotation refuses this by design:
"there is no operation from `Text` to `Syntax<c>`" and "a splice stands where a whole node stands and never inside one"
(`11-quotation.md` §2), and unlike `Untied`/`TiedOn` there is no finite set of spellings to `match` over — a pitch
literal is whatever the composer wrote.

The right program does not rebuild the token at all. It **splices the one the composer wrote**:

```musa
quote at spot {
    Note(${ as_expression(syntax_at(region, spot))? }, $value, ${ tie_of(spot, tying) })
}
```

This is better than what it replaces, for a reason that has nothing to do with line count: `syntax_at` exists so that "a
transformer preserves input with its source information intact", and the spliced node keeps its `Original` source info
(`11-quotation.md` §3), so the emitted `Note`'s pitch points at the composer's own `c5` rather than at a derived node
that happens to have the same text. Origin, `edit`, and `print` all read better for it.

But it needs `as_expression`, and that is the trial's finding about the index. `syntax_at` answers a
`Syntax<TokenTree>`; the argument position inside `Note(…)` demands `Syntax<Expr>`. `11-quotation.md` §1's acceptance
rule runs the other way — a `TokenTree` *position* accepts any category — so nothing in the specification lets a source
node into an expression position.

Weigh the two answers rather than reaching for one:

- **Drop the index.** Under quotation every constructed node is built by the parser, so the mistake §1 says the index
  catches — building a non-expression where an expression was wanted — is already impossible. On constructed nodes the
  index is genuinely redundant, and this is the strongest argument against it that exists.
- **Add the third introduction form.** `11-quotation.md` §1 already says a refined claim is introduced only by an
  operation that establishes it, and lists two such operations. A checked parse is a third, and it is the *definition*
  of the index rather than an addition to it. What it buys is exactly the splice boundary: `bar (4, 4) { { } }` puts a
  brace group where a pitch belongs, and with `as_expression` the adapter answers that at the splice — the line the
  composer's mistake is on — instead of letting a malformed tree reach `checked_expression` and be reported against the
  region.

The second is right, and it also *reduces* the operation count: `checked_expression(subject)` already runs the parser
over a whole result, so `as_expression` generalizes it to a sub-node rather than adding an operation beside it. §13 R6
records the repair.

### 1.3 The printer, which quotation does not touch

`// ---- writing` (lines 2168–2404) is the other direction: `StaffEvent` and `StaffItem` to `Text`. It names no phase
operation and builds no syntax, so quotation is irrelevant to it. What the new language changes there is smaller and
worth stating so prompt 145 does not expect otherwise:

```musa
// spaced: text_equal(first, "") becomes ==
fn spaced(first: Text, second: Text) -> Text {
    if first == "" { second } else if second == "" { first } else { text_join([first, " ", second]) }
}

// written_value: option_fold is gone, because Option is an ordinary enum
fn written_value(held: WrittenDuration) -> Result<Text, Text> {
    match held {
        WrittenDuration::NoteValue { division, dots } ->
            Ok(text_join(["/", nat_literal(division), dotted(dots)])),
        WrittenDuration::ExactSpan { span } -> match ratio_literal(span) {
            Some(spelt) -> Ok(text_join(["(", spelt, ")"])),
            None -> Err("this staff writes no exact span below zero, and this note holds one"),
        },
    }
}

// voiced_pitches: a fold becomes a method on its receiver
fn voiced_pitches(voices: List<Pitch>) -> Text {
    voices.fold_from_start("", fn (sofar: Text, sung: Pitch) -> Text { spaced(sofar, pitch_literal(sung)) })
}
```

Three `text_equal` sites become `==`, two `option_fold` calls become `match`, and four `list_fold_*` calls become
methods. The printer's 236 lines lose about 14. **Quotation is a staff-reader win, not a staff-adapter-wide win**, and
the printer is half the file.

### 1.4 Count

| Definition | Now | On the new language |
| --- | ---: | ---: |
| `named`, `call1`–`call7` | 119 | 0 |
| `no_items`, `nothing_read`, `nothing_pending`, `no_head` | 19 | 24 |
| `anchored` | 7 | 6 |
| `tie_of` | 9 | 6 |
| `dot_count` (new; was inline in `written_duration`) | 0 | 8 |
| `written_duration` | 22 | 11 |
| `stated_span`, `beats_of`, `beats_span` | 30 | 25 |
| the twenty call sites, counted as written | 62 | 24 |
| **emitting plus its call sites** | **268** | **104** |
| the printer (`// ---- writing`) | 236 | 222 |

The record literals are longer than the positional constructions they replace — `nothing_pending` goes from twelve lines
to eleven with every field named, and `no_head` from one line to eight. That is the price §1.2 of `01-surface.md` argues
for and it is paid here in full; what buys it back is the fourteen destructures in §3.

## 2. Program two — staff dispatch

### 2.1 The table has three halves

`11-quotation.md` §4 says the dispatch table has two halves and quote patterns remove one. Against the file that is
wrong, and the third half is the largest:

| Half | Where | Sites | What removes it |
| --- | --- | ---: | --- |
| shape — is this a group, and which delimiter | `group_read`, `token_read` | 5 `text_equal` | a quote pattern, for the shapes Musa's grammar has |
| token kind — `PitchLiteral`, `Integer`, `Rational`, trivia | `body_token`, `stated_token`, `trivial` | 8 `text_equal` | prompt 138's typed `TokenKind` and `==` |
| **notation keyword** — `bar`, `rest`, `slur`, `clef`, … | `word_read`, `clef_named`, `spelling_named` | 8 `text_equal` over 15 spellings | **neither** |

The third half resists both. A keyword is not a shape: `bar (4, 4) { c5/1 }` is three siblings of a layout group, and
Musa's grammar has no form that is an identifier beside a parenthesis group beside a block, so a quote pattern written
at `Expr` cannot describe it. And it is not a token kind: `bar` and `slur` are both `Identifier`.

A quote pattern *can* match a bare identifier — `quote { bar }` — and fifteen of those would replace the fifteen string
literals. It is the wrong answer, and saying why is the useful part: `quote { bra }` is a perfectly good quote of a
perfectly good identifier, so a misspelling stays an arm that silently never matches, which is the exact failure the
`text_equal` chain already has. Nothing is gained and a `match` over shapes is not coverage-checked.

The right answer is an ordinary enum and one lookup:

```musa
// Every word the staff spelling has. A word either opens a form or states one
// of the six facts a document carries in its head.
enum StaffWord {
    Rest, Bar, Pickup, Slur, Tuplet, Grace, Repeat, Ending,
    Time, Meter, Instrument, Transposing, Clef, Spelling, Key,
}

// The one place a staff keyword is spelled, and the only place a misspelling
// can hide.
fn staff_word(spelling: Text) -> Option<StaffWord> {
    if spelling == "rest" { Some(StaffWord::Rest) }
    else if spelling == "bar" { Some(StaffWord::Bar) }
    else if spelling == "pickup" { Some(StaffWord::Pickup) }
    else if spelling == "slur" { Some(StaffWord::Slur) }
    else if spelling == "tuplet" { Some(StaffWord::Tuplet) }
    else if spelling == "grace" { Some(StaffWord::Grace) }
    else if spelling == "repeat" { Some(StaffWord::Repeat) }
    else if spelling == "ending" { Some(StaffWord::Ending) }
    else if spelling == "time" { Some(StaffWord::Time) }
    else if spelling == "meter" { Some(StaffWord::Meter) }
    else if spelling == "instrument" { Some(StaffWord::Instrument) }
    else if spelling == "transposing" { Some(StaffWord::Transposing) }
    else if spelling == "clef" { Some(StaffWord::Clef) }
    else if spelling == "spelling" { Some(StaffWord::Spelling) }
    else if spelling == "key" { Some(StaffWord::Key) }
    else { None }
}
```

That is not a deletion — fifteen string literals stay, once each — and it is still worth doing, because the *dispatch*
becomes exhaustive:

```musa
fn word_read(
    region: Syntax<TokenTree>,
    spot: NodePath,
    spelling: Text,
    kind: TokenKind,
    later: Pending,
) -> Pending {
    match staff_word(spelling) {
        None -> holding_word(region, later, Said { spot = spot, kind = kind, spelling = spelling }),
        Some(word) -> match word {
            StaffWord::Rest -> rest_read(region, spot, later, later.length),
            StaffWord::Bar -> bar_read(region, spot, later, later.numbers, later.taken),
            StaffWord::Pickup -> bar_read(region, spot, later, later.numbers, later.taken),
            StaffWord::Slur -> slur_read(region, spot, later, later.taken),
            StaffWord::Tuplet -> tuplet_read(region, spot, later, later.numbers, later.taken),
            StaffWord::Grace -> grace_read(region, spot, later, later.voiced),
            StaffWord::Repeat -> repeated_read(
                region, spot, later, "Repeat", later.numbers, later.taken,
                "this repeat does not say how many times it holds what",
            ),
            StaffWord::Ending -> repeated_read(
                region, spot, later, "Ending", later.numbers, later.taken,
                "this ending does not say which pass it holds what for",
            ),
            StaffWord::Time -> time_header(region, spot, later, later.numbers),
            StaffWord::Meter -> meter_read(region, spot, later, later.numbers),
            StaffWord::Instrument ->
                literal_header(region, spot, later, "instrument", TokenKind::String, later.words),
            StaffWord::Transposing ->
                literal_header(region, spot, later, "transposing", TokenKind::IntervalLiteral, later.words),
            StaffWord::Clef -> named_header(region, spot, later, "clef", later.words),
            StaffWord::Spelling -> named_header(region, spot, later, "spelling", later.words),
            StaffWord::Key -> key_header(region, spot, later, later.words),
        },
    }
}
```

Adding a sixteenth staff word now fails to compile in `word_read` until it is read, which the `_ ->` arm of the string
version never did. The role integers `10` and `11` that `repeated_read` took are gone with everything else.

### 2.2 The two halves that do go

```musa
// Was: three text_equal calls
fn trivial(kind: TokenKind) -> Bool {
    match kind {
        TokenKind::Whitespace -> true,
        TokenKind::LineComment -> true,
        TokenKind::BlockComment -> true,
        _ -> false,
    }
}

// Was: match kind { "Tilde" -> …, "Dot" -> …, "Integer" -> …, … }
fn body_token(
    region: Syntax<TokenTree>,
    spot: NodePath,
    kind: TokenKind,
    spelling: Text,
    later: Pending,
) -> Pending {
    match kind {
        TokenKind::Tilde -> later with { tying = Tying::Tied { node = syntax_at(region, spot) } },
        TokenKind::Dot -> match one_more_dot(later.dots) {
            Some(more) -> later with { dots = more },
            None -> refusing(later, syntax_at(region, spot), "this value carries too many dots"),
        },
        TokenKind::Integer -> match syntax_number(syntax_at(region, spot)) {
            None -> refusing(later, syntax_at(region, spot), "this is not a written value"),
            Some(value) -> match dotted_factor(later.dots) / value {
                Ok(span) -> holding_length(region, spot, later, Length::Divided {
                    numeral = as_expression(syntax_at(region, spot))?,
                    dots = later.dots,
                    span = span,
                }),
                Err(why) -> refusing(later, syntax_at(region, spot), "this is not a written value"),
            },
        },
        TokenKind::PitchLiteral -> note_read(region, spot, later, spelling, later.length, later.tying),
        TokenKind::Slash -> later,
        TokenKind::Comma -> later,
        _ -> word_read(region, spot, spelling, kind, later),
    }
}
```

The `Divided` numeral is now the composer's own integer token spliced through `as_expression`, not a token rebuilt from
its spelling under role `53`. Same reasoning as §1.2, same improvement in provenance, same dependence on R6.

And the shape half, where a quote pattern does apply — the region's own braces:

```musa
fn entered(
    region: Syntax<TokenTree>,
    state: State,
    here: NodePath,
    delimiter: Delimiter,
    kids: List<SyntaxStep<State, State>>,
) -> State {
    match delimiter {
        Delimiter::Braces -> taking(here, state, body_reading(here, kids)),
        Delimiter::Parentheses -> reopened(state, stated_inside(here, kids)),
        Delimiter::Brackets -> reopened(state, voiced_pending(here, region, kids)),
    }
}
```

That is a `match` on prompt 138's typed `Delimiter`, not a quote pattern. **No quote pattern appears anywhere in the
staff adapter.** The reason is stated once and it settles §4's claim: the adapter reads a token tree of *staff*
notation, and a quote pattern is written in *Musa*, so it can only describe the shapes the two grammars happen to share.
For staff there is one — a call — and the adapter never needs to recognize one.

### 2.3 The bracket hole, closed

Note 41 §4 left half the nested-group hole open: a group written inside `[ … ]` contributes no pitch, because
`voiced_inside` is a `map` over the group's children and a `map` is length-preserving, and the phase language had no
operation that builds or joins a list. `Buildable` is that operation:

```musa
// Every pitch a chord's brackets hold, however deeply a group nests them.
fn voiced_inside(here: NodePath, kids: List<SyntaxStep<State, State>>) -> List<State> {
    kids.fold_from_start(
        [],
        fn (sofar: List<State>, kid: SyntaxStep<State, State>) -> List<State> {
            said_states(run_syntax_step(Opened(Place::InBrackets, nothing_pending(here)), kid))
                .fold_from_start(sofar, fn (into: List<State>, one: State) -> List<State> { into.push(one) })
        },
    )
}
```

`push` is `Buildable<List<A>, A>`'s (`10-traits.md` §8); `fold_from_start` is `Iterable`'s. The inner fold is
concatenation written out, and prompt 141 will name it. The hole the file's own header has listed as a limitation since
prompt 127dcfa closes in five lines, and it closes because a list can be built rather than because anything about the
traversal changed — exactly as note 41 §4 predicted.

### 2.4 Count

| Definition | Now | On the new language |
| --- | ---: | ---: |
| `word_read` | 60 | 41 + 21 (`staff_word`) + 5 (`enum StaffWord`) |
| `body_token` | 51 | 33 |
| `stated_token` | 45 | 25 |
| `one_more_number` | 20 | 16 |
| `trivial` | 7 | 8 |
| `token_read`, `missing_read`, `group_read`, `entered` | 60 | 48 |
| `clef_named`, `spelling_named` | 22 | 22 |
| `voiced_inside` | 6 | 10 |
| **dispatch** | **271** | **229** |

The dispatch table shrinks by 16%, which is much less than §1's 61%, and the reason is entirely the third half: fifteen
notation keywords are the notation's own words and only text can say them. What the new language buys here is not
brevity but coverage — `word_read` and `body_token` are exhaustive matches over declared types, so the twenty-one
`text_equal` sites over thirteen string literals become eight over fifteen, all of them inside two lookup functions
where a misspelling is visible.

## 3. Program three — `Pending` as a record

The declaration barely changes:

```musa
// What has been read since the last word, reading from the end.
record Pending {
    read: Reading;
    length: Length;
    dots: Dots;
    tying: Tying;
    numbers: Numbers;
    taken: Taken;
    voiced: Voiced;
    words: Words;
}

// What one stretch of notation reads to.
record Reading {
    items: Syntax<Expr>;
    span: Ratio;
    opens: Sound;
    hangs: Hanging;
    head: Head;
    refusal: Refusal;
}
```

**Seven argument slots survive.** Note 40 finding 2 predicted it against the fold, note 41 §3 confirmed it at full size,
and it survives records for the third time and for the same reason: every slot is an argument that arrived before the
word that claims it, which is a fact about reading `bar (4, 4) { … }` from its end and about emitting right-nested
`Sounded(anchor, event, after)`. A record does not change either.

What records remove is the fourteen sites that destructure all eight fields to read one or two. `after_item` is the
worst, and it is where the two features compound:

```musa
// Now — 33 lines, one destructure, one positional Reading rebuild
fn after_item(
    later: Pending,
    items: Syntax,
    added: Ratio,
    opens: Sound,
    hangs: Hanging,
    trouble: Refusal,
) -> Pending {
    match later {
        Pending(read, length, dots, tying, numbers, taken, voiced, words) -> match read {
            Reading(was, span, was_open, was_hanging, head, refusal) -> later with {
                read = Reading(
                    items,
                    match ratio_add(span, added) { Ok(total) -> total, Err(why) -> span },
                    opens,
                    hangs,
                    head,
                    first_refusal(trouble, refusal),
                ),
                length = NoLength,
                dots = NoDots,
                tying = Untied,
                numbers = NoNumbers,
                taken = NoBody,
                voiced = NoVoices,
                words = NoWords
            },
        },
    }
}
```

```musa
// On the new language — 20 lines, no destructure, four path updates
fn after_item(
    later: Pending,
    items: Syntax<Expr>,
    added: Ratio,
    opens: Sound,
    hangs: Hanging,
    trouble: Refusal,
) -> Pending {
    later with {
        read.items = items,
        read.span = match later.read.span + added { Ok(total) -> total, Err(why) -> later.read.span },
        read.opens = opens,
        read.hangs = hangs,
        read.refusal = first_refusal(trouble, later.read.refusal),
        length = Length::NoLength,
        dots = Dots::NoDots,
        tying = Tying::Untied,
        numbers = Numbers::NoNumbers,
        taken = Taken::NoBody,
        voiced = Voiced::NoVoices,
        words = Words::NoWords,
    }
}
```

Two rules of `01-surface.md` §1 are load-bearing in that rewrite and both hold. The right-hand sides read the scope
*around* the update, so `later.read.span` is written out and is not the field's own name — which is what makes the line
readable rather than ambiguous. And a path costs one construction per segment, so five updates under `read.` build one
`Reading` and one `Pending`, not five of each.

`faulting` and `refusing` collapse the same way. Across the file:

|  | Now | On the new language |
| --- | ---: | ---: |
| eight-field destructures of `Pending` | 14 | 0 |
| positional `Reading(…)` rebuilds | 20 | 0 |
| `with { … }` updates | 23 | 26 |
| lines in the eleven functions that only destructured to read one field | 176 | 108 |

**`Refusal::Refused { node, why }` is where the grammar does not reach.** `Refusal` is an enum with a two-field case,
and `01-surface.md` §1.3 gives exactly that spelling for `Reading::Refused { at = p, why = w }` — but the EBNF's
`record-literal` and `record-pattern` take a `type-name` as their head, and `Refusal::Refused` is a `qualified`. The
production admits it only by letting `module-path` hold a capitalized segment, which §1.5 forbids in the same document.
§13 R3.

## 4. Program four — `document_read`'s `call7`

The single worst call site is 42 lines: `call7` at role `19`, seven arguments, six of them a five-line
`stated_field(…)?`.

```musa
fn document_read(
    region: Syntax<TokenTree>,
    here: NodePath,
    read: Reading,
) -> Result<Syntax<Expr>, Fault> {
    match read.refusal {
        Refusal::Refused { node, why } -> Err(Fault { node = node, why = why }),
        Refusal::NoRefusal -> match read.hangs {
            Hanging::Hangs { spelling, node } -> Err(Fault {
                node = node,
                why = "this staff ties a note to nothing: no note sounds after it",
            }),
            // Six things the head has to have said, asked in the order a reader
            // would ask them. Each `?` hands the first refusal straight back
            // out, so what is left on the page is the document being built.
            Hanging::Closed -> Ok(quote at here {
                Document(
                    ${ stated_field(region, here, read.head.instrument,
                        "this staff does not say what instrument it is written for")? },
                    ${ stated_field(region, here, read.head.shift,
                        "this staff does not say how far its written pitch sits from its sounding pitch")? },
                    ${ stated_field(region, here, read.head.written_clef,
                        "this staff does not say what clef it is read in")? },
                    ${ stated_field(region, here, read.head.tonality,
                        "this staff does not say what key it is written in")? },
                    ${ stated_field(region, here, read.head.beats,
                        "this staff does not say what time it opens in")? },
                    ${ stated_field(region, here, read.head.spelled,
                        "this staff does not say how a realized span is spelled")? },
                    $read.items,
                )
            }),
        },
    }
}
```

42 lines become 33, which is the smallest proportional win in the trial, and the prompt asked whether it is "obviously
shorter *and* obviously more correct". It is not obviously shorter. It is obviously more correct, in three specific
ways:

- the shape `Document(a, b, c, d, e, f, g)` is on the page as a call rather than as `call7(here, 19, "Document", …)`,
  where the arity is in the function name, the identity is in an integer, and the commas are seven `syntax_token`
  arguments in a helper defined 1,600 lines earlier;
- the two nested `match`es on `refusal` and `hangs` become two arms each with a named field, so the three refusals are
  three lines rather than three destructurings; and
- role `19` is gone, and with it the question nobody could answer of whether `19` is used anywhere else.

**This site is the second forcing case for R4.** There are seven arguments and no name to bind any of them: a function
body is a block holding exactly one expression, so the six `stated_field(…)?` calls have to be written where they are
used. Either `$` admits an expression, or `01-surface.md` §1 admits a `let` inside a block, or the quote needs a
seven-parameter helper — which is `call7` with better parameter names. The first is the smallest change and the one the
sibling quotation form already spells (`01-surface.md` §7's `${e}`).

`?` inside a splice inside a quote is checked exactly as `01-surface.md` §1 says: the answer the `?` leaves is the
enclosing function's, the splice is not an argument to a call that would swallow it, and the six of them nest in written
order so a staff missing three header fields reports the leftmost.

## 5. Programs five to nine — note 28's corpus

These five were chosen against a language that no longer exists, which is what makes them evidence. Every declaration
appears; the surface changes are mechanical except where noted, and the notes are the findings.

### 5.1 Tonal harmony

```musa
// trial/tonal.musa
import std::list as list;
import std::notation::staff as staff;

enum Spelling { C, CSharp, DFlat, D, EFlat, E, F, FSharp, G, AFlat, A, BFlat, B }
enum ChordSymbol { CMajor, CMinor, DFlatMajor, FMajor, GSeven, AMinor, BDiminished }
enum VoicingPolicy { Close, OpenBass }
enum Key { CMajorKey, CMinorKey }
enum ScaleDegree { One, Four, Five, Six, Seven, FlatTwo }
enum HarmonicFunction { Tonic, Predominant, Dominant, Other }
enum TonalError { CannotAnalyzeAnonymousChord }

record FunctionClaim {
    key: Key;
    symbol: ChordSymbol;
    degree: ScaleDegree;
    function: HarmonicFunction;
    evidence: Text;
}

enum Chord {
    NamedChord(ChordSymbol, List<Spelling>),
    AnonymousChord(List<Spelling>),
}

enum Voicing { VoicingValue(List<staff::WrittenPitch>) }

fn tones(symbol: ChordSymbol) -> List<Spelling> {
    match symbol {
        ChordSymbol::CMajor -> [Spelling::C, Spelling::E, Spelling::G],
        ChordSymbol::CMinor -> [Spelling::C, Spelling::EFlat, Spelling::G],
        ChordSymbol::DFlatMajor -> [Spelling::DFlat, Spelling::F, Spelling::AFlat],
        ChordSymbol::FMajor -> [Spelling::F, Spelling::A, Spelling::C],
        ChordSymbol::GSeven -> [Spelling::G, Spelling::B, Spelling::D, Spelling::F],
        ChordSymbol::AMinor -> [Spelling::A, Spelling::C, Spelling::E],
        ChordSymbol::BDiminished -> [Spelling::B, Spelling::D, Spelling::F],
    }
}

fn raise_semitone(spelling: Spelling) -> Spelling {
    match spelling {
        Spelling::C -> Spelling::CSharp,
        Spelling::CSharp -> Spelling::D,
        Spelling::DFlat -> Spelling::D,
        Spelling::D -> Spelling::EFlat,
        Spelling::EFlat -> Spelling::E,
        Spelling::E -> Spelling::F,
        Spelling::F -> Spelling::FSharp,
        Spelling::FSharp -> Spelling::G,
        Spelling::G -> Spelling::AFlat,
        Spelling::AFlat -> Spelling::A,
        Spelling::A -> Spelling::BFlat,
        Spelling::BFlat -> Spelling::B,
        Spelling::B -> Spelling::C,
    }
}

fn written_pitch(spelling: Spelling, register: Nat) -> staff::WrittenPitch {
    match spelling {
        Spelling::C -> staff::pitch("c", "natural", register),
        Spelling::CSharp -> staff::pitch("c", "sharp", register),
        Spelling::DFlat -> staff::pitch("d", "flat", register),
        Spelling::D -> staff::pitch("d", "natural", register),
        Spelling::EFlat -> staff::pitch("e", "flat", register),
        Spelling::E -> staff::pitch("e", "natural", register),
        Spelling::F -> staff::pitch("f", "natural", register),
        Spelling::FSharp -> staff::pitch("f", "sharp", register),
        Spelling::G -> staff::pitch("g", "natural", register),
        Spelling::AFlat -> staff::pitch("a", "flat", register),
        Spelling::A -> staff::pitch("a", "natural", register),
        Spelling::BFlat -> staff::pitch("b", "flat", register),
        Spelling::B -> staff::pitch("b", "natural", register),
    }
}

fn build(symbol: ChordSymbol) -> Chord { Chord::NamedChord(symbol, tones(symbol)) }

fn members(chord: Chord) -> List<Spelling> {
    match chord {
        Chord::NamedChord(symbol, named) -> named,
        Chord::AnonymousChord(anonymous) -> anonymous,
    }
}

fn transpose_up_semitone(chord: Chord) -> Chord {
    Chord::AnonymousChord(members(chord).map(fn (member: Spelling) -> Spelling { raise_semitone(member) }))
}

fn voice(policy: VoicingPolicy, chord: Chord) -> Voicing {
    match policy {
        VoicingPolicy::Close -> Voicing::VoicingValue(
            members(chord).map(fn (member: Spelling) -> staff::WrittenPitch { written_pitch(member, 4) }),
        ),
        VoicingPolicy::OpenBass -> match members(chord) {
            [] -> Voicing::VoicingValue([]),
            [bass, ..upper] -> Voicing::VoicingValue(
                upper
                    .map(fn (member: Spelling) -> staff::WrittenPitch { written_pitch(member, 4) })
                    .fold_from_end(
                        [written_pitch(bass, 3)],
                        fn (one: staff::WrittenPitch, sofar: List<staff::WrittenPitch>) -> List<staff::WrittenPitch> {
                            sofar.push(one)
                        },
                    ),
            ),
        },
    }
}

fn degree(key: Key, symbol: ChordSymbol) -> ScaleDegree {
    match symbol {
        ChordSymbol::CMajor -> ScaleDegree::One,
        ChordSymbol::CMinor -> ScaleDegree::One,
        ChordSymbol::DFlatMajor -> ScaleDegree::FlatTwo,
        ChordSymbol::FMajor -> ScaleDegree::Four,
        ChordSymbol::GSeven -> ScaleDegree::Five,
        ChordSymbol::AMinor -> ScaleDegree::Six,
        ChordSymbol::BDiminished -> ScaleDegree::Seven,
    }
}

fn function_of(symbol: ChordSymbol, after: Option<ChordSymbol>) -> HarmonicFunction {
    match symbol {
        ChordSymbol::CMajor -> HarmonicFunction::Tonic,
        ChordSymbol::CMinor -> HarmonicFunction::Tonic,
        ChordSymbol::DFlatMajor -> match after {
            Some(ChordSymbol::GSeven) -> HarmonicFunction::Predominant,
            Some(next) -> HarmonicFunction::Other,
            None -> HarmonicFunction::Other,
        },
        ChordSymbol::FMajor -> HarmonicFunction::Predominant,
        ChordSymbol::GSeven -> HarmonicFunction::Dominant,
        ChordSymbol::AMinor -> HarmonicFunction::Tonic,
        ChordSymbol::BDiminished -> HarmonicFunction::Dominant,
    }
}

fn evidence(function: HarmonicFunction) -> Text {
    match function {
        HarmonicFunction::Tonic -> "stable goal under this analysis",
        HarmonicFunction::Predominant -> "moves toward the dominant under this analysis",
        HarmonicFunction::Dominant -> "moves toward the tonic under this analysis",
        HarmonicFunction::Other -> "this method assigns no function",
    }
}

fn analyze(key: Key, chord: Chord, after: Option<ChordSymbol>) -> Result<FunctionClaim, TonalError> {
    match chord {
        Chord::AnonymousChord(anonymous) -> Err(TonalError::CannotAnalyzeAnonymousChord),
        Chord::NamedChord(symbol, named) -> Ok(FunctionClaim {
            key = key,
            symbol = symbol,
            degree = degree(key, symbol),
            function = function_of(symbol, after),
            evidence = evidence(function_of(symbol, after)),
        }),
    }
}

fn write(voicing: Voicing, held: WrittenDuration) -> EventTrack[WrittenTime, ScoreFact] {
    match voicing {
        Voicing::VoicingValue(pitches) -> staff::chord(pitches, held),
    }
}
```

Four things this program forces, and the fourth is the one that matters.

**Nested patterns are load-bearing.** `Some(ChordSymbol::GSeven)` is a constructor inside a constructor, and note 28's
version wrote the same thing under a language whose §6.2 fixed patterns at depth one. `02-core-calculus.md` §6.2's
case-tree replacement is exercised by the first corpus program, on a program written before the change.

**`[bass, ..upper]` and `++` are not the same feature.** The pattern is `01-surface.md` §1's list pattern and survives;
note 28's `++` does not exist and this trial refuses to invent it — the append is written as `fold_from_end` with
`push`, which is `Buildable`'s. Prompt 141 will give it a name; nothing here promises one.

**`members` becomes a function.** Note 28 used a `let` inside `voice`'s body to bind it, and there is no `let` in a
block, so a two-arm helper takes its place. This is the third independent site where the missing block-local binding
shows, after §1.1 and §4, and unlike those two it is *cheap* — a named function is arguably better than a local. Its
cost is only that `evidence(function_of(symbol, after))` computes `function_of` twice in `analyze`. That is a real
duplication with no spelling to avoid it, and it is the honest price of "a block holds exactly one expression".

**Hidden constructors have no spelling.** Note 28 wrote `private NamedChord(…)` so that `Chord` could be built only by
`build`. There is no `private` and no `pub` in `01-surface.md` §1's grammar, no visibility marker anywhere in the
language today, and `signature`/`structure` hides only `let` and `fn` members. So
`Chord::NamedChord(ChordSymbol::GSeven, [Spelling::C])` — a chord whose symbol lies about its tones — is constructible
by any client. The program still compiles; the invariant does not survive. §14.

The client is unchanged except for spelling:

```musa
// pieces/neapolitan.musa
import trial::tonal as tonal;
import std::notation::staff as staff;

let neapolitan = tonal::build(ChordSymbol::DFlatMajor);
let dominant = tonal::build(ChordSymbol::GSeven);
let tonic = tonal::build(ChordSymbol::CMinor);

let claim: Result<FunctionClaim, TonalError> =
    tonal::analyze(Key::CMinorKey, neapolitan, Some(ChordSymbol::GSeven));

let page: EventTrack[WrittenTime, ScoreFact] = staff::sequence([
    tonal::write(tonal::voice(VoicingPolicy::OpenBass, neapolitan), staff::half),
    tonal::write(tonal::voice(VoicingPolicy::Close, dominant), staff::half),
    tonal::write(tonal::voice(VoicingPolicy::OpenBass, tonic), staff::whole),
]);
```

`Music` is gone (prompt 127a), so `page` is an `EventTrack[WrittenTime, ScoreFact]` and `staff.realize` with it. Bare
constructors are accepted in checking position — `Some(ChordSymbol::GSeven)` needs the qualification because
`ChordSymbol` is not determined there, and `staff::half` is a value rather than a constructor.

### 5.2 Flexible time

The two adapter regions are unchanged notation, and their expansions are unchanged values. What changes is the surface
around them:

```musa
// pieces/flexible-time.musa
import syntax std::notation::staff as staff;
import std::notation::staff as staff_value;
import std::performance::time as timing;

let melody = staff {
    instrument flute
    clef treble
    meter 4/4

    | c5/8 d5/8 e5/8 f5/8 g5/8 a5/8 b5/8 c6/8
    named held { | g5/4 rest/4 g5/2 }
    named feathered { feather accelerate { a5 b5 c6 d6 } }
    named cadenza { unmeasured { e6 d6 b5 c6 } }
};

let ostinato = staff { instrument hand_drum meter 3/4 | c3/4 rest/4 g3/4 };

let page = staff_value::overlay([melody, ostinato]);

let performance = timing::plan([
    timing::swing(staff_value::region(melody, "bar-1"), 2/1),
    timing::open_fermata(staff_value::region(melody, "held")),
    timing::metric_modulation(staff_value::eighth, staff_value::quarter),
    timing::linear_acceleration(
        staff_value::region(melody, "feathered"),
        timing::beats_per_minute(staff_value::quarter, 72),
        timing::beats_per_minute(staff_value::quarter, 144),
    ),
    timing::performer_timed(staff_value::region(melody, "cadenza")),
]);

let logical_choices = timing::logical_choices([
    timing::named_durations("feathered", [1/8, 1/12, 1/16, 1/32]),
    timing::named_durations("cadenza", [1/4, 1/8, 1/8, 1/2]),
]);

let score = timing::realize_staff(page, logical_choices);
```

This program exercises nothing the specification added and that is its value: **the composer-facing surface did not
move.** Records, enums, traits, quotation, and the dependent core are all invisible here. The only differences are the
brace-delimited region (which prompt 113 already settled) and `::` for module paths.

The one thing worth recording is that the expansion this region produces — `make_document(InstrumentName("flute"), …,
[Bar(f0, 1, [Note(f1, C(5), NoteValue(8, 0), [], NoTie), …]), …])` — is what the staff adapter of §§1–4 builds, and
every one of those `f0`…`f25` anchors comes from `syntax_anchor` rather than from a role integer. The value is
byte-identical; the program that produces it lost 119 lines of helper.

### 5.3 Phrase-led intent

```musa
// trial/phrase_led.musa
import std::list as list;
import std::notation::staff as staff;

enum Svara { Sa, Ri, Ga, Ma, Pa, Da, Ni }
enum Direction { Rising, Falling, Turning }
enum GestureKind { Plain, Oscillate, SlideFromBelow }
enum PhraseError { EmptyPhrase, NonPositiveWeight }

record PhraseToken { svara: Svara; direction: Direction; gesture: GestureKind; weight: Ratio; }
record PhraseContext { tonic_hz: Ratio; register_ratio: Ratio; oscillation_width: Ratio; }

enum TranscriptionLoss {
    CurveReducedToMark(Svara),
    DirectionKeptAsText(Direction),
    TonicReducedToStaffSpelling(Ratio),
}

enum PhraseGesture {
    Hold(Svara, Ratio, Ratio),
    OscillateAround(Svara, Ratio, Ratio, Ratio),
    ApproachFromBelow(Svara, Ratio, Ratio),
}

enum Phrase { PhraseValue(List<PhraseToken>) }

fn positive(token: PhraseToken) -> Bool { 0/1 < token.weight }

fn make(tokens: List<PhraseToken>) -> Result<Phrase, PhraseError> {
    match tokens {
        [] -> Err(PhraseError::EmptyPhrase),
        [head, ..tail] -> if tokens.fold_from_start(
            true,
            fn (state: Bool, token: PhraseToken) -> Bool { if state { positive(token) } else { false } },
        ) { Ok(Phrase::PhraseValue(tokens)) } else { Err(PhraseError::NonPositiveWeight) },
    }
}

fn svara_ratio(svara: Svara) -> Ratio {
    match svara {
        Svara::Sa -> 1/1, Svara::Ri -> 9/8, Svara::Ga -> 6/5, Svara::Ma -> 4/3,
        Svara::Pa -> 3/2, Svara::Da -> 8/5, Svara::Ni -> 9/5,
    }
}

fn center_hz(context: PhraseContext, svara: Svara) -> Ratio {
    context.tonic_hz * context.register_ratio * svara_ratio(svara)
}

fn gesture_for(context: PhraseContext, token: PhraseToken) -> PhraseGesture {
    match token.gesture {
        GestureKind::Plain -> PhraseGesture::Hold(token.svara, center_hz(context, token.svara), token.weight),
        GestureKind::Oscillate -> PhraseGesture::OscillateAround(
            token.svara, center_hz(context, token.svara), token.weight, context.oscillation_width,
        ),
        GestureKind::SlideFromBelow -> PhraseGesture::ApproachFromBelow(
            token.svara, center_hz(context, token.svara), token.weight,
        ),
    }
}

fn perform(phrase: Phrase, context: PhraseContext) -> List<PhraseGesture> {
    match phrase {
        Phrase::PhraseValue(tokens) ->
            tokens.map(fn (token: PhraseToken) -> PhraseGesture { gesture_for(context, token) }),
    }
}

fn staff_pitch(svara: Svara) -> staff::WrittenPitch {
    match svara {
        Svara::Sa -> staff::pitch("c", "natural", 4),
        Svara::Ri -> staff::pitch("d", "natural", 4),
        Svara::Ga -> staff::pitch("e", "flat", 4),
        Svara::Ma -> staff::pitch("f", "natural", 4),
        Svara::Pa -> staff::pitch("g", "natural", 4),
        Svara::Da -> staff::pitch("a", "flat", 4),
        Svara::Ni -> staff::pitch("b", "flat", 4),
    }
}

fn marks_for(token: PhraseToken) -> List<staff::Mark> {
    match token.gesture {
        GestureKind::Plain -> [],
        GestureKind::Oscillate -> [staff::text_mark("oscillating gesture")],
        GestureKind::SlideFromBelow -> [staff::text_mark("approach from below")],
    }
}

fn note_for(token: PhraseToken) -> EventTrack[WrittenTime, ScoreFact] {
    staff::note_with_marks(staff_pitch(token.svara), staff::exact_duration(token.weight), marks_for(token))
}

fn losses_for(token: PhraseToken) -> List<TranscriptionLoss> {
    match token.gesture {
        GestureKind::Plain -> [],
        GestureKind::Oscillate -> [TranscriptionLoss::CurveReducedToMark(token.svara)],
        GestureKind::SlideFromBelow -> [
            TranscriptionLoss::CurveReducedToMark(token.svara),
            TranscriptionLoss::DirectionKeptAsText(token.direction),
        ],
    }
}

fn transcribe(
    phrase: Phrase,
    context: PhraseContext,
) -> (EventTrack[WrittenTime, ScoreFact], List<TranscriptionLoss>) {
    match phrase {
        Phrase::PhraseValue(tokens) -> (
            staff::sequence(tokens.map(fn (token: PhraseToken) -> EventTrack[WrittenTime, ScoreFact] {
                note_for(token)
            })),
            tokens
                .fold_from_start([], fn (losses: List<TranscriptionLoss>, token: PhraseToken) -> List<TranscriptionLoss> {
                    losses_for(token).fold_from_start(losses, fn (into, one) { into.push(one) })
                })
                .push(TranscriptionLoss::TonicReducedToStaffSpelling(context.tonic_hz)),
        ),
    }
}
```

Three findings, all of them about operators.

**`ratio_greater` has no operator, and the answer is to reverse `<`.** `10-traits.md` §5's table has `Ord<A>.less` and
nothing else, so `ratio_greater(token.weight, 0/1)` is `0/1 < token.weight`. That is correct and it reads worse than
what it replaces. It is not worth an amendment: adding `>` as a second spelling of one method is the kind of surface
growth `10-traits.md` §9 exists to refuse, and a program that wants the other reading writes the arguments the other
way.

**`&&` has no spelling.** Note 28 wrote `state && positive(token)`. There is no boolean operator in the grammar and no
`bool_and` builtin, so the fold's step is `if state { positive(token) } else { false }`. In a strict total language that
*is* what `&&` means, and the existing `trivial` in `staff.musa` already writes its disjunction as an `if` ladder, so
this is not a regression. It is recorded because it will be written a great many times.

**`ratio_multiply` chains into `*` and reads much better.** `context.tonic_hz * context.register_ratio *
svara_ratio(svara)` replaces two nested calls and a `let`, which is the third place the missing block-local binding is
paid for and the first place an operator pays it back.

The client is note 28's with record literals in the new spelling and `first`/`second` replaced by a product pattern:

```musa
// pieces/phrase-pressure-test.musa
import trial::phrase_led as phrase;
import std::result as result;

let context = PhraseContext { tonic_hz = 264/1, register_ratio = 2/1, oscillation_width = 16/15 };

let checked_phrase = phrase::make([
    PhraseToken { svara = Svara::Ga, direction = Direction::Falling, gesture = GestureKind::Oscillate, weight = 1/4 },
    PhraseToken { svara = Svara::Ri, direction = Direction::Falling, gesture = GestureKind::SlideFromBelow, weight = 1/4 },
    PhraseToken { svara = Svara::Sa, direction = Direction::Falling, gesture = GestureKind::Plain, weight = 1/2 },
]);

let gesture_route = result::map(checked_phrase, fn (value: Phrase) -> List<PhraseGesture> {
    phrase::perform(value, context)
});

let notation_route = result::map(
    checked_phrase,
    fn (value: Phrase) -> (EventTrack[WrittenTime, ScoreFact], List<TranscriptionLoss>) {
        match phrase::transcribe(value, context) { (page, losses) -> (page, losses) }
    },
);
```

### 5.4 Ensemble-led tuning

```musa
// trial/ensemble_tuning.musa
import std::notation::staff as staff;

enum Degree { Center, UpperThird, UpperFifth }
enum Register { Low, Middle, High }
enum Pairing { Single, Paired }
enum TuningError { NonPositiveReference, NonPositivePairRate }

record TuneRequest { degree: Degree; register: Register; pairing: Pairing; }
record AcousticTarget { lower_hz: Ratio; upper_hz: Option<Ratio>; target_beats_per_second: Ratio; }

enum TuningLoss {
    ExactFrequencyNotShown(Ratio),
    PairingNotShown,
    BeatingTargetNotShown(Ratio),
}

enum Ensemble { EnsembleValue(Ratio, Ratio, Ratio, Ratio) }

fn make(reference_hz: Ratio, low: Ratio, middle: Ratio, high: Ratio) -> Result<Ensemble, TuningError> {
    if 0/1 < reference_hz {
        if if 0/1 < low { if 0/1 < middle { 0/1 < high } else { false } } else { false } {
            Ok(Ensemble::EnsembleValue(reference_hz, low, middle, high))
        } else { Err(TuningError::NonPositivePairRate) }
    } else { Err(TuningError::NonPositiveReference) }
}

fn degree_ratio(degree: Degree) -> Ratio {
    match degree { Degree::Center -> 1/1, Degree::UpperThird -> 5/4, Degree::UpperFifth -> 3/2 }
}

fn register_ratio(register: Register) -> Ratio {
    match register { Register::Low -> 1/2, Register::Middle -> 1/1, Register::High -> 2/1 }
}

fn beat_target(register: Register, low: Ratio, middle: Ratio, high: Ratio) -> Ratio {
    match register { Register::Low -> low, Register::Middle -> middle, Register::High -> high }
}

fn realize(ensemble: Ensemble, request: TuneRequest) -> AcousticTarget {
    match ensemble {
        Ensemble::EnsembleValue(reference_hz, low, middle, high) -> match request.pairing {
            Pairing::Single -> AcousticTarget {
                lower_hz = reference_hz * degree_ratio(request.degree) * register_ratio(request.register),
                upper_hz = None,
                target_beats_per_second = 0/1,
            },
            Pairing::Paired -> AcousticTarget {
                lower_hz = reference_hz * degree_ratio(request.degree) * register_ratio(request.register),
                upper_hz = Some(
                    reference_hz * degree_ratio(request.degree) * register_ratio(request.register)
                        + beat_target(request.register, low, middle, high),
                ),
                target_beats_per_second = beat_target(request.register, low, middle, high),
            },
        },
    }
}

fn staff_degree(degree: Degree, register: Nat) -> staff::WrittenPitch {
    match degree {
        Degree::Center -> staff::pitch("c", "natural", register),
        Degree::UpperThird -> staff::pitch("e", "natural", register),
        Degree::UpperFifth -> staff::pitch("g", "natural", register),
    }
}

fn staff_register(register: Register) -> Nat {
    match register { Register::Low -> 3, Register::Middle -> 4, Register::High -> 5 }
}

fn frequency_losses(target: AcousticTarget) -> List<TuningLoss> {
    match target.upper_hz {
        None -> [TuningLoss::ExactFrequencyNotShown(target.lower_hz)],
        Some(upper) -> [
            TuningLoss::ExactFrequencyNotShown(target.lower_hz),
            TuningLoss::ExactFrequencyNotShown(upper),
            TuningLoss::PairingNotShown,
            TuningLoss::BeatingTargetNotShown(target.target_beats_per_second),
        ],
    }
}

fn transcribe(
    request: TuneRequest,
    target: AcousticTarget,
) -> (EventTrack[WrittenTime, ScoreFact], List<TuningLoss>) {
    (
        staff::note_with_marks(
            staff_degree(request.degree, staff_register(request.register)),
            staff::note_value(2, 0),
            [staff::text_mark("ensemble-specific tuning")],
        ),
        frequency_losses(target),
    )
}
```

**This is the program where the missing block-local binding hurts most, and it is the one place the trial had to make an
aesthetic judgment.** Note 28's `realize` binds `degree_hz`, then `lower`, then `target_rate`, and reads cleanly.
Without `let`, `reference_hz * degree_ratio(request.degree) * register_ratio(request.register)` is written three times
in one record literal, and `beat_target(request.register, low, middle, high)` twice.

The honest alternative is to split `realize` into two helpers taking the computed values as parameters, which is what
§5.1 did for `members` and what the trial recommends for prompt 142's migration. The version above is written the
duplicating way *on purpose*, because it is what an author reaches for first and prompt 144's diagnostics work should
know that. **Repeated subexpressions are the standing cost of a block that holds one expression**, and no mechanism in
the specification addresses it. It is not a falsifier — every program is writable, and the total language has no effects
so the duplication is only work, charged twice to the §4 meter — but it is the trial's clearest ergonomic finding and
§14 records it as a candidate for evidence rather than for an amendment now.

`&&` appears again and nests three deep, which is the ugliest line in the trial.

### 5.5 Interactive performance

```musa
// trial/live_dialogue.musa
import std::notation::staff as staff;
import std::performance::gesture as gesture;

enum Cue { Begin, Continue, Cut }
enum Movement { LeaderCue(Cue), Motion(Text), Stillness }
enum Phase { Waiting, Active(Nat), Ended }
enum Response { Listen, DrumAnswer(Text), Stop }
enum ProtocolError { MotionBeforeBegin, InputAfterEnd }
enum ProtocolLoss { ActualMovementMissing, ActualTimingMissing, PerformerJudgmentMissing }

fn step(state: Phase, movement: Movement) -> Result<(Phase, Response), ProtocolError> {
    match (state, movement) {
        (Phase::Waiting, Movement::LeaderCue(Cue::Begin)) -> Ok((Phase::Active(0), Response::Listen)),
        (Phase::Waiting, Movement::LeaderCue(Cue::Continue)) -> Ok((Phase::Waiting, Response::Listen)),
        (Phase::Waiting, Movement::LeaderCue(Cue::Cut)) -> Ok((Phase::Ended, Response::Stop)),
        (Phase::Waiting, Movement::Motion(name)) -> Err(ProtocolError::MotionBeforeBegin),
        (Phase::Waiting, Movement::Stillness) -> Ok((Phase::Waiting, Response::Listen)),
        (Phase::Active(turn), Movement::LeaderCue(Cue::Begin)) -> Ok((Phase::Active(turn), Response::Listen)),
        (Phase::Active(turn), Movement::LeaderCue(Cue::Continue)) ->
            Ok((Phase::Active(turn + 1), Response::Listen)),
        (Phase::Active(turn), Movement::LeaderCue(Cue::Cut)) -> Ok((Phase::Ended, Response::Stop)),
        (Phase::Active(turn), Movement::Motion(name)) ->
            Ok((Phase::Active(turn), Response::DrumAnswer(name))),
        (Phase::Active(turn), Movement::Stillness) -> Ok((Phase::Active(turn), Response::Listen)),
        (Phase::Ended, anything) -> Err(ProtocolError::InputAfterEnd),
    }
}

fn sounded(response: Response) -> GestureIntent {
    match response {
        Response::Listen -> gesture::silence(1/16),
        Response::DrumAnswer(name) -> gesture::named_drum(name),
        Response::Stop -> gesture::release_all(),
    }
}

fn cue_sheet() -> (EventTrack[WrittenTime, ScoreFact], List<ProtocolLoss>) {
    (
        staff::sequence([
            staff::text_cue("Begin: enter the dialogue"),
            staff::text_cue("Motion: lead drummer answers the observed movement"),
            staff::text_cue("Continue: keep the dialogue open"),
            staff::text_cue("Cut: stop together"),
        ]),
        [
            ProtocolLoss::ActualMovementMissing,
            ProtocolLoss::ActualTimingMissing,
            ProtocolLoss::PerformerJudgmentMissing,
        ],
    )
}
```

**This is the strongest single case for the case-tree replacement in `02-core-calculus.md` §6.2.** Note 28 wrote `step`
as a `match` on `state` containing a `match` on `movement` — five nested arms, eleven lines of scaffolding. Written as
one `match` on the product with nested constructor patterns three deep
(`(Phase::Waiting, Movement::LeaderCue(Cue::Begin))`), it is eleven arms and no nesting, and every arm is on one line.
The depth-one rule the section replaced would have rejected every one of them.

Two renames the trial had to make and records as findings about naming rather than about the language: note 28's type
`State` collides with `staff.musa`'s reader `State`, and its function `gesture` shadows the module it imports. Under
namespaced constructors the *constructor* collision would not arise (`Phase::Active` and `State::Opened` cannot
collide), but a type name and a module alias still can. `01-surface.md` §1's flat value namespace with a conflict-naming
error is what catches it, and it catches it here.

The studio block in note 28 §6.2 is program ten's subject and is not repeated.

### 5.6 What the corpus forced

| Feature | Note 28's decision | This trial |
| --- | --- | --- |
| records and nominal variants | keep | kept; `record` and `enum` are now two words (§5.1, §5.3) |
| hidden constructors | keep | **no spelling exists** (§5.1, §14) |
| exhaustive matches | keep | kept, and now over namespaced constructors |
| nested patterns | not admitted | **required by two of five programs** (§5.1, §5.5) |
| rank-1 inference | keep | replaced by bidirectional elaboration; every program writes its signatures and nothing else changed |
| `Result`, `Text`, exact ratios | keep | kept; `Sub`, `Div` keep the failing shape (§5.4) |
| dependent types | reject | **still unused by all five** — §7 says which mechanisms the adapters exercise instead |
| type-directed macros | reject | still rejected; quotation is not one (`11-quotation.md` §2) |
| `++`, `&&`, `>` | used freely | **none has a spelling**; each is written out (§5.1, §5.3, §5.4) |

The last row is the corpus's real message. Note 28 was written in a sketch surface, and three of its conveniences never
became language. Two of them (`++`, `>`) are correctly refused. The third (`&&`) is written out in every program that
has a conjunction, and every one of them is a fold or a guard.

## 6. Program ten — the studio adapter's `validate`

`stdlib/src/adapters/studio.musa` does not exist; prompt 127dcg's design and note 27 §3 are what this rewrites, and
prompt 146 is what will build it. `validate` is the half of the studio that is *not* an adapter — an ordinary total
package function over the finished description — which is what makes it the trial's only test of the new language away
from syntax entirely.

```musa
// stdlib/src/studio/graph.musa
enum PortKind { Audio(Nat), Control, NoteEvents }

record PortPath { node: Text; port: Text; }
record Parameter { anchor: Syntax<Expr>; name: Text; value: ParameterValue; }

enum ParameterValue { Count(Nat), Plain(Ratio), Seconds(Ratio) }

enum StudioDecl {
    Input { anchor: Syntax<Expr>; name: Text; kind: PortKind; },
    Node { anchor: Syntax<Expr>; name: Text; descriptor: Text; parameters: List<Parameter>; },
    Output { anchor: Syntax<Expr>; name: Text; kind: PortKind; },
    Connect { anchor: Syntax<Expr>; from: PortPath; onto: PortPath; },
    Bind { anchor: Syntax<Expr>; part: Text; node: Text; },
}

record StudioDescription { declarations: List<StudioDecl>; }
record CheckedStudio { description: StudioDescription; }

enum StudioError {
    UnknownDescriptor { anchor: Syntax<Expr>; descriptor: Text; },
    ParameterOutOfRange { anchor: Syntax<Expr>; name: Text; },
    UnknownPort { anchor: Syntax<Expr>; path: PortPath; },
    PortKindMismatch { anchor: Syntax<Expr>; source: PortKind; target: PortKind; },
    UnboundPart { anchor: Syntax<Expr>; part: Text; },
    InstantaneousCycle { anchors: List<Syntax<Expr>>; },
}

impl Eq<PortKind> {
    fn equal(x: PortKind, y: PortKind) -> Bool {
        match (x, y) {
            (PortKind::Audio(m), PortKind::Audio(n)) -> m == n,
            (PortKind::Control, PortKind::Control) -> true,
            (PortKind::NoteEvents, PortKind::NoteEvents) -> true,
            _ -> false,
        }
    }
}

fn port_kind(description: StudioDescription, path: PortPath) -> Option<PortKind> {
    description.declarations.fold_from_start(None, fn (found: Option<PortKind>, one: StudioDecl) -> Option<PortKind> {
        match found {
            Some(kind) -> Some(kind),
            None -> match one {
                StudioDecl::Input { anchor, name, kind } -> if name == path.node { Some(kind) } else { None },
                StudioDecl::Output { anchor, name, kind } -> if name == path.node { Some(kind) } else { None },
                StudioDecl::Node { anchor, name, descriptor, parameters } ->
                    if name == path.node { descriptor_port(descriptor, path.port) } else { None },
                StudioDecl::Connect { anchor, from, onto } -> None,
                StudioDecl::Bind { anchor, part, node } -> None,
            },
        }
    })
}

// A check answers what is wrong, or nothing. There is no unit value in
// expression position, so `Option<StudioError>` is what a check that produces
// no value returns.
fn connection_checked(description: StudioDescription, one: StudioDecl) -> Option<StudioError> {
    match one {
        StudioDecl::Connect { anchor, from, onto } -> match port_kind(description, from) {
            None -> Some(StudioError::UnknownPort { anchor = anchor, path = from }),
            Some(source) -> match port_kind(description, onto) {
                None -> Some(StudioError::UnknownPort { anchor = anchor, path = onto }),
                Some(target) -> if source == target {
                    None
                } else {
                    Some(StudioError::PortKindMismatch { anchor = anchor, source = source, target = target })
                },
            },
        },
        _ -> None,
    }
}

fn validate(description: StudioDescription) -> Result<CheckedStudio, StudioError> {
    match description.declarations.fold_from_start(
        None,
        fn (sofar: Option<StudioError>, one: StudioDecl) -> Option<StudioError> {
            match sofar {
                Some(why) -> Some(why),
                None -> declaration_checked(description, one),
            }
        },
    ) {
        Some(why) -> Err(why),
        None -> Ok(CheckedStudio { description = description }),
    }
}
```

Five findings, and the second is the trial's clearest argument for the whole design.

**Named-field enum cases carry the whole error type.** `StudioError` has six cases with two to four fields each; note 27
§3.2 wrote `StudioDecl` positionally, and `Connect(s15, PortPath("expression_depth", "control"), PortPath("room",
"audio"))` says nothing about which path is the source. With named fields the diagnostic-producing code reads
`anchor = anchor, source = source, target = target` and the mistake of swapping two `PortKind` arguments becomes
unspellable. This is the strongest case in the trial for `01-surface.md` §1.3's named-field form, and §13 R3 is what
makes it writable.

**`Eq<PortKind>` is the first hand-written instance in the trial, and it is the one the design is for.** Note 27's
`validate` "checks exact port-kind equality"; today that is a hand-written six-arm comparison in every package that
needs it. One `impl Eq<PortKind>` and `source == target` is the operator table working exactly as `10-traits.md` §5
describes: a known concrete head, no search, no defaulting.

**Coherence has nothing to do here, and that is the finding.** Every constraint in ten programs resolves at a known
concrete head — `Text`, `Ratio`, `Nat`, `PortKind`, `List<A>` under a `where`. Not one is postponed on a metavariable,
not one needs the local-beats-global rule, and not one instance has a `where` clause of its own, so §4's Paterson
measure is never consulted. The trait system is exercised only in its simplest mode. That is not an argument to remove
it — §5's operator table is what makes `==` mean anything — but it means **`10-traits.md` §2's coherence argument and
§4's termination measure are unexercised by any program written so far**, and §7 records what still owes them evidence.

**Folding to the first failure is verbose and there is no `traverse`.** `validate` writes it out, because `?` cannot
cross a closure boundary (`01-surface.md` §1: the answer a `?` leaves is the enclosing function's, and a fold step is a
different function). Note 28 §1 listed `traverse_result` among the shared standard operations; prompt 141 owns whether
it comes back. Recorded, not repaired.

**There is no unit value in expression position, and the studio is where it first matters.** `01-surface.md` §1 refuses
one deliberately — it is the reason `else` is mandatory — so a check that produces nothing on success answers
`Option<StudioError>` rather than `Result<Unit, StudioError>`. That reads *better*, not worse: `None` means "nothing
wrong" rather than "the successful unit", and combining checks is the ordinary first-`Some` fold. It is recorded because
the `Result`-shaped instinct is what an author arriving from Rust reaches for, and the language will refuse it at the
`Ok(unit)` that instinct writes.

## 7. Every mechanism against a program

The rule prompt 132 sets is that a mechanism no program exercises is a mechanism to delete. This is that table.

| Mechanism | Exercised by | Executable evidence owed |
| --- | --- | --- |
| predicative universes | nothing in the trial writes a universe; every program is at `Type 0` | 133, 148 |
| dependent function type Π | `Iterable::map<D, B>`'s `where Buildable<D, B>` is the closest, and it is not dependent | 133, 148 |
| dependent records | trait dictionaries only (`Eq<PortKind>`, §6); no program writes a dependent field | 133, 137 |
| **records with η** | `10-traits.md` §2's coherence argument; **no program exercises it directly** | 133, 148 |
| inductive families with parameters | every `enum` in §5, `Option`, `Result`, `List<A>` | 135 |
| **inductive families with indices** | `Syntax<Cat>` only, and only at closed index constructors (§1.2, §10) | 135, 138 |
| dependent match with coverage | `word_read` over `StaffWord` (§2.1), `step` over the product (§5.5) — both non-dependent | 135 |
| **nested patterns / case trees** | `(Phase::Waiting, Movement::LeaderCue(Cue::Begin))` (§5.5), `Some(ChordSymbol::GSeven)` (§5.1) | 135 |
| **`Id` / `refl` / `J`** | **nothing** | — |
| **K** | **nothing** (§10) | — |
| NbE conversion | every program, invisibly | 133, 148 |
| metavariables and pattern unification | every unannotated lambda parameter in every fold step | 134, 148 |
| **postponement on a blocked constraint** | **nothing** — every head is concrete (§6) | 134 |
| well-founded termination | `staff_word`, `validate`'s fold, `voiced_inside` — all structural | 135 |
| **a non-structural measure** | **nothing**; every recursion in ten programs is structural | 135 |
| `Storable` | `SyntaxStep<State, State>` must not be, and is not held anywhere | 137, 148 |
| **records** | `Pending`, `Reading`, `Head`, `Said`, `FunctionClaim`, `PhraseToken`, `PhraseContext`, `TuneRequest`, `AcousticTarget`, `PortPath`, `Parameter`, `StudioDescription` | 136 |
| **path update** | `after_item`, `faulting`, `refusing` (§3) | 136 |
| **namespaced enum constructors** | every program in §5; the `Untied` collision (§5.5) | 136 |
| **named-field enum cases** | `Refusal`, `Tying`, `StudioDecl`, `StudioError` (§3, §6) | 136 |
| `enum Empty {}` | nothing, except `Decision<P>`'s `No(P -> Empty)`, which nothing constructs | 136 |
| **traits and operators** | `==` at `Text`/`Nat`/`PortKind`, `*` and `/` and `+` at `Ratio`, `<` at `Ratio` | 137 |
| **coherence and the orphan rule** | **nothing** — one instance per head, all local (§6) | 137, 148 |
| **the Paterson instance measure** | **nothing** — no instance in ten programs has a `where` clause | 137 |
| derived methods | `map`, `filter`, `collect` on `List` (§2.3, §5.1, §5.3) | 137, 141 |
| inherent methods | `Duration::of` in the printer; `xs.push` is `Buildable`'s, not inherent | 137 |
| **`DecEq`** | **nothing** in a program; §10 uses it as the argument against K | 137 |
| `Syntax<Expr>` / `Syntax<TokenTree>` | every emitting site (§1), every splice (§1.2, §2.2) | 138 |
| **`Syntax<Item>` / `Syntax<Pattern>`** | **nothing** (§9) | — |
| **`quote at p { … }`** | twenty call sites (§1.2), `document_read` (§4) | 139 |
| **`$x` value splice** | every quote in §1 and §4 | 139 |
| **`${ e }` expression splice** | `dot_count`, `anchored`, `as_expression`, six `stated_field`s (§1.1, §1.2, §4) | 139 |
| **`$..xs` sequence splice** | the studio's `make_description([$..decls])` (§6); **no staff site** | 139 |
| derived provenance | all twenty-seven role integers (§1.1) | 139, 148 |
| **quote patterns** | **nothing in either adapter** (§2.2) | — |
| `Buildable` | `voiced_inside` (§2.3), `voice`'s `OpenBass` arm (§5.1), `transcribe` (§5.3) | 141 |
| **`Vec A n`** | **nothing** | — |

Six rows read "nothing", and §9 says what happens to each.

## 8. The falsifiers, answered

Each answer names the program that decided it. "Probably not" is not an answer a later prompt can act on, so each is a
yes or a no.

**Rank-2 or higher polymorphism, or a higher-kinded type variable the specification does not give — no.** The closest
call is `Iterable<C, A>`'s derived `map<D, B>(source: C, f: A -> B) -> D where Buildable<D, B>`, where `C` and `D` are
*types*, not type constructors: `List<Pitch>` and `List<Text>` are two `C`s rather than one `C` at two arguments. The
recursor's four branches are rank-1, as note 40 §2 established and §2.2 above re-confirms; `recurse_syntax` takes
functions and returns a value, and nothing takes a polymorphic function as an argument. Decided by §2.3 and §5.1.

**A `partial` definition, general recursion, or a termination measure the checker cannot see — no.** Every recursion in
ten programs is structural: `run_syntax_step` on a sealed child, `fold_from_start`/`fold_from_end` over a list,
`nat_fold` over a count, and generated recursors over declared families. Not one program presents a measure at all,
which is the strongest possible answer and also means `02-core-calculus.md` §2.4's *general* measure is unexercised
(§7). Decided by §2.3, §5.3, and §6.

**Search during trait resolution — an overlapping instance, a default, or an order-dependent resolution — no, and by a
wide margin.** Every constraint in ten programs resolves at a known concrete head. No instance is written twice for one
head, no `where` clause appears on any instance, no constraint is postponed, and no `collect` is written in an inferring
position. Decided by §6, which is the only program with a hand-written instance at all.

**A hand-written provenance path, or a role integer by another name — no.** The rewritten adapter contains zero
`syntax_built` calls and zero integer literals in a role position; `syntax_anchor` answers a node rather than a number
and the adapter can read nothing from it. What replaces the twenty-seven values is
`Derived { origin, quotation, path }`, computed. Decided by §1.1 and §4.

**An escape from `Syntax<Cat>`'s index — a cast between categories, or an untyped `Syntax` — yes, once, and it is a
checked parse rather than a cast.** `as_expression : Syntax<TokenTree> -> Option<Syntax<Expr>>` is required by every
site that splices a composer's token into an emitted expression, and it is not an escape in the sense the falsifier
means: it establishes the claim by parsing rather than asserting it, which is `11-quotation.md` §1's own rule for how a
refined claim is introduced. What it *does* falsify is §1's enumeration of two introduction forms. Decided by §1.2, and
§13 R6 is the repair.

**A string round-trip to say something the typed API should have said — yes, fifteen times, and no typed API could say
it.** The staff keywords (`bar`, `rest`, `slur`, …) are compared against `Text` because they are the notation's own
words and the region hands them over as an `Identifier` token's spelling. §2.1 centralizes them into one lookup and one
declared enum, which is the most a type system can do about a word a composer types. This is the one falsifier that
fires in the letter and not in the spirit, and it fires against the *notation*, not against the design: any adapter for
any notation with keywords will have exactly one such table.

## 9. What the trial deletes

| Deleted | Because | Which prompt deletes it |
| --- | --- | --- |
| `Cat::Item` and `Cat::Pattern` | nothing in ten programs constructs either, and each carries a third of prompt 138's index-soundness obligation | 138, repaired in `11-quotation.md` §1 here |
| `syntax_built`, `syntax_token`, `syntax_identifier`, `syntax_group` | quotation replaces all four; §1.1's emitting section names none of them | 139 |
| `syntax_binder`, `syntax_reference`, `syntax_binding` | zero uses in `staff.musa`, `doubled.musa`, or `mod.musa`, and hygiene inside a quote is `11-quotation.md` §2's rule rather than an operation an author calls | 139 |
| `checked_expression` as a whole-result gate only | generalized to `as_expression` at any node (§1.2); the duplicate-path check it also performs stays | 138 |
| `syntax_anchor`'s third argument | the `SourceInfo` it took is exactly what `11-quotation.md` §3 computes | 138 |
| the K axiom | §10 | 135, repaired in `02-core-calculus.md` §1.4 here |
| `option_fold` | already deleted by `01-surface.md` §1; §5 of the same document still calls it | repaired here, R1 |

**The phase registry goes from fourteen operations to seven**: `recurse_syntax`, `run_syntax_step`,
`syntax_fold_from_leaves`, `syntax_at`, `syntax_anchor`, `syntax_number`, `as_expression`. That is the trial's largest
structural result and it was not predicted by any of prompts 129–131, which spoke about role integers and helper
functions rather than about the registry.

Two rows deserve their re-opening condition stated, because a deletion without one is a decision nobody can revisit.
`Cat::Item` comes back when an adapter expands a region into declarations rather than into an expression — no planned
adapter does. `syntax_binder` and `syntax_reference` come back when an adapter introduces a name the composer can see
and refer to; today's staff and studio both emit closed expressions.

**Three mechanisms are unexercised and are *not* deleted**, and saying why is as important as the deletions:

- **`Id`, `refl`, and `J`.** Nothing in ten programs writes a propositional identity, but §10's argument that K is
  derivable *depends* on `Id` and `DecEq` existing, and `02-core-calculus.md` §5's consistency obligation is stated over
  them. Deleting the identity type would delete the vocabulary the metatheory is written in.
- **Quote patterns.** Neither adapter uses one, because staff notation is not Musa syntax (§2.2). But an adapter *over
  Musa syntax* — a template dialect, a lint, a structured edit — is exactly what the form is for, and prompt 147's
  freeze is where the absence of a user should be decided. Recorded as at risk rather than deleted.
- **The non-structural termination measure.** Every recursion in the trial is structural, so `02-core-calculus.md`
  §2.4's general form buys nothing here. It is kept because the *point* of §2.4 is that the checker has no hole in it,
  and a checker that only accepted structural decrease would be the old rule with a new name.

## 10. The K decision, settled

`02-core-calculus.md` §1.4 nominated this prompt: "The trial rewrites index-unifying programs for `Vec` and
`Syntax<Cat>`; if they go through without K, the K decision is dropped there rather than discovered during prompt 135."

**The trial cannot answer the question as posed, because no program unifies an index at all.**

- `Syntax<Cat>` appears in the two adapters only at *closed* index constructors — `Syntax<Expr>` and
  `Syntax<TokenTree>`. There is no `Syntax<c>` for a variable `c` anywhere, so a `match` on a syntax value never learns
  anything about its index and no unification problem arises.
- `Vec A n` appears in no program. The two fixed-arity things the staff adapter has — up to two numbers, up to two words
  — are `Numbers` and `Words`, enums whose cases are named, and both read better than `Vec Syntax<Expr> 2` would.
- The five corpus programs are entirely non-dependent, as note 28 §7 already recorded and §5.6 re-confirms.

So K is admitted for a use nothing exercises, and by the trial's own rule it goes. The stronger reason it can go without
loss is `10-traits.md` §7's, which was written as a remark and turns out to be the argument: **for every type these ten
programs declare, K is a theorem rather than an axiom.** Every declared type here is a finite inductive family over base
types with decidable equality, and Hedberg's theorem gives uniqueness of identity proofs from decidable equality. What
index unification in `match` would need — `DecEq Cat`, over two closed cases — is the easiest instance there is.

The recommendation, and `02-core-calculus.md` §1.4's repair in §13 R7:

- **K is not admitted as an axiom.** `Id`, `refl`, and `J` stay; uniqueness of identity proofs is derived through
  `DecEq` where a program needs it.
- **Prompt 135's coverage checker may not use a unification rule that requires K** until a program requires one. Since
  no program in the trial unifies an index, this costs nothing today and is checkable at the declaration where the rule
  would be written.
- **Re-opening is an ordinary amendment**: a program with an indexed family whose index type has no `DecEq`, and whose
  `match` needs the deletion rule. Prompt 141's `Vec A n` is the first candidate; `Nat` has `DecEq`, so it probably is
  not one.

What this buys is the door §1.4 said would close. Univalence, higher inductive types, and cubical parametricity are
inconsistent with K as an axiom and are merely absent without it. None is on any roadmap, which is why §1.4 was willing
to pay; the trial finds the payment was not necessary.

## 11. Staff/studio asymmetry

Note 40 §12 recorded an asymmetry about the *recursor*: studio threads an inherited context it never reads, so the
derived bottom-up fold is 13 lines against the recursor's 35 for the same answers. That finding is untouched — nothing
in prompts 129–131 changes which traversal a studio line wants, and `syntax_fold_from_leaves` stays public for it
(`11-quotation.md` §5).

The asymmetry the *new* mechanisms produce runs the other way and is smaller than expected:

|  | Staff | Studio |
| --- | --- | --- |
| quotation at construction | 20 sites, 119 lines of helper deleted | ~6 sites, no helper existed to delete |
| `$..xs` sequence splice | **never used** | `make_description([$..decls])` — the whole result |
| quote patterns | never used (notation is not Musa) | never used (declaration lines are not Musa either) |
| records | `Pending`, `Reading`, `Head`, `Said` — 14 destructures deleted | `PortPath`, `Parameter`, `StudioDescription` — small |
| named-field enum cases | `Refusal`, `Tying`, `Length`, `Numbers` — legibility | `StudioDecl`, `StudioError` — **the error type is unwritable positionally** (§6) |
| traits and operators | `==` replacing 21 `text_equal` sites, of which 13 become typed | one hand-written `impl Eq<PortKind>`, and it is the trial's only instance |
| `Buildable` | closes note 41 §4's bracket hole | not needed |
| inherited context | required (three places, §2.2) | never read (note 40 §12) |

Two rows are worth naming rather than averaging.

**The sequence splice is entirely studio's.** The staff adapter emits right-nested `Sounded(anchor, event, after)` and
never a list, so `$..xs` has no staff site at all; the studio emits one list and uses it once. A feature with one user
across two adapters is a feature to watch, and prompt 146 is where it either earns its keep or is recorded as
speculative.

**Named-field enum cases are studio's strongest need and staff's mildest.** Staff's enums are small and their positional
forms are readable; `StudioError`'s six cases with two to four fields each are the case where positional construction is
actively dangerous, because two `PortKind` arguments in a mismatch error can be swapped without a type error. Prompt 130
argued the feature from `Pending`; the better argument is a diagnostic type nobody has written yet.

## 12. The predicted size of the staff rewrite

Prompt 145 is gated on this number, so it is derived rather than asserted, and the derivation is shown so that prompt
145 can say which line of it was wrong.

The file is **2,404 lines / 93,252 bytes / 1,814 non-comment non-blank lines** at prompt 127dcfb. It divides into six
parts, and §§1–4 above measured four of them directly:

| Part | Lines | Measured or estimated | New | Δ |
| --- | ---: | --- | ---: | ---: |
| header, `syntax staff {`, and the reading data declarations (1–352) | 352 | estimated: `data` → `record`/`enum`, named fields | 366 | +14 |
| emitting, and its twenty call sites (353–588 plus the sites) | 268 | **measured, §1.4** | 104 | −164 |
| pieces and placing (590–1042) | 453 | estimated: §3's destructure deletion at 11 sites | 385 | −68 |
| the forms and the header (1044–1550) | 507 | estimated: `later.numbers` for destructures, quotes for `callN` | 431 | −76 |
| the reader and the document (1551–2166) | 616 | **measured, §2.4 and §4** | 565 | −51 |
| writing, the printer (2168–2404) | 236 | **measured, §1.3** | 222 | −14 |
| **total** | **2,404** |  | **2,073** | **−331** |

Two of the six rows are estimates and both are estimated the same way: the destructure count in that range times the
three lines a destructure costs, plus the `callN` sites in that range times the difference §1.4 measured. The estimate
is deliberately *conservative* — it credits nothing to legibility, nothing to the possibility that a rewrite finds a
better factoring, and it charges the record literals in full.

**The prediction for prompt 145: 2,050 ± 100 lines, and 79,000 ± 4,000 bytes.** Bytes fall further than lines because
the deleted material is unusually wide — `syntax_group(syntax_built(here, role, 0), "layout", [` is 48 columns before
its first argument — and the added material is unusually narrow.

**That is a 14% reduction, and it is not dramatic.** The prompt asked for a figure on "dramatically", so: the design
fails its own acceptance gate if prompt 145 does not reach **at least 25%** — 1,800 lines — and the trial predicts it
will not. Three reasons, all of them findings rather than excuses:

1. **The file is half printer and half reader, and quotation only touches the reader's output.** §1.3 measured the
   printer at −6%. No mechanism in prompts 129–131 is aimed at it.
2. **The dispatch table's third half does not shrink** (§2.1). Fifteen notation keywords are fifteen keywords.
3. **`Pending` survives, for the third time** (§3). What records delete is the fourteen destructures, which §3 measured
   at −68 lines — real, and one thirtieth of the file.

So the honest prediction is that **prompt 145 will not clear the bar prompt 132 was asked to set, and the bar is the
wrong measurement.** The line count was chosen in prompt 128's amendment as the evidence that the language was
underpowered, and it was the right evidence for *that* claim: 119 lines of `callN`, 27 unchecked integers, and 21 string
comparisons are what an underpowered language looks like. It is the wrong evidence for whether the replacement worked,
because most of the file was never about the language.

The trial therefore proposes a second gate for prompt 145, alongside the line count and not instead of it — every item
is a count the file can be grepped for, and every one goes to zero or to a named number:

| Gate | Now | Required at 145 |
| --- | ---: | ---: |
| `callN`-style construction helpers | 7 | 0 |
| hand-allocated role integers | 27 | 0 |
| `syntax_built` calls | 56 | 0 |
| eight-field destructures of `Pending` | 14 | 0 |
| positional `Reading(…)` rebuilds | 20 | 0 |
| `text_equal` on a token kind or delimiter | 13 | 0 |
| `text_equal` on a notation keyword | 8 | ≤ 2, inside lookup functions |
| nested group inside `[ … ]` dropped | yes | no |
| phase operations named by the file | 8 | ≤ 5 |
| lines | 2,404 | ≤ 2,150 |

Prompt 145's Design should be repaired to carry this table, and prompt 132's Stop forbids this trial from writing that
repair. §13 R8 records what it says.

## 13. Corrections to the specification

Each is a repair with the program that forced it. None is an amendment to
[`docs/rules/constitution.md`](../../../rules/constitution.md) or
[`docs/rules/obligations.md`](../../../rules/obligations.md), and none adds a capability the specification did not
already claim.

**R1 — `01-surface.md` §5 still calls `option_fold`.** §1 of the same document deleted it ("`option_fold` is gone:
`Option` is an ordinary enum") and `10-traits.md` §5's table says the replacement is `match`. Forced by §1.3, which
rewrites the two printer sites. Repaired by rewriting the `sounded` example as a `match`.

**R2 — the function type has no multi-parameter spelling.** `01-surface.md` §1.6 declares
`fn fold_from_start<B>(source: C, zero: B, step: (B, A) -> B) -> B`, and in §1's type grammar `(B, A)` is a *product*
and `->` is a one-argument arrow, so that signature says the step takes one pair. Every fold in Musa calls its step with
two arguments and a call must be complete (`02-core-calculus.md` §1.3), so the product reading is wrong. Note 40 §2
found the same gap for the recursor's four-argument branches and lived with unannotated lambdas; §1.6 cannot, because it
is a *declaration*. Repaired by adding a parenthesized parameter-list production to the type grammar, with `A -> B`
retained as the one-parameter shorthand and `((A, B)) -> C` as the way to write a function over a pair. Forced by
`Iterable`'s own declaration, and by every `fold_from_start` call in §§2, 5, and 6.

**R3 — a named-field enum case is unwritable in literal and pattern position.** `01-surface.md` §1.3 gives
`Reading::Refused { at = p, why = w }` as an example, and the EBNF's `record-literal` and `record-pattern` take
`type-name` as their head. `Refusal::Refused` is a `qualified`, and reading it as a `type-name` requires `module-path`
to hold a capitalized segment, which §1.5's path rule forbids in the same document. Repaired by giving both productions
`(type-name | qualified)` as their head. Forced by §3 (`Refusal::Refused { node, why }`, `Tying::Tied { node }`) and by
§6, where the entire studio error type is named-field cases.

**R4 — `$` must admit an expression, spelled `${ e }`.** `11-quotation.md` §2 says `$x` "splices one value" and every
example splices a bound name. Fifteen of the twenty rewritten construction sites splice a computed value, and a function
body is a block holding exactly one expression (`01-surface.md` §1), so there is nowhere to bind one. Repaired by fixing
the splice grammar as `"$" IDENT | "${" expr "}" | "$.." IDENT`, with `$x` the shorthand for `${x}`. This is the
spelling `01-surface.md` §7 already fixed for kernel quotes, so `11-quotation.md` §6's comparison table gains a shared
row rather than keeping a difference. It adds no capability: `$..xs` already splices a computed list. Forced by §1.1
(`dot_count`), §1.2 (`anchored`, `as_expression`), and §4 (six `stated_field(…)?`).

**R5 — `11-quotation.md` §4's "two halves" is three.** The staff dispatch table has shape decisions, token-kind
decisions, and *notation keywords*, and only the first two are addressed. Repaired by adding the third half with its
answer — a declared enum and one lookup — and by recording that a quote pattern is the wrong tool for it, since a
misspelt quoted identifier is as silent as a misspelt string. Forced by §2.1. The same repair notes that **neither
trialled adapter uses a quote pattern at all**, because the notation both read is not Musa syntax, which §9 records as a
mechanism at risk.

**R6 — `11-quotation.md` §1 needs a third introduction form, and `Cat` needs two fewer cases.** The section says a
refined claim is introduced only by an operation that establishes it, and lists two: a quote and a quote pattern. A
checked parse is a third, and it is required by every site that splices a composer's token into an emitted expression.
Repaired by adding `as_expression : Syntax<TokenTree> -> Option<Syntax<Expr>>` as the third form — generalizing
`checked_expression` rather than adding an operation beside it — and by reducing `Cat` to `Expr` and `TokenTree`, with
the re-opening condition in §9. Forced by §1.2 and §2.2.

**R7 — `02-core-calculus.md` §1.4's K decision is dropped.** Repaired by stating that K is not admitted as an axiom,
that `Id`/`refl`/`J` stay, that `DecEq` supplies uniqueness of identity proofs where a program needs it, and that
admitting K globally is a separate amendment with a program that needs it and has no `DecEq`. The paragraph that
nominates prompt 132 is replaced by what prompt 132 found. Forced by §10, and by the absence of any index unification in
ten programs.

**R8 — prompt 145's acceptance gate is a line count and should be a table.** §12 predicts the rewrite lands at 2,050 ±
100 lines, a 14% reduction, and argues that the line count measures the file rather than the language. The ten-row table
in §12 is what prompt 145 should be gated on. **This trial does not write that repair**: prompt 132's Stop forbids
changing another prompt, and the repair belongs in the commit that acts on it under the prompt README's §6.

**[`42-dependent-core-decision.md`](42-dependent-core-decision.md) needs no repair, and this was checked rather than
assumed.** Note 39 §11.2's five items are the cost of adopting a dependent core — constitution §9's rule, the CBPV and
elimination choices, obligations §10's admission route, the proof obligations that replace Algorithm W, and the prompts
to repair — and the trial changes the answer to none of them. R7 drops an axiom *within* the core note 42 admitted; it
does not change whether the core was worth adopting, which is what note 42 decided and what prompt 145 answers.

## 14. What this trial does not cover

**Hidden constructors have no spelling, and no prompt from 133 to 149 adds one.** Note 28 §7 recorded them as "keep"
with three independent uses, and §5.1 shows what their absence costs: `Chord::NamedChord(ChordSymbol::GSeven,
[Spelling::C])` is a chord whose symbol contradicts its tones, and nothing stops a client writing it. The language has
no `pub`, no `private`, and no module-level visibility at all; `signature`/`structure` hides `let` and `fn` members of a
structure and cannot hold a type. Adding visibility is a new mechanism and prompt 132's Stop forbids it, so this is
recorded as work the pass does not cover and the next prompt to run is the one that writes it, under the prompt README's
§7 with this finding as its evidence.

**Repeated subexpressions have no remedy.** §5.4 writes one product three times and one call twice inside a single
record literal, because a block holds exactly one expression and there is no `let` in it. The workaround — split the
function so the value becomes a parameter — is what §5.1 does and what prompt 142's migration should do, and it is not
always available or always better. This is not a falsifier and not a defect: it is the standing price of `01-surface.md`
§1's rule, it costs work rather than meaning in a total language with no effects, and it is recorded so that a later
proposal has a measurement to argue from rather than a feeling.

**`&&` and `||` have no spelling**, and every conjunction in ten programs is an `if` ladder. `10-traits.md` §5's
operator table has no boolean row and there is no `bool_and` builtin. Recorded rather than repaired, for the same
reason: a program is writable and the case for a spelling should be made with a count.

**Nothing here was type-checked.** §0 says what that limits. The mechanisms marked "nothing" in §7 are the ones a
compiler would have found nothing to say about either, so the paper limitation and the coverage gap point at the same
places: conversion, coverage in the presence of indices, coherence, and the termination measure are all owed executable
evidence by prompts 133–137 and audited by 148.

**Prompt 146's studio adapter is not written here, only its `validate`.** The adapter half — expansion, `edit`, and
`print` over graph declarations — is exercised in this trial only through note 40 §4's programs, which the recursor
findings already cover and which prompts 129–131 do not change. The eight-item coverage list stays prompt 146's.
