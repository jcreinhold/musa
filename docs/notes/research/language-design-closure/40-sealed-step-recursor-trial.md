# Trialling the sealed-step recursor before any code implements it

## Purpose

Note 39 §5 proposes replacing `syntax_fold` — the bottom-up catamorphism prompt 127da made the only way into a syntax
value — with a total inherited-context recursor over **sealed steps**. That proposal had never been written against a
program. This note writes it against five, settles the surface §12.1 left open, and discharges the normalization
obligation §5.2's qualification names.

Nothing here is implementation and nothing here amends `docs/rules/`. Prompt 127dcfaf amends the candidate language
pages and transcribes what is frozen below; prompt 127dcfag rewrites and remeasures the staff adapter on it.

## Verdict

**The interface passes, and is frozen in §11.** Sealing prevents the reassociation that killed the first draft, with no
dynamic owner check, no failure result, no rank-2 region, no affine restriction, and no general recursion. The
reducibility argument goes through (§10) under one premise this note names explicitly and note 39 did not: **definition
acyclicity**, which the compiler already enforces.

Three findings qualify the recommendation, and none of them is a stop:

1. **The fold is not merely awkward for staff; it is incomplete.** A group whose meaning depends on where it stands
   cannot be re-descended under the fold at all — the natural code is a mutual recursion between the piece reader and
   the group reader, and the compiler rejects it as a dependency cycle. The shipped `staff.musa` lives with the
   consequence: anything written inside a `(...)` or `[...]` that is itself a group is silently dropped. §3.3 exhibits
   the rejection.
2. **`Pending` survives.** The recursor removes the shadow tree (`data Read`) and the second traversal over it; it does
   not remove the right-to-left accumulator, which is a fact about the notation and not about the traversal. Note 39
   §5.1 reads as though both would go. §13 corrects it.
3. **Studio does not benefit.** Once 127dcg's division of labour is honoured, no studio declaration needs inherited
   context, and the recursor version of the same program is 36 lines against the fold's 13. The derived bottom-up fold
   therefore stays public, renamed to say how it runs. §12 records the asymmetry.

## 1. What was tried

```text
recurse_syntax(
  missing,            // C -> NodePath -> A
  token,              // C -> NodePath -> Text -> Text -> A
  identifier,         // C -> NodePath -> Text -> A
  group,              // C -> NodePath -> Text -> List<SyntaxStep<C,A>> -> A
  initial_context,    // C
  subject,            // Syntax
) -> A

run_syntax_step(context, next) -> A     // C -> SyntaxStep<C,A> -> A
```

The intrinsic characterization is note 39 §5.2's, unchanged: `R : (C, Syntax) -> A` is the observationally unique
function with

```text
R(c, Missing(path))                       = missing(c, path)
R(c, Token(path, kind, text))             = token(c, path, kind, text)
R(c, Identifier(path, name))              = identifier(c, path, name)
R(c, Group(path, delimiter, [s1..sn]))    = group(c, path, delimiter, [step_R(s1), .., step_R(sn)])
run_syntax_step(c, step_R(s))             = R(c, s)
```

The branch argument order matches today's `syntax_fold` with `C` prepended, and the delimiter is `Text` as the shipped
fold spells it rather than the `Delimiter` sum prompt 127da's design named — that sum never reached the surface.

## 2. How the programs were checked

Every program below was type-checked with `musa check`, with three stand-ins, because the real types do not exist yet:

- `Syntax` became `Text` and `NodePath` became `Nat`. Both are phase-local: ordinary source cannot name them, which the
  checker says in as many words ("unknown type `Syntax` … not a value type"). That is law 11 already true today.
- `syntax_at`, `syntax_number`, `syntax_anchor`, and `syntax_built` became local stubs of the same arity and result
  type.
- `SyntaxStep<C,A>` became the bare arrow `C -> A`, and `run_syntax_step` became `fn (context, next) { next(context) }`.

The third substitution is the one that matters, and it is worth being exact about what it does and does not establish.
It checks arity, argument order, pattern coverage, fold direction, the scoping of `with`, and that the programs are
rank-1 inferable. It cannot check sealing, because **no source-level type can express a sealed step**: the checker
refuses a `data` field of function type — "a stored field may not be a function … an arrow is never storable data" — so
a package can neither declare `SyntaxStep` nor forge one. That refusal is not an obstacle to the trial; it is law 1
holding by construction, and it is why the type must be compiler-owned and excluded from `d`.

Two accidents of the surface were found this way and are recorded because they shape how the programs are written:

- **The branch types have no surface spelling.** In the type grammar, `A -> B` is a *one*-argument arrow and `(A, B)` is
  a product, so a four-argument branch type cannot be written down. Branches are therefore passed as unannotated lambdas
  and inferred — which is what `staff.musa` and `doubled.musa` already do. A helper that takes a whole branch as a
  parameter cannot be written; helpers take the branch's *pieces*, and every program below is factored that way.
- **`step` and `rest` are reserved words**, so a program binds a step as `next` or `kid`. This is a naming note, not a
  design one, but a paper program that used `step` would not have compiled.

## 3. Program one: staff, simplified

`C` is where the reader is plus what it has read since the last pitch; `A` is the same type. The slice reads a braces
body of `c5(3/8)` items with `~` ties, right to left, because what a tie reaches is already in hand when the tie is met.

### 3.1 On the recursor

```musa
data Tying { Untied, Tied }

// The notes read so far, right-nested.
data Notes {
    NoNotes,
    Noted(spot: NodePath, spelling: Text, held: Ratio, tie: Tying, then: Notes),
}

data Refusal { NoRefusal, Refused(spot: NodePath, said: Text) }

// A written value, as its numerals arrive.
data Held { NoHeld, Whole(count: Ratio), Divided(count: Ratio, under: Ratio) }

// What has been read since the last pitch. Reading backwards, a value and a
// tie arrive before the note they belong to.
data Pending { Pending(held: Held, tie: Tying, notes: Notes, refusal: Refusal) }

// Where the reader is. The token branch is one function, so what a numeral
// means arrives with the context rather than being guessed from its digits.
data Place { InBody, InLength }

data State { State(place: Place, pending: Pending) }

// Reading from the end, the last refusal recorded is the leftmost one, which
// is the one to report.
fn refusing(state: State, here: NodePath, said: Text) -> State {
    match state {
        State(place, pending) -> State(place, pending with { refusal = Refused(here, said) }),
    }
}

fn beats(held: Held) -> Option<Ratio> {
    match held {
        NoHeld -> None,
        Whole(count) -> Some(count),
        Divided(count, under) -> match ratio_div(count, under) {
            Ok(value) -> Some(value),
            Err(why) -> None,
        },
    }
}

// Inside parentheses, read left to right: the first numeral is the value and
// the second is what it is divided by.
fn holding(state: State, here: NodePath, count: Ratio) -> State {
    match state {
        State(place, pending) -> match pending {
            Pending(held, tie, notes, refusal) -> match held {
                NoHeld -> State(place, pending with { held = Whole(count) }),
                Whole(first) -> State(place, pending with { held = Divided(first, count) }),
                Divided(first, under) -> refusing(state, here, "this written value states too many numbers"),
            },
        },
    }
}

fn node_at(region: Syntax, here: NodePath) -> Syntax {
    option_fold(region, fn (node: Syntax) -> Syntax { node }, syntax_at(region, here))
}

fn numeral(region: Syntax, state: State, here: NodePath) -> State {
    option_fold(
        refusing(state, here, "this staff cannot read this number"),
        fn (count: Ratio) -> State { holding(state, here, count) },
        syntax_number(node_at(region, here)),
    )
}

// A pitch takes the value and the tie standing to its right and becomes a
// note; what it leaves behind holds neither.
fn noting(state: State, here: NodePath, spelling: Text) -> State {
    match state {
        State(place, pending) -> match pending {
            Pending(held, tie, notes, refusal) -> option_fold(
                refusing(state, here, "this pitch is not written with a value"),
                fn (count: Ratio) -> State {
                    State(place, Pending(NoHeld, Untied, Noted(here, spelling, count, tie, notes), refusal))
                },
                beats(held),
            ),
        },
    }
}

fn tying(state: State) -> State {
    match state {
        State(place, pending) -> State(place, pending with { tie = Tied }),
    }
}

fn trivial(kind: Text) -> Bool {
    if text_equal(kind, "Whitespace") {
        true
    } else if text_equal(kind, "LineComment") {
        true
    } else { text_equal(kind, "BlockComment") }
}

fn token_read(region: Syntax, state: State, here: NodePath, kind: Text, text: Text) -> State {
    match state {
        State(place, pending) -> match place {
            InLength -> if text_equal(kind, "Integer") { numeral(region, state, here) } else { state },
            InBody -> if text_equal(kind, "PitchLiteral") {
                noting(state, here, text)
            } else if text_equal(kind, "Tilde") {
                tying(state)
            } else if trivial(kind) {
                state
            } else { refusing(state, here, "this staff does not know this word") },
        },
    }
}

// Parentheses state a written value and nothing else, so what comes back out
// of them is the value they stated, under the place the reader was already in.
fn held_back(before: State, after: State) -> State {
    match before {
        State(place, pending) -> match after {
            State(inner, read) -> match read {
                Pending(held, tie, notes, refusal) -> State(
                    place,
                    pending with { held = held, refusal = refusal },
                ),
            },
        },
    }
}

// A group decides the order its children are read in and the context each is
// read under. That decision is the whole of what the recursor adds.
fn group_read(
    state: State,
    here: NodePath,
    delimiter: Text,
    kids: List<SyntaxStep<State, State>>,
) -> State {
    match state {
        State(place, pending) -> if text_equal(delimiter, "parentheses") {
            held_back(
                state,
                list_fold_from_start(
                    State(InLength, pending with { held = NoHeld }),
                    fn (kid, earlier) { run_syntax_step(earlier, kid) },
                    kids,
                ),
            )
        } else {
            list_fold_from_end(
                State(InBody, pending),
                fn (kid, later) { run_syntax_step(later, kid) },
                kids,
            )
        },
    }
}

// One region of staff notation, read from the end.
let expand = fn (region) {
    recurse_syntax(
        fn (state, here) { refusing(state, here, "this staff cannot read this") },
        fn (state, here, kind, text) { token_read(region, state, here, kind, text) },
        fn (state, here, name) { refusing(state, here, "this staff does not know this word") },
        fn (state, here, delimiter, kids) { group_read(state, here, delimiter, kids) },
        State(InBody, Pending(NoHeld, Untied, NoNotes, NoRefusal)),
        region,
    )
};
```

### 3.2 On the fold, for comparison

The same slice on `syntax_fold` needs a second type — the tree, read again — because a group's children have to be
carried up unread when what they mean depends on where the group turns out to stand:

```musa
data Read {
    Trivia,
    Piece(spot: NodePath, kind: Text, spelling: Text),
    Grouped(spot: NodePath, delimiter: Text, parts: List<Read>),
    Bodied(spot: NodePath, notes: Notes, refusal: Refusal),
    Unreadable(spot: NodePath),
}

// A second traversal, over the children the fold already visited, because a
// numeral inside parentheses means something a numeral in the body does not
// and only the parent knows which it was.
fn stated_numeral(region: Syntax, one: Read, pending: Pending) -> Pending {
    match one {
        Piece(spot, kind, spelling) -> if text_equal(kind, "Integer") {
            numeral(region, pending, spot)
        } else { pending },
        Trivia -> pending,
        Grouped(spot, delimiter, parts) -> pending,
        Bodied(spot, notes, refusal) -> pending,
        Unreadable(spot) -> pending,
    }
}

fn stated_held(region: Syntax, parts: List<Read>, pending: Pending) -> Pending {
    list_fold_from_start(
        pending with { held = NoHeld },
        fn (one, earlier) { stated_numeral(region, one, earlier) },
        parts,
    )
}

// Only one level deep, and not because that is what the notation wants:
// `group_read` may not call `taken_piece`, because `taken_piece` calls
// `group_read` and definitions may not call each other. Anything written
// inside these parentheses that is itself a group is dropped unread.
fn group_read(
    region: Syntax,
    here: NodePath,
    delimiter: Text,
    parts: List<Read>,
    later: Pending,
) -> Pending {
    if text_equal(delimiter, "parentheses") { stated_held(region, parts, later) } else { later }
}

// The dispatch the fold's four branches already made, made again.
fn taken_piece(region: Syntax, one: Read, later: Pending) -> Pending {
    match one {
        Trivia -> later,
        Unreadable(spot) -> refusing(later, spot, "this staff cannot read this"),
        Piece(spot, kind, spelling) -> token_read(later, spot, kind, spelling),
        Grouped(spot, delimiter, parts) -> group_read(region, spot, delimiter, parts, later),
        Bodied(spot, notes, refusal) -> later,
    }
}

fn read_body(region: Syntax, parts: List<Read>) -> Pending {
    list_fold_from_end(
        Pending(NoHeld, Untied, NoNotes, NoRefusal),
        fn (one, after) { taken_piece(region, one, after) },
        parts,
    )
}
```

### 3.3 The measurement, and the rejection

Counting only the definitions one version has and the other does not:

|  | recursor | fold |
| --- | --- | --- |
| traversal-specific lines | 34 (`Place`, `State`, `held_back`, `group_read`) | 44 (`Read`, `stated_numeral`, `stated_held`, `group_read`, `taken_piece`, `read_body`) |
| constructor arms written for a shadow tree | 0 | 10 (five in `stated_numeral`, five in `taken_piece`) |
| traversals over the same children | 1 | 2 |
| nested group inside `(...)` | read | dropped |

Ten lines is not the finding. The last row is. The fold version above is only well-formed because `group_read` gives up:
written the way the notation actually wants — a group inside parentheses read the way a group in the body is read — the
compiler rejects it outright:

```text
dependency-cycle
  × these definitions call each other
      the cycle is group_read → taken_piece → group_read
  help: pass the changing value as a parameter, or replace the recursion with a finite fold
```

The help is exactly right and exactly unavailable: the finite fold is what produced `Read` in the first place, and
folding `Read` again is the recursion the checker just refused. The shipped `stdlib/src/adapters/staff.musa` has the
same shape and therefore the same hole — every one of its helpers that meets a nested `Grouped` answers `false`, `None`,
or "unchanged" (lines 535, 573, 625, 660). So the fold does not merely make the staff reader awkward; it makes a staff
reader that reads nested groups **unwritable**. Prompt 127dcfag must close that hole and say that it did.

What the recursor does *not* remove is `Pending`. Reading a word after its arguments is a fact about this notation, and
the same accumulator would appear in a hand-written recursive-descent reader. Note 39 §5.1 lists "context, lookahead,
order, and failure" as encoded in `Pending` and closures; context and order move into the traversal, and lookahead and
failure stay where they were. §13 corrects the impression that all four move.

## 4. Program two: studio

The slice reads a graph body whose children are declaration lines, each either `processor NAME = DESC { PARAM = VALUE }`
or `connect FROM -> TO`.

### 4.1 Shared values

```musa
data Params { NoParams, Param(spot: NodePath, name: Text, value: Text, then: Params) }

data Decl {
    Unread(spot: NodePath, said: Text),
    Processor(spot: NodePath, name: Text, descriptor: Text, params: Params),
    Connect(spot: NodePath, from: Text, onto: Text),
}

data Decls { NoDecls, Declared(one: Decl, then: Decls) }

// The words of one line, in the order they were written. Three is every form
// this graph has, and an empty spelling is a slot not yet filled.
data Header { Header(first: Text, second: Text, third: Text) }

let no_header = Header("", "", "");

fn empty(word: Text) -> Bool { text_equal(word, "") }

fn worded(header: Header, word: Text) -> Header {
    match header {
        Header(first, second, third) -> if empty(first) {
            Header(word, second, third)
        } else if empty(second) {
            Header(first, word, third)
        } else if empty(third) {
            Header(first, second, word)
        } else { header },
    }
}

// What a child contributes to the group above it.
data Read {
    Trivia,
    Word(spot: NodePath, text: Text),
    Stated(spot: NodePath, name: Text, value: Text),
    Body(spot: NodePath, params: Params),
    Declaration(spot: NodePath, one: Decl),
    Graph(spot: NodePath, decls: Decls),
    Unreadable(spot: NodePath),
}

fn worded_by(one: Read, header: Header) -> Header {
    match one {
        Word(spot, text) -> worded(header, text),
        Trivia -> header,
        Stated(spot, name, value) -> header,
        Body(spot, params) -> header,
        Declaration(spot, decl) -> header,
        Graph(spot, decls) -> header,
        Unreadable(spot) -> header,
    }
}

fn body_of(one: Read, found: Params) -> Params {
    match one {
        Body(spot, params) -> params,
        Trivia -> found,
        Word(spot, text) -> found,
        Stated(spot, name, value) -> found,
        Declaration(spot, decl) -> found,
        Graph(spot, decls) -> found,
        Unreadable(spot) -> found,
    }
}

fn stated_by(one: Read, later: Params) -> Params {
    match one {
        Stated(spot, name, value) -> Param(spot, name, value, later),
        Trivia -> later,
        Word(spot, text) -> later,
        Body(spot, params) -> later,
        Declaration(spot, decl) -> later,
        Graph(spot, decls) -> later,
        Unreadable(spot) -> later,
    }
}

fn declared_by(one: Read, later: Decls) -> Decls {
    match one {
        Declaration(spot, decl) -> Declared(decl, later),
        Trivia -> later,
        Word(spot, text) -> later,
        Stated(spot, name, value) -> later,
        Body(spot, params) -> later,
        Graph(spot, decls) -> later,
        Unreadable(spot) -> Declared(Unread(spot, "this studio cannot read this"), later),
    }
}

// One line: a declaration if its first word says so, and otherwise the
// `name = value` a parameter body is written out of. Nothing above the line
// has to say which, because the line says it itself.
fn line_read(here: NodePath, header: Header, params: Params) -> Read {
    match header {
        Header(first, second, third) -> if text_equal(first, "processor") {
            Declaration(here, Processor(here, second, third, params))
        } else if text_equal(first, "connect") {
            Declaration(here, Connect(here, second, third))
        } else if empty(third) {
            Stated(here, first, second)
        } else {
            Declaration(here, Unread(here, "this studio does not know this declaration"))
        },
    }
}

fn word_read(here: NodePath, kind: Text, text: Text) -> Read {
    if text_equal(kind, "Whitespace") {
        Trivia
    } else if text_equal(kind, "LineComment") {
        Trivia
    } else if text_equal(kind, "Equals") {
        Trivia
    } else if text_equal(kind, "Arrow") {
        Trivia
    } else { Word(here, text) }
}
```

### 4.2 On the fold

```musa
fn fold_group(here: NodePath, delimiter: Text, kids: List<Read>) -> Read {
    if text_equal(delimiter, "braces") {
        Body(here, list_fold_from_end(NoParams, fn (kid, later) { stated_by(kid, later) }, kids))
    } else if text_equal(delimiter, "brackets") {
        Graph(here, list_fold_from_end(NoDecls, fn (kid, later) { declared_by(kid, later) }, kids))
    } else {
        line_read(
            here,
            list_fold_from_start(no_header, fn (kid, header) { worded_by(kid, header) }, kids),
            list_fold_from_start(NoParams, fn (kid, found) { body_of(kid, found) }, kids),
        )
    }
}

let expand = fn (region) {
    syntax_fold(
        fn (here) { Unreadable(here) },
        fn (here, kind, text) { word_read(here, kind, text) },
        fn (here, name) { Word(here, name) },
        fn (here, delimiter, kids) { fold_group(here, delimiter, kids) },
        region,
    )
};
```

### 4.3 On the recursor

```musa
data Reading { InGraph, InLine, InBody }

fn recursor_group(
    context: Reading,
    here: NodePath,
    delimiter: Text,
    kids: List<SyntaxStep<Reading, Read>>,
) -> Read {
    if text_equal(delimiter, "braces") {
        Body(
            here,
            list_fold_from_end(
                NoParams,
                fn (kid, later) { stated_by(run_syntax_step(InLine, kid), later) },
                kids,
            ),
        )
    } else if text_equal(delimiter, "brackets") {
        Graph(
            here,
            list_fold_from_end(
                NoDecls,
                fn (kid, later) { declared_by(run_syntax_step(InLine, kid), later) },
                kids,
            ),
        )
    } else {
        line_read(
            here,
            list_fold_from_start(
                no_header,
                fn (kid, header) { worded_by(run_syntax_step(InLine, kid), header) },
                kids,
            ),
            list_fold_from_start(
                NoParams,
                fn (kid, found) { body_of(run_syntax_step(InBody, kid), found) },
                kids,
            ),
        )
    }
}
```

### 4.4 What the studio program shows

`Reading` is threaded and never read. Nothing in this graph's expansion needs it: 127dcg assigns descriptors, units,
ranges, ports, bindings, and cycles to `validate`, an ordinary total package function over the finished description, and
leaves expansion "name shape, balanced paths, unit spelling, duplicate parameter text on one node, and the grammar of a
connection". None of those is decided above the line, and a line says which form it is with its own first word.

The two group branches measure 13 lines (fold) against 35 (recursor), for the same answers. The recursor also runs the
children twice in the line case — once for the header words and once for the body — which the fold does not, because the
fold's children are already values. That is not a defect of the recursor; it is what selective descent costs when
nothing is being selected.

Note 39 §5.4's studio sketch — "a node declaration can inspect its header before selecting the parameter grammar for its
body" — describes a language studio does not have. §13 corrects it.

## 5. Program three: the anchored edit

The edit command names a node by **anchor**: its pre-order position in the region's own reading order, which is the only
name the compiler and a later package function both have for it (prompt 127dcb, and `syntax_anchor`'s registry entry).
`C` is the anchor wanted; `A` is whether the locus has been found.

```musa
data Found { NotHere, FoundIt(spot: NodePath, spelling: Text) }

// The anchor of the node at `here`, as the compiler's own number. The adapter
// hands the number back and reads nothing else from it.
fn anchored(region: Syntax, here: NodePath) -> Option<Ratio> {
    option_fold(
        None,
        fn (node: Syntax) -> Option<Ratio> { syntax_number(node) },
        syntax_anchor(region, here, syntax_built(here, 0, 0)),
    )
}

fn token_seek(region: Syntax, wanted: Ratio, here: NodePath, kind: Text, text: Text) -> Found {
    if text_equal(kind, "PitchLiteral") {
        option_fold(
            NotHere,
            fn (anchor: Ratio) -> Found {
                if ratio_equal(anchor, wanted) { FoundIt(here, text) } else { NotHere }
            },
            anchored(region, here),
        )
    } else { NotHere }
}

// Steps left to right, and no step at all once the locus is in hand. A fold
// has already evaluated every child by the time this decision could be made,
// which is the whole of the difference.
fn group_seek(
    wanted: Ratio,
    here: NodePath,
    delimiter: Text,
    kids: List<SyntaxStep<Ratio, Found>>,
) -> Found {
    list_fold_from_start(
        NotHere,
        fn (kid, found) {
            match found {
                FoundIt(spot, spelling) -> found,
                NotHere -> run_syntax_step(wanted, kid),
            }
        },
        kids,
    )
}

let locate = fn (region, wanted) {
    recurse_syntax(
        fn (wanted, here) { NotHere },
        fn (wanted, here, kind, text) { token_seek(region, wanted, here, kind, text) },
        fn (wanted, here, name) { NotHere },
        fn (wanted, here, delimiter, kids) { group_seek(wanted, here, delimiter, kids) },
        wanted,
        region,
    )
};
```

The locus comes back as a `NodePath` the recursor supplied, which is what prompt 127dcfb's replacement is anchored to,
so the path law does the identifying and the adapter forges nothing. The traversal **omits** every step after the match
— the first program in this trial to exercise law 6's omission half, and something a fold cannot do at all, since by the
time its group branch runs, every child has already been evaluated.

## 6. Program four: the degenerate leaves

```musa
fn leaf_missing(context, here) { NotHere }
fn leaf_token(context, here, kind, text) { NotHere }
fn leaf_identifier(context, here, name) { NotHere }
```

Stated because a case that is obvious in prose is where a law hides. A leaf branch receives no step, so it mints
nothing, runs nothing, and cannot re-enter the recursor through the interface at all. Its termination argument is that
`R(c, Missing(p))`, `R(c, Token(p, k, t))`, and `R(c, Identifier(p, n))` each reduce in one step to an application of a
reducible branch to reducible arguments. It is the base case of §10's inner induction and carries no side conditions.

The one thing the degenerate case does *not* say is that a leaf branch cannot recurse: it can start a fresh
`recurse_syntax` on any `Syntax` value in scope, exactly as a group branch can. That is program five's territory, and
the leaf branches are not a special case of it.

## 7. Program five: hostile nested traversal

`C` and `A` are both `Nat -> Nat`. An outer group branch captures its first step into an `Option`, so the step outlives
the branch that received it; answers with a closure, so the step is used after that branch has returned; and, inside
that closure, starts a **fresh recursor on the original subject** whose group branch runs the captured **outer** step
twice, under two different contexts.

```musa
// The first step of a group, kept in an ordinary container so that it outlives
// the branch that received it.
fn first_of(kids: List<SyntaxStep<Nat -> Nat, Nat -> Nat>>) -> Option<SyntaxStep<Nat -> Nat, Nat -> Nat>> {
    list_fold_from_end(None, fn (kid, found) { Some(kid) }, kids)
}

// The inner recursor's group branch: it runs a step it did not mint, twice,
// under two different contexts, and neither run is one of its own children.
fn inner_group(kid, inner, spot, delimiter, others) {
    run_syntax_step(run_syntax_step(inner, kid), kid)
}

// The outer group branch: capture the first step, and answer with a function
// that starts a fresh recursor on the *original* subject and runs the captured
// step from inside it, after this branch has already returned.
fn outer_group(region, context, here, delimiter, kids) {
    option_fold(
        context,
        fn (kid) {
            fn (n: Nat) {
                recurse_syntax(
                    fn (inner, spot) { inner },
                    fn (inner, spot, kind, text) { inner },
                    fn (inner, spot, name) { inner },
                    fn (inner, spot, delimiter, others) { inner_group(kid, inner, spot, delimiter, others) },
                    run_syntax_step(context, kid),
                    region,
                )(n)
            }
        },
        first_of(kids),
    )
}

let hostile = fn (region) {
    recurse_syntax(
        fn (context, here) { context },
        fn (context, here, kind, text) { context },
        fn (context, here, name) { context },
        fn (context, here, delimiter, kids) { outer_group(region, context, here, delimiter, kids) },
        fn (n: Nat) { n },
        region,
    )
};
```

Every hazard §12.1 names is present: same `C` and `A` in both recursors, a captured step crossing into a foreign
traversal, a restart on an ancestor (here the original subject itself), function-valued context and answer, capture
inside a closure, delayed invocation after the branch returned, repeated invocation under two contexts, and a step
stored in a container.

**What sealing decides.** `run_syntax_step(c, kid)` is not a request addressed to whatever recursor lexically encloses
it. The equation `run_syntax_step(c, step_R(s)) = R(c, s)` fixes both halves at mint time: `s` is the outer child, `R`
is the outer algebra. `inner_group` cannot reinterpret `kid` as one of its own children because it has no operation that
takes a step apart, and no way to build a step of its own. The inner recursor's own steps and the outer's have the same
*type* and different *values*, and no dynamic check is needed to keep them apart — which is precisely what the first
draft's `SyntaxChild<C,A>` plus a separately supplied `descend` could not say, since there `descend_inner(c, child)`
type-checked and would have run an outer child under the inner algebra. Under sealing there is no failure result, no
rank-2 region, no affine use restriction, and nothing an owner check would have to reject at run time.

**What sealing does not decide** is termination, and §10 is where that is discharged.

## 8. The eleven laws against the five programs

| # | Law | Exercised by | Evidence prompt 127dcfaf owes |
| --- | --- | --- | --- |
| 1 | Sealed formation | all five | Compile-fail: `SyntaxStep` has no constructor in scope, and a `data` field of function type is refused (already true — §2). |
| 2 | Association | program 5 | Differential test: the outer step, run from the inner recursor's group branch, answers with the outer algebra applied to the outer child. |
| 3 | Inherited context | programs 1, 2, 3 | A branch that records its context; the recorded sequence equals the contexts the group branch passed, with no ambient state. |
| 4 | Path uniqueness | programs 1, 3 | The paths a recursor run collects equal, in order, the paths `syntax_fold_from_leaves` reports for the same region. |
| 5 | Structural decrease and reducibility | programs 4, 5 | §9's lemma as a property test on a generated tree; §10's proof written into `05-verification.md`; program 5 terminating under a finite budget. |
| 6 | Repeatability | programs 3, 5 | Omission: program 3 over a region whose later siblings would refuse, answering without refusing. Repetition: program 5's two runs of one step under two contexts, agreeing on subject and algebra. |
| 7 | Determinism | all five | Two runs of program 1 over one region agree on value **and** on charge. |
| 8 | Opacity | all five | Compile-fail: no operation but `run_syntax_step` accepts a `SyntaxStep`; no operation yields a path, range, scope, or algebra from one. |
| 9 | Fold derivation | program 2 | Program 2's two versions produce equal values and equal charges; `syntax_fold_from_leaves` charges exactly what today's `syntax_fold` charges, so no shipped adapter's budget moves. |
| 10 | Budget accounting | programs 3, 5 | Charges: minting `n` steps costs the group's `n`; an omitted step costs nothing beyond its mint; a step run twice costs twice. |
| 11 | Phase conservativity | all five | `Syntax`, `NodePath`, and `SyntaxStep` are unknown types in ordinary source (already true for the first two — §2); the completed phase result is storable; the transformer uses the one checker and the one evaluator. |

No law is unexercised, and no program is redundant: removing program 3 loses law 6's omission half, removing program 5
loses laws 2 and 5, and removing program 4 loses the base case §10's induction needs.

## 9. Sealed association and local decrease

**Lemma (association and decrease).** Let `R` be the function determined by an algebra and let `step_R(s)` be the step
minted for a proper child `s` of a group `Group(p, d, [s1..sn])` during the evaluation of `R(c, Group(p, d, [s1..sn]))`.
Then for every context `c'`, `run_syntax_step(c', step_R(s))` reduces to `R(c', s)`, and `s` is a strict subtree of that
group.

*Proof.* Immediate from the intrinsic equations. Minting is the only way a step comes into existence (law 1: there is no
constructor and, as §2 records, no source-level type that could hold one), and minting occurs only in the group case,
where each `si` is by construction an immediate proper child. Running is the only elimination, and its equation names
`R` and `s` — neither of which any operation can replace, because nothing takes a step apart. Hence the pair `(R, s)`
sealed at mint time is the pair used at every run, however many runs there are, wherever they occur, and whichever
recursor is lexically enclosing. ∎

This is the whole of what sealing buys, and it is worth being exact that it is *not* a termination theorem. It says each
individual step application descends. It does not say that the tree being evaluated shrinks at every reduction, and in
program 5 it does not: the inner recursor restarts on the original subject, which is larger than the child whose step is
in flight. A proof that tried to exhibit one globally decreasing runtime tree-size measure would fail on exactly that
trace. §10 does not try.

## 10. Normalization

The phase language is the source language's calculus with `Syntax`, `NodePath`, the phase builtins, and now
`SyntaxStep<C,A>`. Strong normalization is proved the way it already is for the calculus — Tait reducibility — with one
new type clause and two new fundamental-lemma cases.

### 10.1 The premise

**Definition acyclicity.** Definitions within a piece form a directed acyclic graph; the compiler rejects a cycle with
`DependencyCycle` ("these definitions call each other"), as §3.3 exhibits. There is no `fix`, no `letrec`, and no
recursive lambda.

This premise is doing real work below and note 39 §5.2's qualification does not mention it. Without it, an algebra could
refer to a definition that refers back to the algebra, and the fundamental lemma's induction on terms would be circular:
a group branch could start a recursor whose algebra is itself, on a subject it chooses, with no decrease anywhere. The
premise costs nothing new — it is already enforced, for reasons that predate this design — but it must be stated,
because it is what makes the *term* induction well-founded.

### 10.2 The reducibility candidate

By induction on types, over closed well-typed phase terms:

- base types (`Nat`, `Ratio`, `Text`, `Bool`, `NodePath`, `Syntax`, declared data): `t` is reducible iff `t` is strongly
  normalizing;
- `T1 -> T2`: `t` is reducible iff `t` is SN and `t(u)` is reducible at `T2` for every reducible `u : T1`;
- `List<T>`, `Option<T>`, products, sums: `t` is SN and every component is reducible;
- **`SyntaxStep<C,A>`: `t` is reducible iff `t` is SN and `run_syntax_step(c, t)` is reducible at `A` for every
  reducible `c : C`.**

The step clause is a *hereditary* condition in the sense of the Tait-computability reading of logical relations: the
predicate at a type constructor is defined by the action of its eliminator, exactly as the arrow clause is. It is
universally quantified over contexts, which is what makes it survive capture, storage, delay, and repetition — those
operations do not choose the context, and the clause has already promised every context.

### 10.3 The fundamental lemma, new cases

The lemma is the usual one — every well-typed term with reducible values substituted for its free variables is reducible
— proved by induction on the typing derivation. Acyclicity (§10.1) is what makes that induction well-founded across
definitions.

**Case `recurse_syntax(m, tk, id, g, c0, s)`.** Assume `m`, `tk`, `id`, `g`, `c0`, and `s` reducible. `s` is a value of
type `Syntax`, which is storable data with no arrow at any depth, hence a finite tree. Prove by an inner induction on
the size of `s`, with the statement universally quantified over contexts:

> for every reducible `c`, `recurse_syntax(m, tk, id, g, c, s)` is reducible.

- `s = Missing(p)` / `Token(p, k, x)` / `Identifier(p, n)`: the term reduces to `m(c, p)` / `tk(c, p, k, x)` /
  `id(c, p, n)`, applications of reducible functions to reducible arguments, hence reducible. This is program four.
- `s = Group(p, d, [s1..sn])`: the term reduces to `g(c, p, d, [step(s1), .., step(sn)])`. Each `step(si)` is reducible
  at `SyntaxStep<C,A>`: it is a value, hence SN, and for every reducible `c'`, `run_syntax_step(c', step(si))` reduces
  to `recurse_syntax(m, tk, id, g, c', si)`, which is reducible by the inner induction hypothesis, since `si` is a
  strict subtree (§9). A list of reducible elements is reducible, so `g` applied to reducible arguments is reducible. ∎

**Case `run_syntax_step(c, e)`.** Assume `c` and `e` reducible. Reducibility of `e` at `SyntaxStep<C,A>` is exactly the
statement that `run_syntax_step(c, e)` is reducible at `A` for reducible `c`. ∎

### 10.4 What the hostile terms need, and where they get it

- **Function-valued `C` and `A`.** Nothing in either case constrains `C` or `A`; the candidate is defined at every type,
  and the group case only needs `g` reducible at its own type. `A = Nat -> Nat` is already legal today — `syntax_fold`'s
  answer variable is `Kind::Ordinary`, so "a fold may build a list of functions" is a property the shipped compiler
  already has.
- **Capture in a closure.** A step captured in a lambda body is a reducible value substituted for a free variable; the
  λ-case of the fundamental lemma requires nothing more.
- **Storage in a container.** `Option<SyntaxStep<C,A>>` is reducible when its component is, by the container clause. The
  step is not re-derived on the way out.
- **Delayed use.** The step clause quantifies over all reducible contexts, with no reference to when the run happens.
- **Repeated use, under different contexts.** Likewise: the clause is a statement about every context, so two runs are
  two instances of one promise, and §9 says both use the same sealed pair.
- **A nested recursor on the original subject.** The inner `recurse_syntax` appears inside `g`'s body. `g`'s
  reducibility is established by the *term* induction, before and independently of the value induction on subjects;
  within `g`'s proof, the inner `recurse_syntax` is handled by its own instance of the case above, with its own value
  induction on its own subject. The two inductions are lexicographic — outer on the typing derivation, inner on the
  subject value — and neither appeals to the other's conclusion. Acyclicity is what forbids the one term that would
  break this: an algebra that is its own descendant in the definition graph.

No case required a dynamic owner check, a failure result for `run_syntax_step`, a rank-2 region, an affine or linear use
restriction, or general recursion. **The stop condition in prompt 127dcfae's Design is not met, and the interface is
frozen.**

## 11. The settled surface

Frozen, and to be transcribed by prompt 127dcfaf:

| Question §12.1 left open | Settled as | What it beat, and why |
| --- | --- | --- |
| Names | `recurse_syntax`, `run_syntax_step`, `SyntaxStep<C, A>` | `fold_syntax_with_context` — accurate and it buries the one word that matters. |
| Branch order | `missing`, `token`, `identifier`, `group`, then `initial_context`, then `subject` | Context or subject first: rejected because the shipped fold reads algebra-then-subject and no program wanted the change. |
| Context argument | `C` is the **first** argument of every branch | Last: rejected because `group`'s last argument is the step list and burying the context behind it reads worse in all four programs. |
| Child paths | **Not** paired with the step; a branch receives its own path when it is entered | `List<(NodePath, SyntaxStep<C,A>)>`: rejected because no program needed it — program 3 selects by anchor, not by path — and the pair costs a destructuring in staff's group branch, which is the branch that folds steps most. It also hands a branch a way to name a child without entering it, which is information the interface does not otherwise give. |
| Invocation | the builtin `run_syntax_step(c, next)` | Callable syntax `next(c)`: rejected twice over. It would make a step a function type, so any `C -> A` would unify with it and sealing would stop being a type-level fact; and an arrow-shaped step could not be excluded from `d` for the reason it must be. |
| Argument order of the runner | `run_syntax_step(context, next)` | `(next, context)`: rejected because the folding use is `fn (kid, later) { run_syntax_step(later, kid) }`, and the context belongs where the accumulator is. |
| Leaf payloads | unchanged from the fold: `(C, NodePath)`, `(C, NodePath, Text, Text)`, `(C, NodePath, Text)` | Passing the node itself, so `syntax_number` needs no `syntax_at` round trip: a real wart, rejected here because it is separable from inherited context and should be argued on its own evidence rather than smuggled through a traversal change. |
| Delimiter | `Text`, as shipped | The `Delimiter` sum of prompt 127da's design: rejected because it never reached the surface and this trial is not the place to introduce it. |
| Kinding | `SyntaxStep<C, A>` is a phase-local nominal constructor at kind *ordinary*, excluded from `d`; `C` and `A` are ordinary | Nothing — §2's "a stored field may not be a function" makes any other answer unsound, since a step hides the algebra. |
| Type grammar | `SyntaxStep<C, A>` is spellable **in the phase environment only**, as the first two-argument type constructor | Leaving it unspellable: rejected because then every group branch would have to be one inline lambda; program 1's `group_read` and program 2's `recursor_group` both take the step list as a parameter. |
| The derived fold | stays public, renamed **`syntax_fold_from_leaves`**, specified as `recurse_syntax` at a context nothing reads | Keeping the name `syntax_fold`: rejected because note 39 requires the name to make bottom-up behaviour explicit, and prompt 127dcfaa has just established in this same series that a fold's name must say which end it runs from. Deleting it: rejected because §12 shows a real adapter that reads better on it. |
| Registry | all three join `PhaseFamily::Fold`; each states what it hides | Nothing; the registry requires an entry and a hidden-information string per operation. |

Hidden information, for the registry entries prompt 127dcfaf writes:

- `recurse_syntax` — "the reader's node representation, each node's structural path, and the suspended entry into a
  proper child";
- `run_syntax_step` — "which child and which algebra a step was minted for";
- `syntax_fold_from_leaves` — unchanged from today's `syntax_fold`.

## 12. Staff/studio asymmetry

The two adapters disagree, and the disagreement has a mechanical statement rather than a stylistic one.

**Both interfaces pay for a uniform `A`.** A traversal with one answer type forces a sum of "what a child can
contribute" — `Read` in program 2, `State` in program 1. That is not a fold defect and the recursor does not remove it.

**Only the fold pays for a `Grouped(path, delimiter, List<A>)` constructor** — a subtree carried up unread for an
ancestor to interpret — and it pays only when a group's meaning depends on inherited information. Staff has such groups:
a numeral inside `(...)` is a written value and the same numeral in the body is not, and only the parent knows which.
Studio has none: a line says which form it is with its own first word, and everything a descriptor decides belongs to
`validate` rather than to expansion.

So the finding is not "the recursor is better" but "inherited context is worth what the notation's ambiguity costs":

|  | staff | studio |
| --- | --- | --- |
| a child's meaning depends on its parent | yes | no |
| shadow-tree constructor under the fold | required | not required |
| re-descent under the fold | **unwritable** (§3.3) | not needed |
| traversal-specific lines, fold → recursor | 44 → 34 | 13 → 36 |
| verdict | recursor | fold |

This is a legitimate answer to prompt 127dcfae's second question and it is why the derived fold stays public. Prompt
127dcg writes the studio adapter against the implemented recursor and either confirms this or reports a divergence from
this paper program, which would itself be a finding about the trial.

## 13. Corrections to note 39 §5

**Correction 1 — §5.1 overstates what moves.** "That makes the staff reader encode context, lookahead, order, and
failure in `Pending` and closures" reads as though the recursor removes all four. It removes context and order. Reading
a word after its arguments is a fact about staff notation, so lookahead stays in an accumulator and failure stays beside
it; the same accumulator would appear in a hand-written descent. What the recursor removes is the *shadow tree* and the
second traversal over it. §3.3 measures both halves.

**Correction 2 — §5.4's studio sketch describes a language studio does not have.** "A node declaration can inspect its
header before selecting the parameter grammar for its body" is true of a language whose parameter grammar varies by
descriptor. Under prompt 127dcg's division, expansion checks nothing that varies that way, and every line names its own
form. §12 replaces the sketch with the measured comparison.

**Correction 3 — §5.2's qualification omits its own premise.** The reducibility argument it calls for is sound only
because definitions are acyclic, which is what keeps the induction on typing derivations well-founded when an algebra's
body starts a fresh recursor. The premise is already enforced by the compiler; §10.1 states it, and prompt 127dcfaf's
amendment must carry it.

**Not a correction — §5.2's schematic survives intact.** The branch shapes, the intrinsic equations, the `d` exclusion,
and the `C = Unit` derivation are frozen as written. §11's settlements fill in what the schematic left open rather than
changing it.

**Newly found, and not in §5 at all.** The fold's incompleteness (§3.3) is a stronger claim than note 39 makes anywhere.
It is not that the fold is awkward; it is that a fold-based reader of nested groups cannot be written, and the shipped
staff adapter has the hole to prove it.

## 14. What prompt 127dcfaf owes

- The amendments to `docs/rules/language/00-semantics.md` and `02-core-calculus.md` (the type, its kinding, its `d`
  exclusion, the intrinsic equations, and the acyclicity premise §10.1 names), and to `05-verification.md` (the eleven
  laws and §10's proof shape) — before any code, in rules-before-code order.
- `SyntaxStep<C, A>`, `recurse_syntax`, `run_syntax_step`, and `syntax_fold_from_leaves`, exactly as §11 freezes them.
- Every row of §8's evidence column.
- Cost-table entries for minting and running a step, with law 9's charge equality: `syntax_fold_from_leaves` charges
  what `syntax_fold` charges today, so no shipped adapter's budget moves.
- Prompt 127dcfag then rewrites the staff adapter, remeasures against note 38's reading of its shape and §3.3 here, and
  closes the nested-group hole §3.3 exhibits — or reports that it could not, which under §12.1's falsifier would be
  evidence against this interface rather than against the adapter.
