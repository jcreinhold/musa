//! What elaborating a document promises.
//!
//! Beside the driver rather than in `tests/suite/`, for [`crate::lower::laws`]'s
//! reason: the context is [`crate::registry::owned`] and the answer is a
//! [`musa_calculus::Term`], and both are private to this crate.
//!
//! # The survey
//!
//! Half of this module is [`the_standard_library_elaborates`] and its two
//! adapter laws, and they are the point of the prompt rather than a check on it.
//! Elaborating `stdlib/` through the new checker and recording exactly what it
//! still refuses is what tells prompt 142 which of its failures will be
//! migrations and which would be defects: every entry below is a spelling 142's
//! Target already owns, and an entry that appeared without one would be a defect
//! in one of the thirteen prompts that built the reading.
//!
//! The recorded lists are exact, the way `registry::rules::UNREGISTERED` is
//! exact. A corpus file that starts failing for a new reason fails the build; a
//! reason that stops applying has to be struck from the list in the commit that
//! fixed it, which is what keeps the survey a record rather than a wish.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a law that cannot fail loudly is not a law"
)]

use musa_syntax::{SyntaxKind, SyntaxNode};

use super::{Document, Source, elaborate};
use crate::resolve::Resolver;

// ---- reading a written document back out of a parse ----

/// The `library { … }` node `source` writes, with a loud failure when it does
/// not parse.
fn library(source: &str) -> SyntaxNode {
    let document = musa_syntax::parse(source);
    assert!(
        document.errors().is_empty(),
        "the law's own source parses: {:?}",
        document.errors()
    );
    written_library(&document.syntax()).expect("the source writes a library")
}

/// The first `library` under `root`, when there is one.
fn written_library(root: &SyntaxNode) -> Option<SyntaxNode> {
    root.descendants().find(|node| node.kind() == SyntaxKind::LibraryDecl)
}

/// `source` elaborated on its own, and every distinct fault, sorted.
///
/// The fault is the code and the message and never the span, because a law that
/// pinned spans would fail on a blank line and the span is
/// [`crate::lower::refusals`]'s to get right.
fn elaborated(source: &str) -> (Option<Document>, Vec<String>) {
    faults(&[Source::own(&library(source))])
}

/// The same for a document already assembled out of several sources.
fn faults(sources: &[Source]) -> (Option<Document>, Vec<String>) {
    let mut resolver = Resolver::new();
    let document = elaborate(&mut resolver, sources, None);
    let mut said: Vec<String> = resolver
        .diagnostics
        .iter()
        .map(|complaint| format!("{:?}: {}", complaint.code, complaint.message))
        .collect();
    said.sort();
    said.dedup();
    (document, said)
}

/// `source`, which every law that expects success reads its answer out of.
fn document(source: &str) -> Document {
    let (document, said) = elaborated(source);
    document.unwrap_or_else(|| panic!("the document elaborates: {said:?}"))
}

// ---- the walk ----

#[test]
fn ordinary_definitions_elaborate_and_read_back() {
    let document = document(
        "library {
            fn twice(n: Nat) -> Nat { Succ(Succ(n)) }
            let three: Nat = twice(1);
            let also: Nat = 3;
        }",
    );
    assert_eq!(
        document.names().iter().map(|name| &**name).collect::<Vec<_>>(),
        ["twice", "three", "also"],
        "the names are the ones the document wrote, in written order"
    );
    let (computed, _) = document.value("three").expect("`three` is bound");
    let (written, _) = document.value("also").expect("`also` is bound");
    assert_eq!(
        computed, written,
        "a definition reads back as the normal form its body computes"
    );
}

#[test]
fn a_body_may_name_a_definition_written_after_it() {
    let document = document(
        "library {
            let answer: Nat = later(1);
            fn later(n: Nat) -> Nat { Succ(n) }
            let written: Nat = 2;
        }",
    );
    let (answer, _) = document.value("answer").expect("`answer` is bound");
    let (written, _) = document.value("written").expect("`written` is bound");
    assert_eq!(answer, written, "and it means what it names");
}

#[test]
fn a_data_declaration_may_be_written_after_the_one_that_names_it() {
    let document = document(
        "library {
            data Held { Wrap(inner: Inner) }
            data Inner { Only }
            let held: Held = Wrap(Only);
        }",
    );
    let (value, _) = document.value("held").expect("`held` is bound");
    let shown = format!("{value:?}");
    assert!(shown.contains("\"Wrap\""), "the outer constructor: {shown}");
    assert!(shown.contains("\"Only\""), "and the inner one: {shown}");
}

#[test]
fn two_data_declarations_that_name_each_other_are_refused() {
    let (document, said) = elaborated(
        "library {
            data Left { Wrap(inner: Right) }
            data Right { Wrap(inner: Left) }
        }",
    );
    assert!(document.is_none(), "the cycle is refused");
    assert_eq!(
        said,
        ["DependencyCycle: `Left`, `Right` name each other"],
        "and the refusal names both declarations"
    );
}

#[test]
fn the_phase_vocabulary_is_readable_only_in_a_phase_source() {
    const WRITTEN: &str = "library { fn probe(held: Syntax<Expr>) -> Syntax<Expr> { held } }";
    let (ordinary, said) = elaborated(WRITTEN);
    assert!(ordinary.is_none(), "ordinary source cannot name `Syntax`: {said:?}");
    let (phase, said) = faults(&[Source::own(&library(WRITTEN)).in_phase()]);
    assert!(phase.is_some(), "an adapter phase can: {said:?}");
}

/// A structured answer reads back as the data it is.
///
/// The compiler's half of prompt 141q, and the reason that prompt exists:
/// [`crate::registry::read_back`] answers a *base literal*, and a count and a
/// list are neither. `3` is a numeral at the counting family `Nat`, `[1, 2]` is
/// `List.Cons` over `List.Empty`, and `read_back` refuses both.
///
/// Stated over source rather than over hand-written terms because that is the
/// claim worth making. The shapes below are what `let n: Nat = 3;` *actually*
/// elaborates to through the prelude's declarations and 141g's lowering, so a
/// change to either that broke a consumer's reading would fail here rather than
/// in whatever pass first tried to count something.
///
/// The parameter is what `List` adds over `Nat`. A value of `List Nat` is
/// `List.Cons Nat 1 (…)`, and every field below is a field: the reading drops
/// the parameter because the declaration says where the fields begin.
#[test]
fn a_count_and_a_list_read_back_as_canonical_data() {
    let document = document(
        "library {
            let n: Nat = 3;
            let xs: List<Nat> = [1, 2];
        }",
    );
    let read = |name: &str| {
        let (normal, _) = document.value(name).expect("the definition is bound");
        musa_calculus::canonical(&normal).unwrap_or_else(|| panic!("`{name}` reads back as data"))
    };
    let case = |name: &str, fields: Vec<musa_calculus::Datum>| musa_calculus::Datum::Case {
        constructor: std::sync::Arc::from(name),
        fields,
    };
    let whole = |count| musa_calculus::Datum::Count {
        family: std::sync::Arc::from("Nat"),
        count,
    };
    assert_eq!(read("n"), whole(3), "a count reads back as the number it is");
    assert_eq!(
        read("xs"),
        case(
            "List.Cons",
            vec![
                whole(1),
                case("List.Cons", vec![whole(2), case("List.Empty", Vec::new())]),
            ],
        ),
        "and a list is its members, with the element type left out"
    );
}

/// Every container reachable from a piece that imports nothing folds, and the
/// three derived methods come with the two written ones.
///
/// This is the collection half of prompt 141 arriving at the surface, and the
/// claim is deliberately made over *source*: `01-surface.md` §1.6 says a
/// container earns `map`, `filter`, and `collect` by writing `fold_from_start`
/// and `fold_from_end`, and the prelude's two impls write exactly two methods
/// each. If the derived bodies were not inherited, `xs.collect()` below would
/// fail to resolve rather than answer.
///
/// # Why over the prelude and not over `stdlib/`
///
/// `List` and `Option` are prelude families and their instances are prelude
/// instances, so the source here writes no `import`. A law that imported would
/// be checking the module system, which
/// [`the_standard_library_elaborates`] already does; what this one checks is
/// that a piece which imports nothing can still fold, which is what makes the
/// traits reachable from an adapter and from `examples/named-answer.musa`
/// alike.
///
/// # The directions
///
/// `rebuilt` and `gathered` are the pair that decides the builder is right.
/// `fold_from_end` with `Cons` is the identity on lists, and `collect` is
/// `fold_from_start` with a snoc `push`; they have to answer the same list or
/// the builder is reversing, which is note 41 §7's whole complaint.
///
/// # `nothing` is bound and `Some(5)` is not
///
/// Deliberate, and the difference is `10-traits.md` §6. A method resolves by
/// *exact receiver*: `Some(5)` says `Option` and `None` on its own does not,
/// because its parameter is the only thing that fixes the type and nothing here
/// fixes it. So `None.fold_from_end(…)` is refused by design and an author
/// writes the annotation once at the binding — which is what the corpus does
/// anyway, since the empty case usually arrives in a variable rather than
/// spelled at the call.
#[test]
fn a_prelude_container_folds_and_earns_its_derived_methods() {
    let document = document(
        "library {
            let length: Nat = [1, 2, 3].fold_from_start(0, fn (built: Nat, item: Nat) -> Nat { Succ(built) });
            let rebuilt: List<Nat> =
                [1, 2].fold_from_end([], fn (item: Nat, later: List<Nat>) -> List<Nat> { Cons(item, later) });
            let gathered: List<Nat> = [1, 2].collect();
            let doubled: List<Bool> = [1, 2].map(fn (item: Nat) -> Bool { True });
            let kept: List<Nat> = [0, 1, 2].filter(fn (item: Nat) -> Bool {
                match item { Zero -> False, Succ(fewer) -> True }
            });
            let held: Nat = Some(5).fold_from_end(0, fn (found: Nat, fallback: Nat) -> Nat { found });
            let nothing: Option<Nat> = None;
            let missing: Nat = nothing.fold_from_end(7, fn (found: Nat, fallback: Nat) -> Nat { found });
        }",
    );
    let read = |name: &str| {
        let (normal, _) = document.value(name).expect("the definition is bound");
        musa_calculus::canonical(&normal).unwrap_or_else(|| panic!("`{name}` reads back as data"))
    };
    let whole = |count| musa_calculus::Datum::Count {
        family: std::sync::Arc::from("Nat"),
        count,
    };
    let case = |name: &str, fields: Vec<musa_calculus::Datum>| musa_calculus::Datum::Case {
        constructor: std::sync::Arc::from(name),
        fields,
    };
    let listed = |items: Vec<musa_calculus::Datum>| {
        items
            .into_iter()
            .rev()
            .fold(case("List.Empty", Vec::new()), |rest, item| {
                case("List.Cons", vec![item, rest])
            })
    };

    assert_eq!(read("length"), whole(3), "a forward fold visits every member once");
    assert_eq!(
        read("rebuilt"),
        listed(vec![whole(1), whole(2)]),
        "and the catamorphism with `Cons` is the identity"
    );
    assert_eq!(
        read("gathered"),
        listed(vec![whole(1), whole(2)]),
        "so `collect` — a forward fold and a snoc — must answer the same, not the reverse"
    );
    assert_eq!(
        read("doubled"),
        listed(vec![case("Bool.True", Vec::new()), case("Bool.True", Vec::new())]),
        "`map` builds at the type its answer is checked against"
    );
    assert_eq!(
        read("kept"),
        listed(vec![whole(1), whole(2)]),
        "and `filter` keeps what the predicate admits, in order"
    );
    assert_eq!(read("held"), whole(5), "an `Option` fold reaches the held value");
    assert_eq!(read("missing"), whole(7), "and answers the seed when there is none");
}

/// The two questions a [`Document`] answers are one interface, not two.
///
/// [`Document::value`] hands back an [`musa_calculus::ElabError`] and nothing else
/// about place, so the only way a caller turns one into a diagnostic is through
/// the site table the same document holds — which is the arrangement
/// `02-core-calculus.md` §7 asks for, with the core a leaf that never learns
/// what a file is. A name nobody wrote has no span, and the restatement says so
/// rather than pointing at node one.
#[test]
fn a_refusal_about_a_name_is_restated_through_the_documents_own_sites() {
    let document = document("library { let held: Nat = 1; }");
    let error = document.value("absent").expect_err("`absent` is not bound");
    let restated = crate::lower::refusals::restate(document.sites(), &error);
    assert_eq!(restated.code, musa_score::diagnose::Code::UnknownName);
    assert!(
        restated.labels.is_empty(),
        "a name the reading never numbered has nowhere to point: {:?}",
        restated.labels
    );
}

// ---- the survey ----

/// [`STANDARD_LIBRARY`], parsed, as the sources an importing document sees.
///
/// `pub(crate)` for one caller outside this module: prompt 141p's example survey
/// in [`crate::lower::piece::laws`] reads the same corpus, because an example
/// that writes `use theory::harmony;` sees exactly these and surveying it
/// without them would report missing names that are not missing. One list rather
/// than two, so a library added to `stdlib/` cannot appear in one survey and not
/// the other.
///
/// [`Source::imported`] and not [`Source::own`], which is the difference between
/// surveying the corpus and inventing a document nobody writes. A library is a
/// *file the piece imported*, and [`crate::document::elaborate`] asks which of
/// the two a source is for a reason: this document's own declarations are the
/// ones a reader sees at once and must not repeat, and an import's are the ones
/// compared against other imports. Handed over as own sources, seventeen files'
/// worth of names became the example's own, so `examples/anonymous-functions`
/// declaring `let counted` beside `nat.musa`'s `private fn counted` was reported
/// as a name written twice in one file — a collision of the harness's making,
/// which the compiler does not report on that file and `01-surface.md` §1 has
/// not asked anyone to report.
///
/// The path is the file's, because it is what the collision between two imports
/// is *named* by. The span is not: there is no `import` statement here to point
/// at, and a survey has no text of its own for one to be in.
pub(crate) fn library_sources() -> Vec<Source> {
    STANDARD_LIBRARY
        .iter()
        .map(|&(name, source)| {
            let held = musa_syntax::parse(source);
            assert!(held.errors().is_empty(), "`{name}` parses: {:?}", held.errors());
            let path = format!("stdlib/src/{name}.musa");
            Source::imported(
                &written_library(&held.syntax()).expect("every standard library file writes a library"),
                crate::imports::Imported {
                    path: &path,
                    qualifier: None,
                    at: musa_score::origin::SourceSpan::new(0, 0),
                },
            )
        })
        .collect()
}

/// Every library `stdlib/` writes that is not an adapter, in import order.
///
/// One document rather than fourteen, because that is what an importing file
/// sees: `triad` is written in `harmony.musa` and read in several others, and
/// surveying each file alone would report names as missing that are not
/// missing at all.
const STANDARD_LIBRARY: &[(&str, &str)] = &[
    ("core", include_str!("../../../../stdlib/src/core.musa")),
    ("algebra", include_str!("../../../../stdlib/src/algebra.musa")),
    ("collections", include_str!("../../../../stdlib/src/collections.musa")),
    ("list", include_str!("../../../../stdlib/src/list.musa")),
    ("pitch", include_str!("../../../../stdlib/src/pitch.musa")),
    ("scale", include_str!("../../../../stdlib/src/scale.musa")),
    ("harmony", include_str!("../../../../stdlib/src/harmony.musa")),
    ("voicing", include_str!("../../../../stdlib/src/voicing.musa")),
    ("context", include_str!("../../../../stdlib/src/context.musa")),
    (
        "transformational",
        include_str!("../../../../stdlib/src/transformational.musa"),
    ),
    (
        "post_tonal/pcset",
        include_str!("../../../../stdlib/src/post_tonal/pcset.musa"),
    ),
    (
        "post_tonal/serial",
        include_str!("../../../../stdlib/src/post_tonal/serial.musa"),
    ),
    (
        "tonal/harmony",
        include_str!("../../../../stdlib/src/tonal/harmony.musa"),
    ),
    (
        "tonal/sequences",
        include_str!("../../../../stdlib/src/tonal/sequences.musa"),
    ),
    (
        "tonal/schemas",
        include_str!("../../../../stdlib/src/tonal/schemas.musa"),
    ),
    (
        "notation/staff",
        include_str!("../../../../stdlib/src/notation/staff.musa"),
    ),
];

/// The whole standard library, elaborated as one document, with nothing left
/// over.
///
/// Two faults stood here while 142 was in progress and both are now migrated
/// rather than pending. **`Music`**, the contextual type this prompt deletes,
/// was written in a `list.musa` and a `voicing.musa` signature; a fragment is an
/// `EventTrack ⟨written⟩` and a motif is a function to one, which is what
/// `00-semantics.md` §3 already said. The **anonymous product** was the second,
/// until 142 found that only *half* of it was missing: `(a, b)` had lowered here
/// since 141g and it was the type `(A, B)` alone that refused, which is one
/// construct disagreeing with itself rather than a stage. Both halves now read
/// `Pair`, the family 141ha declared for the machine calculus's wiring — a
/// written product is a constructor application and therefore canonical data,
/// which a structural record is not.
///
/// Everything the thirteen prompts built holds on the standard library's real
/// Musa: every `data` declaration, every generic signature, the qualified paths
/// 141l reads, the notation vocabulary 141j and 141k registered, and the eight
/// collection eliminators this prompt wrote as library code — which is why
/// `nat.musa` is a row in [`STANDARD_LIBRARY`] and not only in `lib.musa`.
#[test]
fn the_standard_library_elaborates() {
    let (_, said) = faults(&library_sources());
    assert!(said.is_empty(), "the standard library elaborates whole: {said:?}");
}

/// `stdlib/src/adapters/doubled.musa`, elaborated in phase scope.
///
/// Whole, with nothing left over. What it took was the annotation on `expand`
/// and on `edit`: the replaced checker was told each operation's type by
/// [`crate::phase::read_adapter_module`] and solved the rest, and a bidirectional
/// reading settles a `fn` where the `fn` stands. So the phase's interface is now
/// written in the two files that implement it rather than held in a table beside
/// them — the same move [`crate::phase::Printer`] already argued for `print`, and
/// the repair `01-surface.md` §1 asks for anywhere else a declaration is left
/// open.
#[test]
fn the_doubled_adapter_elaborates() {
    let said = adapter(include_str!("../../../../stdlib/src/adapters/doubled.musa"));
    assert!(said.is_empty(), "the doubled adapter elaborates whole: {said:?}");
}

/// `stdlib/src/adapters/staff.musa`, elaborated in phase scope, with the one
/// module it imports.
///
/// Whole, and the import is why the list is not empty. §5.9's phase "adds three
/// things and takes nothing away", so an adapter reads `std::list` like any
/// other document; `map`, `filter`, and `range` are ordinary source now that
/// `Iterable` carries the folds, and a survey that withheld the module would
/// report three names as missing that the phase finds.
///
/// This file is prompt 165's benchmark and 142's Stop forbids rewriting it, so
/// what it took was a *migration*: the two `::` paths a type namespace now
/// wants, the three products that had to become records for `with` to reach
/// them, and the fold call sites the trait rewrote.
#[test]
fn the_staff_adapter_elaborates() {
    let said = adapter(include_str!("../../../../stdlib/src/adapters/staff.musa"));
    assert!(said.is_empty(), "the staff adapter elaborates whole: {said:?}");
}

/// One adapter, read the way the expansion phase reads one.
///
/// Through [`crate::phase::read_adapter_module`] and not through a document
/// assembled here, because two details of §5.9's scope are the phase's and
/// cannot be guessed from the file: the modules an `import` brings in, and the
/// declarations `print` takes with it. `print` is read where it *runs* — with
/// the notation module in scope — so a survey that elaborated it with the rest
/// would report the notation's own type names as missing. Reusing the phase's
/// entry is what keeps this a law about the adapter rather than about the
/// harness.
fn adapter(source: &str) -> Vec<String> {
    match crate::phase::read_adapter_module(source, crate::phase::PhaseImports::bundled()) {
        Ok(_) => Vec::new(),
        Err(crate::phase::ModuleFault::Broken(said)) => {
            let mut said: Vec<String> = said
                .iter()
                .map(|complaint| format!("{:?}: {}", complaint.code, complaint.message))
                .collect();
            said.sort();
            said.dedup();
            said
        }
        Err(crate::phase::ModuleFault::Stopped) => vec!["Stopped: a compilation limit was crossed".to_owned()],
    }
}

/// A written `primitive("name", version, c)` is the registration that pair
/// selects, and an unregistered pair is refused where it is written.
///
/// `03-machine-calculus.md` §1 lets the name and version decide the step, both
/// ports, and the type of the configuration, so there is nothing for a signature
/// to say and nothing for the elaborator to solve: the reading selects, and what
/// it selects is a closed type. The negative half is the whole of the check —
/// with no signature for an unregistered pair, "this build knows no such unit"
/// is a refusal at the call rather than an unsolved metavariable somewhere
/// downstream.
#[test]
fn a_written_unit_selects_the_signature_its_name_and_version_name() {
    let (built, said) = faults(&[Source::own(&library(
        "library { let gain = machine(primitive(\"scale\", 1, 3/2)); }",
    ))]);
    let built = built.unwrap_or_else(|| panic!("a registered unit elaborates: {said:?}"));
    let (_, ty) = built.value("gain").expect("`gain` is bound");
    assert!(
        format!("{ty:?}").contains("Machine"),
        "a unit wrapped in `machine` is a machine, and its type says so: {ty:?}"
    );

    for (written, expected) in [
        (
            "machine(primitive(\"gian\", 1, 3/2))",
            "not a unit this build registers",
        ),
        ("machine(primitive(\"scale\", 99, 3/2))", "no version 99"),
    ] {
        let (refused, said) = faults(&[Source::own(&library(&format!("library {{ let gain = {written}; }}")))]);
        assert!(refused.is_none(), "`{written}` is refused");
        assert!(
            said.iter().any(|complaint| complaint.contains(expected)),
            "`{written}` says why: {said:?}"
        );
    }
}

/// A machine reads back as the description a consumer prepares, and a machine
/// whose ports nothing decided does not read back at all.
///
/// `03-machine-calculus.md` §2 gives the eight forms no reductions, so what
/// comes back is the description the declaration built rather than anything the
/// value computed: the nodes are the forms that were written, children before
/// parents, and the ports are the *unit's* — nothing in the source below spells
/// `Ratio` and both ends of the projection say it anyway.
#[test]
fn a_machine_reads_back_as_its_description_once_its_ports_are_decided() {
    let built = document(
        "library {
            let one = machine(primitive(\"scale\", 1, 3/2));
            let chained = connect(machine(primitive(\"scale\", 1, 3/2)), identity);
            let counted: Nat = 3;
        }",
    );
    let machines = built.machines();
    assert_eq!(
        machines.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(),
        ["one", "chained"],
        "a definition that is not a machine is not one, and costs no readback"
    );
    let (_, one) = machines.first().expect("`one` is a machine");
    assert_eq!(
        (one.step(), one.input(), one.output()),
        ("AudioFrameStep", "Ratio", "Ratio")
    );
    assert_eq!(one.nodes().len(), 1, "`machine(p)` describes the unit inside it");
    assert_eq!(
        one.nodes().first().and_then(musa_score::machine::SpecNode::id),
        Some("scale")
    );

    let (_, chained) = machines.get(1).expect("`chained` is a machine");
    assert_eq!(chained.nodes().len(), 3, "the unit, the identity, and the join");
    let root = chained.root().expect("a projection has a root");
    for child in chained
        .nodes()
        .get(root)
        .map(musa_score::machine::SpecNode::children)
        .unwrap_or_default()
    {
        assert!(*child < root, "a node's children precede it");
    }
}

/// `identity` is a machine at every step and every port. Written alone it
/// binds the polymorphic value — the ports are quantified at the declaration
/// and solved at the use (`02-core-calculus.md` §2.1), and nothing about the
/// `let` asks for them. The refusal arrives where a port is *needed* and
/// still not written: a use that leaves them undetermined. Written with its
/// type it is a machine like any other.
#[test]
fn an_open_machine_is_refused_until_its_ports_are_written() {
    let (open, said) = elaborated("library { let open = identity; }");
    assert!(
        open.is_some(),
        "the polymorphic value binds: nothing was asked of it yet: {said:?}"
    );

    let (used, said) = elaborated("library { let open = identity; let joined = connect(open, open); }");
    assert!(
        used.is_none(),
        "a machine whose ports nothing determines connects to nothing"
    );
    assert!(
        said.iter().any(|complaint| complaint.contains("could not determine")),
        "the refusal names what was not determined: {said:?}"
    );

    let decided = document("library { let decided: Machine<AudioFrameStep, Ratio, Ratio> = identity; }");
    let machines = decided.machines();
    let (name, described) = machines.first().expect("a written type decides the ports");
    assert_eq!(name, "decided");
    assert_eq!((described.input(), described.output()), ("Ratio", "Ratio"));
    assert_eq!(
        described.nodes().first().map(musa_score::machine::SpecNode::form),
        Some(musa_score::machine::SpecForm::Identity)
    );
}
