//! What makes two calls one body, and what a shared body costs.
//!
//! The sharing table is an internal transformation of the piece's term, not a
//! cache, so its laws are stated where a composer can see them: in the term
//! the compiler prints, and in the budget it charges. Two calls that denote
//! the same music must bind one body; two calls that denote different music
//! must not; and neither answer may change which pieces the compiler accepts.

#![allow(clippy::expect_used)]

use std::fmt::Write as _;

use musa_compiler::{Code, CompileOptions, Realization, SourceDocument, compile, kernel_text};

/// How many bodies a piece's term binds.
///
/// Counted from the printed term because that is the compiler's own account
/// of what it built — a body elaborated twice is two `let shared` bindings,
/// whatever the timings say.
fn bodies(source: &str) -> usize {
    kernel_text(
        &SourceDocument::new(source, "sharing-laws.musa"),
        &Realization::deterministic(),
    )
    .expect("the piece elaborates")
    .matches("let shared")
    .count()
}

/// A piece whose one voice holds `body`, with `declarations` above the score.
fn piece(declarations: &str, body: &str) -> String {
    format!(
        "piece \"Sharing\" {{\n    meter 4/4;\n    key c major;\n{declarations}\n    score {{ part p {{ voice v {{\n{body}\n    }} }} }}\n}}\n"
    )
}

/// The occurrence count a refused piece says it attempted, with the operation
/// that was charged when it crossed.
///
/// `None` when the piece is accepted. The meter's own words are the observable
/// here: `docs/rules/language/06-performance.md` fixes that a rejection names the
/// operation, metric, attempted count, and limit.
fn attempted(source: &str) -> Option<(String, u64)> {
    let compilation = compile(
        &SourceDocument::new(source, "sharing-laws.musa"),
        &CompileOptions::default(),
    );
    compilation
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code == Code::ResourceLimit)
        .map(|diagnostic| {
            let label = diagnostic
                .labels
                .first()
                .expect("a resource diagnostic labels the count")
                .text
                .clone();
            let count = label
                .split_whitespace()
                .nth(1)
                .and_then(|word| word.parse().ok())
                .expect("a resource label reads `attempted {n} {metric}`");
            (diagnostic.message.clone(), count)
        })
}

/// `calls` uses of a motif whose body is `body` sixteenth notes.
fn repeated_cell(body: usize, calls: usize) -> String {
    piece(
        &format!("    motif cell() {{ repeat {body} {{ c5/16 }} }}"),
        &"        use cell();\n".repeat(calls),
    )
}

#[test]
fn identical_calls_at_distinct_sites_elaborate_one_body() {
    // The call-site gap, closed. Four `use`s of one motif denote one piece of
    // music four times over; where they are written is carried at the
    // reference, in its mark, and is not a property of the body.
    assert_eq!(
        bodies(&piece(
            "    motif cell() { c5/4 d5/4 e5/4 f5/4 }",
            &"        use cell();\n".repeat(4)
        )),
        1
    );
}

#[test]
fn a_parameterized_body_is_one_binding_per_distinct_argument() {
    // Eight calls, two arguments, two bodies: what a body costs is set by the
    // arguments it is given, not by how often it is asked for. This is the
    // measured shape of the full-laziness gap — the residual duplication is
    // bounded by the piece's written vocabulary rather than by its length.
    let calls = ["c5", "d5"]
        .iter()
        .cycle()
        .take(8)
        .fold(String::new(), |mut calls, root| {
            let _ = writeln!(calls, "        use cell({root});");
            calls
        });
    assert_eq!(
        bodies(&piece("    motif cell(root: Pitch) { root/4 g5/4 a5/4 b5/4 }", &calls)),
        2
    );
}

#[test]
fn one_body_read_under_two_scales_is_two_bodies() {
    // Why the call site could go: what it was standing in for is the pitch
    // context in force at the call. A degree resolves against the innermost
    // `in scale`, so the same motif under C major and C dorian is two pieces
    // of music and must be two bindings.
    // `scale_context_laws.rs` fixes the pitches this distinction produces;
    // this fixes that the sharing key is what keeps them apart.
    assert_eq!(
        bodies(&piece(
            "    motif cell() { (c5 step 0)/4 (c5 step 1)/4 (c5 step 2)/4 (c5 step 3)/4 }",
            "        in scale c major { use cell(); use cell(); }\n        \
             in scale c dorian { use cell(); use cell(); }"
        )),
        2
    );
}

#[test]
fn one_more_call_of_a_shared_body_charges_one_more_body() {
    // Sharing is a fact about the compiler; the budget is a fact about the
    // program. A body the compiler elaborated once and referenced three
    // hundred times must be charged three hundred times, or a piece would be
    // accepted for being written in a way the compiler happens to like.
    //
    // Stated as a difference rather than a total so it fixes the *rate* and
    // says nothing about the piece's fixed overhead. A meter that charged the
    // sharing table instead of the music would report the same crossing point
    // for both call counts, and the difference would be zero.
    for body in [2000_u64, 4000] {
        let calls = usize::try_from(1_000_000 / body).expect("a call count fits") / 2 * 3;
        let (operation, fewer) = attempted(&repeated_cell(usize::try_from(body).expect("a body size fits"), calls))
            .expect("this many calls exceed the budget");
        let (_, more) = attempted(&repeated_cell(
            usize::try_from(body).expect("a body size fits"),
            calls + 1,
        ))
        .expect("one more call still exceeds it");
        assert_eq!(more - fewer, body + 1, "one call is one body and its segment");
        assert!(
            operation.contains("elaborating the piece timeline"),
            "the score is charged once, at the boundary, not once per call: {operation}"
        );
    }
}

#[test]
fn a_shared_body_carries_no_call_site_and_each_reference_carries_its_own() {
    // Provenance survives sharing by moving: the body is printed once with a
    // placeholder where the call would be, and every reference states its own
    // site in its mark (`docs/rules/kernel/10-term-calculus.md` T6).
    let printed = kernel_text(
        &SourceDocument::new(
            piece(
                "    motif cell() { c5/4 d5/4 e5/4 f5/4 }",
                &"        use cell();\n".repeat(3),
            ),
            "sharing-laws.musa",
        ),
        &Realization::deterministic(),
    )
    .expect("the piece elaborates");
    let sites: std::collections::BTreeSet<&str> = printed
        .lines()
        .filter_map(|line| line.split("via motif ").nth(1))
        .map(|site| site.trim_end().trim_end_matches('"'))
        .collect();
    assert_eq!(sites.len(), 3, "three references, three call sites:\n{printed}");
}
