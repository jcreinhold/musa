//! A computed number and a computed text, written into a quote (prompt 162g).
//!
//! `docs/rules/language/11-quotation.md` §2 refuses `eval`, and the refusal is
//! about a *capability*: no operation reads a text as source. `syntax_numeral`
//! and `syntax_text` do not read anything. Each takes a value and answers with
//! the one expression node that denotes it — one integer token, one string
//! literal — so no grammar is consulted and the argument cannot decide what
//! kind of node comes out.
//!
//! Every law here runs the real thing, as `quotation_laws` does: a probe
//! adapter is handed to the compiler the way any package adapter is, and what
//! the compiler says about the piece that imports it is the law's answer.
//!
//! **The value is observed, not the printed text.** A piece can declare an
//! indexed family whose constructor chooses both indices, which is `Equal` in
//! `stdlib/src/indexed.musa`; a `Refl` built around a spliced literal then
//! type-checks exactly when that literal is convertible with the one the piece
//! wrote by hand. That is the strongest observation available and it is the
//! right one, because the claim is about terms and not about spelling.
//!
//! Four laws:
//!
//! - **Denotation.** The numeral is the number it was given, and a spliced
//!   numeral and a written one are one term. The text is the text it was
//!   given, including the characters that would otherwise end its literal.
//! - **Inversion.** The reader reads the numeral back as the number it was
//!   written from, which is what makes the writing an encoding rather than a
//!   rendering.
//! - **Provenance.** Both stand at a place derived from the `NodePath` they
//!   were given, so two calls at one path are refused by the gate and two
//!   calls at two paths are not — and a numeral does not collide with the
//!   anchor that shares the reservation.
//! - **Category.** Both answer at `Expr` without the parser having run, and
//!   the answer passes the expansion gate.

// A law that does not hold is reported by panicking with what actually
// happened, which is more useful than an assertion message alone.
#![allow(clippy::panic)]

use musa_compiler::{CompileOptions, ImportSources, SourceDocument, compile, resolve_import};
use musa_score::Severity;

/// Where the probe adapter is imported from, as the piece writes it.
const PROBE: &str = "probe::adapter";

/// An adapter whose whole answer is what `emit` builds at the region's root.
///
/// The fold is here for the reason every construction site has one: a
/// `NodePath` is not something an adapter can invent, so the only `here` there
/// is comes from a traversal.
fn adapter(emit: &str) -> String {
    format!(
        "\n    let level = \"readable\";\n{emit}\n    let expand = fn (region: Syntax(TokenTree)) -> \
         Result(Syntax(TokenTree), Pair(Syntax(TokenTree), Text)) {{ Ok(forget(built(region))) }};\n\n    let built = \
         fn (region: Syntax(TokenTree)) -> Syntax(Expr) {{\n        syntax_fold_from_leaves(\n            fn (here) \
         {{ emit(here, []) }},\n            fn (here, kind, text) {{ emit(here, []) }},\n            fn (here, name) \
         {{ emit(here, []) }},\n            fn (here, delimiter, children) {{ emit(here, children) }},\n            \
         region,\n        )\n    }};\n\n"
    )
}

/// A piece with `declarations` above it whose one region is read by the probe.
fn piece(declarations: &str, binding: &str) -> String {
    format!(
        "piece \"probe\" {{\n    import syntax {PROBE} as probe;\n\n{declarations}    let {binding} = syntax probe {{ \
         a }};\n\n    score {{ part p {{ voice v {{ c4/1 }} }} }}\n}}\n"
    )
}

/// Every error compiling that piece against that adapter, with its causes.
fn errors(declarations: &str, binding: &str, module: &str) -> Vec<String> {
    let source = SourceDocument::new(piece(declarations, binding), "probe.musa");
    let mut imports = ImportSources::default();
    imports.insert(resolve_import("probe.musa", PROBE), module.to_owned());
    compile(
        &source,
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    )
    .diagnostics()
    .iter()
    .filter(|diagnostic| diagnostic.severity == Severity::Error)
    .flat_map(|diagnostic| {
        let mut found = vec![whole(
            &diagnostic.message,
            diagnostic.note.as_deref(),
            diagnostic.help.as_deref(),
        )];
        found.extend(
            diagnostic
                .causes
                .iter()
                .map(|cause| whole(&cause.message, cause.note.as_deref(), cause.help.as_deref())),
        );
        found
    })
    .collect()
}

/// One diagnostic, or one cause, as the small document it is.
///
/// All three parts, because a refusal puts the claim in the message and the
/// reason in the note, and a law that read only the first line would be
/// satisfied by any refusal at all.
fn whole(message: &str, note: Option<&str>, help: Option<&str>) -> String {
    let mut lines = vec![message.to_owned()];
    if let Some(note) = note {
        lines.push(format!("note: {note}"));
    }
    if let Some(help) = help {
        lines.push(format!("help: {help}"));
    }
    lines.join("\n")
}

/// `Equal` at `Nat` and at `Text`, as the piece declares them.
///
/// Declared in the piece rather than imported so that each law is one document
/// and reads as one: these are `stdlib/src/indexed.musa`'s `Equal` at two fixed
/// carriers, and nothing about the laws depends on the parameterized version.
const EQUALITIES: &str = "    data SameNat: (left: Nat, right: Nat) -> Type { ReflNat(only: Nat): SameNat(only, only) }\n    data \
                          SameText: (left: Text, right: Text) -> Type { ReflText(only: Text): SameText(only, only) }\n\n";

/// An adapter that answers with `ReflNat` wrapped around a numeral for `value`.
fn numeral_probe(value: &str) -> String {
    adapter(&format!(
        "    let emit = fn (here: NodePath, kids: List(Syntax(Expr))) -> Syntax(Expr) {{ quote at here {{ ReflNat(${{ \
         syntax_numeral(here, {value}) }}) }} }};"
    ))
}

/// The same, for a text.
fn text_probe(value: &str) -> String {
    adapter(&format!(
        "    let emit = fn (here: NodePath, kids: List(Syntax(Expr))) -> Syntax(Expr) {{ quote at here {{ ReflText(${{ \
         syntax_text(here, {value}) }}) }} }};"
    ))
}

#[test]
fn a_numeral_is_the_number_it_was_given() {
    // Five numbers, chosen to cover what the spelling could get wrong: zero,
    // one, a number with two digits, and one past every width a role or a
    // digit count is written in.
    for number in ["0", "1", "12", "4096", "18446744073709551615"] {
        let module = numeral_probe(number);
        let found = errors(EQUALITIES, &format!("proof: SameNat({number}, {number})"), &module);
        assert!(
            found.is_empty(),
            "`syntax_numeral(here, {number})` is not the written `{number}`: {found:?}"
        );
    }

    // The negative control, which is what makes the five above evidence: a
    // proof of the *wrong* equation is refused, so the annotation is reading
    // the value and not merely the shape.
    let refused = errors(EQUALITIES, "proof: SameNat(13, 13)", &numeral_probe("12"));
    assert!(!refused.is_empty(), "twelve proved thirteen equal to itself");
}

#[test]
fn a_text_survives_the_characters_that_would_end_its_literal() {
    // The compiler writes the escape, and this is what that buys. Each of
    // these is a text an adapter could hold and could not spell: a quotation
    // mark ends the literal, a backslash starts an escape, a line feed is
    // outside the lexer's string body altogether, and a `$` is the character a
    // quote body reads as a splice.
    //
    // Both sides of the equation are the *same* text written two ways — the
    // adapter's argument is escaped by the composer here, and the spliced
    // literal is escaped by the compiler — so the law says the two escapings
    // agree.
    for written in [
        r#""plain""#,
        r#""a \" b""#,
        r#""a \\ b""#,
        r#""a \n b""#,
        r#""a $ b""#,
        r#""\"\\\n$""#,
        r#""""#,
    ] {
        let module = text_probe(written);
        let found = errors(EQUALITIES, &format!("proof: SameText({written}, {written})"), &module);
        assert!(
            found.is_empty(),
            "`syntax_text(here, {written})` is not the written {written}: {found:?}"
        );
    }

    let refused = errors(
        EQUALITIES,
        r#"proof: SameText("a b", "a b")"#,
        &text_probe(r#""a \n b""#),
    );
    assert!(
        !refused.is_empty(),
        "a text holding a line feed proved a text holding a space"
    );
}

#[test]
fn the_reader_reads_back_the_number_the_numeral_was_written_from() {
    // §2's inversion, asked inside the adapter because that is where both
    // halves are: `syntax_number` is the reader's own reading, handed back,
    // and the numeral is the writing. Observed as a type, so a round trip that
    // failed answers with a `Text` where the piece annotated a `Nat`.
    let read_back = |written: &str, expected: &str| {
        adapter(&format!(
            "    let emit = fn (here: NodePath, kids: List(Syntax(Expr))) -> Syntax(Expr) {{\n        match \
             syntax_number(forget(syntax_numeral(here, {written}))) {{\n            None -> quote at here {{ \"not a \
             number\" }},\n            Some(read) -> if ratio_equal(read, {expected}) {{ quote at here {{ 1 }} }} \
             else {{ quote at here {{ \"a different number\" }} }},\n        }}\n    }};"
        ))
    };
    for (written, expected) in [("0", "0/1"), ("1", "1/1"), ("12", "12/1"), ("4096", "4096/1")] {
        let found = errors("", "held: Nat", &read_back(written, expected));
        assert!(
            found.is_empty(),
            "`syntax_number` did not read `{written}` back as `{expected}`: {found:?}"
        );
    }

    let refused = errors("", "held: Nat", &read_back("12", "13/1"));
    assert!(!refused.is_empty(), "the reader read twelve back as thirteen");
}

#[test]
fn a_literal_stands_at_a_place_derived_from_the_path_it_was_given() {
    // The place is a function of the arguments (§5.8's D3), so two calls of one
    // builder at one path stand at one place and the gate says so — the same
    // one-call-per-path obligation `11-quotation.md` §5 states of the anchor.
    let twice = adapter(
        "    let emit = fn (here: NodePath, kids: List(Syntax(Expr))) -> Syntax(Expr) { quote at here { (${ \
         syntax_numeral(here, 1) }, ${ syntax_numeral(here, 2) }) } };",
    );
    let refused = errors("", "pair: (Nat, Nat)", &twice);
    assert!(
        refused
            .iter()
            .any(|message| message.contains("not a well-formed expression")),
        "two numerals at one path were not refused: {refused:?}"
    );

    // And two paths are two places. `syntax_built` is how an adapter derives a
    // second one, which is the same answer the anchor's law gives.
    let apart = adapter(
        "    let emit = fn (here: NodePath, kids: List(Syntax(Expr))) -> Syntax(Expr) { quote at here { (${ \
         syntax_numeral(here, 1) }, ${ syntax_numeral(syntax_built(here, 0, 0), 2) }) } };",
    );
    let found = errors("", "pair: (Nat, Nat)", &apart);
    assert!(found.is_empty(), "two numerals at two paths collided: {found:?}");

    // The three builders that share the δ reservation take three positions
    // within it, so a numeral, a text and an anchor about one node do not
    // collide with each other. Without that the reservation would hold one
    // builder and this prompt would have added a second silently.
    let together = adapter(
        "    let emit = fn (here: NodePath, kids: List(Syntax(Expr))) -> Syntax(Expr) { quote at here { (${ \
         syntax_numeral(here, 1) }, ${ syntax_text(here, \"two\") }) } };",
    );
    let found = errors("", "pair: (Nat, Text)", &together);
    assert!(
        found.is_empty(),
        "a numeral and a text about one node collided: {found:?}"
    );
}

#[test]
fn a_literal_is_an_expression_without_the_parser_having_run() {
    // Both answer at `Expr` (§2 fixes construction there), so neither needs
    // `as_expression` and both may stand where an expression stands. The
    // adapter's whole answer is the literal itself, which is how the expansion
    // gate sees it: `check_expression` runs over what `expand` returns.
    let numeral = adapter(
        "    let emit = fn (here: NodePath, kids: List(Syntax(Expr))) -> Syntax(Expr) { syntax_numeral(here, 7) };",
    );
    let found = errors("", "held: Nat", &numeral);
    assert!(found.is_empty(), "a bare numeral is not an expression: {found:?}");

    let text = adapter(
        "    let emit = fn (here: NodePath, kids: List(Syntax(Expr))) -> Syntax(Expr) { syntax_text(here, \"said\") \
         };",
    );
    let found = errors("", "held: Text", &text);
    assert!(found.is_empty(), "a bare text literal is not an expression: {found:?}");

    // And the categories are not interchangeable: the answer is a `Text`, so a
    // piece that annotated a `Nat` is refused. Without this the two laws above
    // would pass on an adapter that answered with anything at all.
    let refused = errors("", "held: Nat", &text);
    assert!(!refused.is_empty(), "a text literal stood where a number was annotated");
}
