//! What reading a notated block promises.
//!
//! Beside the module rather than in `tests/suite/`, for [`crate::lower::laws`]'s
//! reason: the answer is a [`musa_core::Raw`], the context is
//! [`crate::registry::owned`], and both are private to this crate.
//!
//! # The two properties a law here can have
//!
//! **The core accepts what the fold writes.** `musa-core` decides what inhabits
//! `EventTrack ⟨written⟩`, so agreeing with it is the whole of what a reading can
//! be right about, and every law that builds a track hands it to
//! [`musa_core::check`] rather than inspecting its shape and calling that
//! agreement.
//!
//! **Two spellings are one term.** `00-semantics.md` §3 says the function and
//! block spellings of transpose, stretch, retrograde, and inversion "invoke the
//! same semantic action, so their equality is an implementation theorem rather
//! than a duplicated convention". A theorem is a law, and stating it needs a
//! comparison up to the site numbers the two source texts necessarily differ in
//! — [`shape`] is that comparison and nothing more.
//!
//! What these do *not* test is what the core already refuses: no law here
//! asserts that `follow` has two arguments, or that a track is not a natural
//! number. Where a form is refused *at the node*, the law is that the complaint
//! is at the node, which is the part the core cannot do.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_core::{Cx, Origin, Raw, RawShape, Term};
use musa_language::{SyntaxKind, SyntaxNode};

use super::super::items::{Definition, Item};
use super::super::{Lowering, Sites};
use crate::diagnose::{Code, Diagnostic};
use crate::resolve::Resolver;

// ---- reading a written program back out of a parse ----

/// The tree `source` parses to, with a loud failure when it does not parse.
///
/// Every law below writes real `.musa` and reads the node it wants out of the
/// tree. That is deliberate and it is what caught the two shapes this module's
/// [`super::statements`] exists to reconcile: a `music` expression holds its
/// items directly and `transpose up M3 { … }` holds them under a `Block`, which
/// is a fact about the parser that no hand-assembled tree would have contained.
fn parsed(source: &str) -> SyntaxNode {
    let document = musa_language::parse(source);
    assert!(
        document.errors().is_empty(),
        "the law's own source parses: {:?}",
        document.errors()
    );
    document.syntax()
}

/// The first descendant of `root` whose kind is `wanted`.
fn first(root: &SyntaxNode, wanted: SyntaxKind) -> SyntaxNode {
    root.descendants()
        .find(|node| node.kind() == wanted)
        .unwrap_or_else(|| panic!("the tree holds a {wanted:?}"))
}

/// One lowered expression, and everything the lowering had to say about it.
struct Read {
    /// The term, absent when a statement was refused.
    built: Option<Raw>,
    /// What was refused, in the order it was reported.
    complaints: Vec<Diagnostic>,
}

impl Read {
    /// The term, with a loud failure when the source was refused.
    fn term(&self) -> &Raw {
        assert!(
            self.complaints.is_empty(),
            "this law's source lowers without complaint: {:?}",
            self.complaints
        );
        self.built.as_ref().expect("the source lowers")
    }

    /// Every fact this reading constructed, in the order the fold made them.
    ///
    /// `sounded` and `play` are the two, and naming them rather than "every
    /// call" is what lets a law say "three pitches, three constructions"
    /// without counting the `together`s that stack them.
    ///
    /// Innermost first, because that is the order they were written: the note
    /// inside `transpose up M3 { c4/4 }` is constructed before the
    /// transposition that encloses it.
    fn constructions(&self) -> Vec<&Raw> {
        let mut found = Vec::new();
        collect(self.term(), &mut found);
        found
    }

    /// The last fact constructed — the outermost one the fold reached.
    fn outermost(&self) -> &Raw {
        self.constructions()
            .pop()
            .unwrap_or_else(|| panic!("this law's source constructs a fact"))
    }
}

/// Every `sounded` and `play` in `raw`, innermost first.
fn collect<'a>(raw: &'a Raw, found: &mut Vec<&'a Raw>) {
    let (head, arguments) = spine(raw);
    for argument in &arguments {
        collect(argument, found);
    }
    if matches!(head.shape(), RawShape::Var(name) if &**name == "sounded" || &**name == "play") {
        found.push(raw);
    }
}

/// The expression `written` denotes, lowered where a value is written.
///
/// `-> Nat` is a lie the lowering never reads: [`Lowering::value`] holds no
/// context and checks nothing, which is [`crate::lower::laws`]'s own arrangement
/// and the reason a law can write one program instead of a whole piece.
fn read(written: &str) -> Read {
    let root = parsed(&format!("library {{ fn probe() -> Nat {{ {written} }} }}"));
    let node = first(&root, SyntaxKind::MusicExpr);
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let built = Lowering::new(&mut resolver, &mut sites).value(&node);
    Read {
        built,
        complaints: resolver.diagnostics,
    }
}

/// The declaration `written` lowers to.
fn declared(written: &str, wanted: SyntaxKind) -> (Option<Item>, Vec<Diagnostic>) {
    let root = parsed(&format!("library {{ {written} }}"));
    let node = first(&root, wanted);
    let mut resolver = Resolver::new();
    let mut sites = Sites::default();
    let item = Lowering::new(&mut resolver, &mut sites).item(&node);
    (item, resolver.diagnostics)
}

/// The definition `written` lowers to, with a loud failure when it complains.
fn definition(written: &str, wanted: SyntaxKind) -> Definition {
    let (item, complaints) = declared(written, wanted);
    assert!(
        complaints.is_empty(),
        "`{written}` lowers without complaint: {complaints:?}"
    );
    match item {
        Some(Item::Definition(defined)) => defined,
        other => panic!("`{written}` is a definition, not {other:?}"),
    }
}

// ---- looking at a term without looking at where it was written ----

/// A term's shape, with the site numbers erased.
///
/// "One raw term up to the origins" is what §3's implementation theorem asks a
/// law to check, and two source texts that write the same music necessarily
/// write it at different offsets. [`musa_core::Origin`] is a site index, so
/// erasing it is a substitution rather than a parse: everything else in the
/// derived rendering — the names, the literals, the spine — is the term.
fn shape(raw: &Raw) -> String {
    erased(&format!("{raw:?}"))
}

/// `shown` with every site number and source offset replaced by one underscore.
fn erased(shown: &str) -> String {
    ["Origin(", "start: ", "end: "]
        .into_iter()
        .fold(shown.to_owned(), |shown, marker| without_numbers_after(&shown, marker))
}

/// `shown` with every run of digits that immediately follows `marker` replaced
/// by one underscore.
///
/// Three markers cover both origins: [`musa_core::Origin`] is a site index and
/// renders as `Origin(7)`, and the compiler's own origin — carried as a
/// `Provenance` payload inside a `sounded` call — renders its spans as
/// `start: 36, end: 40`. Nothing else in the rendering follows those markers, and
/// the digits that *are* the term — an octave, a numerator — follow their own
/// field names and survive.
fn without_numbers_after(shown: &str, marker: &str) -> String {
    let mut erased = String::with_capacity(shown.len());
    let mut rest = shown;
    while let Some(at) = rest.find(marker) {
        let (before, tail) = rest.split_at(at.saturating_add(marker.len()));
        erased.push_str(before);
        erased.push('_');
        rest = tail.trim_start_matches(|written: char| written.is_ascii_digit());
    }
    erased.push_str(rest);
    erased
}

/// A spine's head and its arguments, outermost application last.
fn spine(raw: &Raw) -> (&Raw, Vec<&Raw>) {
    let mut head = raw;
    let mut arguments = Vec::new();
    while let RawShape::App { function, argument, .. } = head.shape() {
        arguments.push(argument);
        head = function;
    }
    arguments.reverse();
    (head, arguments)
}

/// The name a spine's head writes, or `<not a name>` for a law's message.
fn head(raw: &Raw) -> String {
    match spine(raw).0.shape() {
        RawShape::Var(name) => name.to_string(),
        RawShape::Lit(literal) => format!("#{literal}"),
        other => format!("<{other:?}>"),
    }
}

/// Argument `n` of `raw`'s spine, counting from the one written first.
///
/// A law reaches for an argument by position constantly — the fold so far is
/// `follow`'s first, the fact is `sounded`'s third — and a position that is not
/// there is a law failing, not a slice being indexed. Naming the operation is
/// what lets the failure say which argument of which head was missing.
fn argument(raw: &Raw, n: usize) -> &Raw {
    let (_, arguments) = spine(raw);
    arguments
        .get(n)
        .copied()
        .unwrap_or_else(|| panic!("`{}` takes an argument {n}", head(raw)))
}

/// How many times `name` heads an application anywhere in `raw`.
fn counted(raw: &Raw, name: &str) -> usize {
    shape(raw).matches(&format!("Var({name:?})")).count()
}

// ---- the context and the types the core answers in ----

/// The compiler's own context.
fn host() -> Cx {
    crate::registry::owned().expect("the compiler's own context builds")
}

/// `EventTrack ⟨written⟩` — what a complete block inhabits.
fn track() -> Term {
    crate::registry::tagged_type("EventTrack", crate::core::Coordinate::WrittenTime)
}

// ---- the fold ----

/// §2's empty block, and the one place the seed is visible on its own.
#[test]
fn an_empty_block_is_the_track_of_no_occurrences() {
    let cx = host();
    let read = read("music { }");
    assert!(read.constructions().is_empty(), "an empty block constructs no fact");
    let RawShape::Lit(ref written) = *read.term().shape() else {
        panic!("an empty block folds to a literal, not {:?}", read.term().shape());
    };
    assert_eq!(
        *written,
        crate::registry::empty_track(),
        "and the literal is `nothing`, which is what the fold seeds with"
    );
    musa_core::check(&cx, &track(), read.term()).expect("and the block is a track like every other, empty or not");
}

/// The fold itself: one `follow` per statement, over the seed.
#[test]
fn the_fold_writes_one_follow_for_each_statement() {
    for (written, statements) in [
        ("music { }", 0),
        ("music { c4/4 }", 1),
        ("music { c4/4 d4/4 }", 2),
        ("music { c4/4 d4/4 e4/4 }", 3),
    ] {
        let read = read(written);
        assert_eq!(
            counted(read.term(), "follow"),
            statements,
            "`{written}` folds {statements} statement(s) onto the seed"
        );
    }
}

/// A two-statement block is one term the core admits, and the same term the
/// left-nested fold describes.
#[test]
fn two_statements_are_one_follow_the_core_accepts() {
    let cx = host();
    let read = read("music { c4/4 d4/4 }");
    let (head, arguments) = spine(read.term());
    assert!(
        matches!(head.shape(), RawShape::Var(name) if &**name == "follow"),
        "the outer call is a `follow`, not {:?}",
        head.shape()
    );
    assert_eq!(
        arguments.len(),
        2,
        "`follow` takes the fold so far and the next statement"
    );
    assert_eq!(
        counted(argument(read.term(), 0), "follow"),
        1,
        "and the fold so far is itself a `follow`, because the fold is left-nested"
    );
    musa_core::check(&cx, &track(), read.term())
        .unwrap_or_else(|failure| panic!("the core accepts what the fold wrote, not {failure:?}"));
}

/// §2: "`use e;` checks that `e` is a written-time score track" — and the check
/// is the core's, so the reading is the expression and nothing else.
#[test]
fn use_folds_on_the_expression_it_names() {
    let read = read("music { use saved; }");
    assert!(
        read.constructions().is_empty(),
        "naming a track constructs no fact of its own"
    );
    let (_, arguments) = spine(read.term());
    let named = arguments.get(1).expect("`follow` takes the statement second");
    assert!(
        matches!(named.shape(), RawShape::Var(name) if &**name == "saved"),
        "`use saved;` is `saved`, with no call around it: {:?}",
        named.shape()
    );
}

// ---- what a statement says ----

/// A rest sounds one fact, and the fact is the silence rather than a note nobody
/// hears.
#[test]
fn a_rest_is_one_sounded_of_a_rest_fact() {
    let read = read("music { rest 1/4 }");
    let asked = read.outermost();
    assert_eq!(head(asked), "sounded", "a rest is constructed by `sounded`");
    let (_, arguments) = spine(asked);
    assert_eq!(
        arguments.len(),
        4,
        "an origin, a scope, the fact, and how long it is held"
    );
    assert_eq!(
        head(argument(asked, 2)),
        "Fact.Rest",
        "and the fact it sounds is the rest"
    );
}

/// 141j: "`play` takes a `Voicing` and answers a track of several simultaneous
/// `Note` facts, which is what a chord is."
///
/// `stack c4 major/2` is that chord — the one written form that names a chord
/// class, which is what a voicing is a voicing *of*. `[c4 e4 g4]` is a
/// simultaneity of written pitches and names no class, so it cannot reach `play`
/// and does not; the law below it is the other half of the same claim.
#[test]
fn a_stacked_chord_is_one_play() {
    let read = read("music { stack c4 major/2 }");
    let asked = read.outermost();
    assert_eq!(head(asked), "play", "a stacked chord is constructed by `play`");
    let (_, arguments) = spine(asked);
    assert_eq!(
        arguments.len(),
        4,
        "an origin, a scope, the voicing, and how long it sounds"
    );
    assert_eq!(
        counted(read.term(), "Fact.Note"),
        0,
        "and `play` builds the note facts itself, so the reading writes none"
    );
}

/// A bracketed chord is its pitches, each sounding over the same span.
#[test]
fn a_written_simultaneity_is_one_sounded_for_each_pitch() {
    let read = read("music { [c4 e4 g4]/2 }");
    let sounding = read.constructions();
    assert_eq!(sounding.len(), 3, "three pitches, three constructions");
    for asked in &sounding {
        assert_eq!(head(asked), "sounded", "each pitch is its own fact");
        assert_eq!(
            head(argument(asked, 2)),
            "Fact.Note",
            "and each fact is a note, because a written simultaneity spells no chord class"
        );
    }
    assert_eq!(
        counted(read.term(), "together"),
        2,
        "and they sound at once, which is what two `together`s over three tracks say"
    );
}

// ---- the two spellings ----

/// §3's implementation theorem: `transpose up P5 { … }` *is* `transpose` applied
/// to what `{ … }` means on its own.
///
/// Stated over four pairs of programs rather than four comments, and up to the
/// origins, because two source texts write the same music at different offsets.
/// Each pair is one block spelling and the body it encloses read on its own: what
/// the law checks is that the transformation's argument is that body's fold and
/// nothing else, which is the half of the equality this module decides. The other
/// half — that a written `transpose(i, e)` builds the same application — is
/// [`Lowering::application`]'s one line, and `values.rs` is where it is read.
///
/// The whole-program form of this law, `Ok(transpose(M3, music { c4/4 }?)?)`,
/// cannot be written yet: `e?` elaborates to a `match` on `e`, and a `match` in
/// scrutinee position has no inferable type, so a `?` applied to an expression
/// that already drained one is refused by the core. That is the `?` desugaring's
/// limitation rather than this module's, and prompt 142 owns the surface.
#[test]
fn a_transformation_block_is_the_builtin_applied_to_its_body() {
    for (block, body, word) in [
        ("music { transpose up M3 { c4/4 } }", "music { c4/4 }", "transpose"),
        ("music { stretch 2 { c4/4 } }", "music { c4/4 }", "stretch"),
        ("music { invert around c4 { c4/4 } }", "music { c4/4 }", "invert"),
        (
            "music { retrograde { c4/4 d4/4 } }",
            "music { c4/4 d4/4 }",
            "retrograde",
        ),
    ] {
        let enclosing = read(block);
        // All four stand in the fold, as the statement the outer `follow` was
        // given. Before prompt 141m three of them stood behind an answer
        // instead, because three of them could fail.
        let (head, arguments) = spine(argument(enclosing.term(), 1));
        assert!(
            matches!(head.shape(), RawShape::Var(name) if &**name == word),
            "`{block}` calls `{word}`, not {:?}",
            head.shape()
        );
        assert_eq!(
            shape(arguments.last().expect("a transformation takes its track last")),
            shape(read(body).term()),
            "and its track argument is `{body}`'s own fold, up to the origins"
        );
    }
}

/// A block asks nothing, which is what makes it a value.
///
/// The law prompt 141m is for. Every constructor a notation statement reaches
/// for is total, so the reading writes no `?` of its own and drains none: what a
/// block denotes is the fold, and the fold is an `EventTrack ⟨written⟩` rather
/// than a `Result` an author would have to open before using it.
///
/// Stated over the statements that used to ask the most — a note, a chord, and
/// each of the four transformations — because a law that only looked at
/// `retrograde`, which never asked, would have passed before the change too.
#[test]
fn a_block_asks_nothing() {
    let cx = host();
    for written in [
        "music { c4/4 }",
        "music { stack c4 major/2 }",
        "music { transpose up M3 { c4/4 } }",
        "music { stretch 2 { c4/4 } }",
        "music { invert around c4 { c4/4 } }",
        "music { retrograde { c4/4 } }",
    ] {
        let read = read(written);
        assert_eq!(
            counted(read.term(), "Result.Ok"),
            0,
            "`{written}` writes no answer, because nothing in it can fail"
        );
        assert!(
            !matches!(read.term().shape(), RawShape::Match { .. }),
            "`{written}` drains nothing: {:?}",
            read.term().shape()
        );
        musa_core::check(&cx, &track(), read.term())
            .unwrap_or_else(|failure| panic!("`{written}` is a track, not {failure:?}"));
    }
}

// ---- the reading context ----

/// §2: `in scale` "is resolved while pitches are resolved — before any track
/// value exists — so it emits no key signature".
#[test]
fn in_scale_moves_the_pitch_a_step_reads_and_emits_no_fact() {
    let stepped = read("music { in scale c major { (c4 step 2)/4 } }");
    let plain = read("music { e4/4 }");
    assert_eq!(
        shape(stepped.outermost()),
        shape(plain.outermost()),
        "two steps up from `c4` in C major is `e4`, and the two readings are one term"
    );
    assert_eq!(
        counted(stepped.term(), "Fact.Key"),
        0,
        "and putting a collection in force is not a modulation"
    );
    assert_eq!(
        stepped.constructions().len(),
        1,
        "one construction, the note's: `in scale` denotes its body and calls nothing"
    );
}

/// The other side of the same sentence: a step with no collection in force is a
/// diagnostic rather than an implicit C major.
#[test]
fn a_step_with_no_collection_in_force_is_refused_where_it_is_written() {
    let read = read("music { (c4 step 2)/4 }");
    assert!(read.built.is_none(), "the block is not read");
    let complaint = read.complaints.first().expect("one complaint");
    assert_eq!(complaint.code, Code::Misplaced, "a step is misplaced, not unknown");
    assert!(
        complaint.message.contains("needs a scale"),
        "and it says what is missing: {}",
        complaint.message
    );
}

// ---- what a value may not say ----

/// §3: a block "may **not** contain a key, meter, tempo, or clef change" — the
/// four, each refused at its own node, under the code that says *misplaced*.
#[test]
fn a_declaration_with_scope_authority_is_refused_at_its_own_node() {
    for (written, named) in [
        ("music { key c major; c4/4 }", "key"),
        ("music { meter 3/4; c4/4 }", "meter"),
        ("music { tempo quarter = 120; c4/4 }", "tempo"),
        ("music { clef bass; c4/4 }", "clef"),
    ] {
        let read = read(written);
        assert!(read.built.is_none(), "`{written}` is not read");
        let complaint = read
            .complaints
            .first()
            .unwrap_or_else(|| panic!("`{written}` is refused"));
        assert_eq!(
            complaint.code,
            Code::Misplaced,
            "`{named}` is misplaced rather than unsupported: it is a permanent answer"
        );
        assert!(
            complaint.message.contains(named),
            "and the complaint names the statement: {}",
            complaint.message
        );
    }
}

// ---- the two declarations ----

/// §2: a motif is "a named `fn … -> EventTrack[WrittenTime, ScoreFact] { music {
/// body } }`", and a motif with parameters is an ordinary function with ordinary
/// parameters.
#[test]
fn a_motif_is_the_function_it_is_said_to_be() {
    let defined = definition("motif turn(root: Pitch) { c4/4 d4/4 }", SyntaxKind::MotifDecl);
    assert_eq!(&*defined.name, "turn", "it keeps the name it was written with");
    let RawShape::Lam { ref body, .. } = *defined.value.shape() else {
        panic!("a motif with a parameter is a λ, not {:?}", defined.value.shape());
    };
    assert_eq!(
        counted(body, "follow"),
        2,
        "and under the binder is the fold of the two statements it was written with"
    );
    assert!(
        !matches!(body.shape(), RawShape::Match { .. }),
        "and nothing to drain: a motif's body is the fold, not an answer around one"
    );
    let none = definition("motif plain() { c4/4 }", SyntaxKind::MotifDecl);
    assert!(
        !matches!(none.value.shape(), RawShape::Lam { .. }),
        "a motif with no parameters binds nothing, because the grammar gave it nothing to bind"
    );
}

/// §2: a fragment is the `let`.
#[test]
fn a_fragment_is_the_binding_it_is_said_to_be() {
    let defined = definition("fragment answer { c4/4 d4/4 }", SyntaxKind::FragmentDecl);
    assert_eq!(&*defined.name, "answer", "it keeps its name");
    assert!(
        defined.ty.is_none(),
        "and writes no type: what a body of transformations answers is the core's to infer"
    );
    assert!(
        !matches!(defined.value.shape(), RawShape::Match { .. }),
        "and it is the fold itself, which is what lets `use answer;` fold it on directly"
    );
    assert_eq!(
        counted(&defined.value, "follow"),
        2,
        "and the value is the fold of what was written"
    );
}

/// §2: `use e;` "checks that `e` is a written-time score track" — and a saved
/// fragment is one, so the fold takes it directly.
///
/// The sentence a fallible constructor made unwritable. While a block denoted
/// `Result (EventTrack ⟨written⟩) Text`, every fragment was an answer and
/// `follow` demanded a track, so joining two blocks meant a `?` the composer had
/// no reason to write. Stated closed — the fragment's own value bound around the
/// block that uses it — because the claim is that the core accepts the join, and
/// a law with a free variable in it would claim nothing.
#[test]
fn a_saved_fragment_folds_into_another_block_through_use() {
    let cx = host();
    let saved = definition("fragment answer { c4/4 d4/4 }", SyntaxKind::FragmentDecl);
    let read = read("music { use answer; e4/4 }");
    let used = argument(argument(read.term(), 0), 1);
    assert!(
        matches!(used.shape(), RawShape::Var(name) if &**name == "answer"),
        "the fragment is folded on as it stands, not as {:?}",
        used.shape()
    );
    assert_eq!(
        counted(read.term(), "Result.Ok"),
        0,
        "and neither block is an answer somebody has to open"
    );
    let joined = Raw::bind(Origin::UNKNOWN, "answer", saved.value, read.term().clone());
    musa_core::check(&cx, &track(), &joined)
        .unwrap_or_else(|failure| panic!("the core accepts one block used inside another, not {failure:?}"));
}

/// Both declarations lower to something the core admits at the type §2 gives
/// them.
#[test]
fn a_fragment_inhabits_the_track_type_it_was_promised() {
    let cx = host();
    let defined = definition("fragment answer { c4/4 d4/4 }", SyntaxKind::FragmentDecl);
    musa_core::check(&cx, &track(), &defined.value)
        .unwrap_or_else(|failure| panic!("the core accepts a fragment's value, not {failure:?}"));
}
