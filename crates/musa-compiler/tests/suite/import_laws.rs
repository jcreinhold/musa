//! What an import is, and what it is not (roadmap §16).
//!
//! Imports are the one place a musa compilation reads something the author of
//! the piece did not write, so the rules are worth pinning: paths join
//! lexically, a library is not a piece, names are flat and collisions are
//! errors, a cycle is reported rather than followed, and — the property the
//! rest depend on — an imported motif produces exactly the notes it would
//! have produced written in place.

// Test helpers use expect() on statically-valid inputs: a failure is a bug in
// the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use musa_compiler::{Compilation, CompileOptions, ImportSources, SourceDocument, compile, resolve_import};

use musa_score::ScoreSnapshot;

/// Compile `source` as `name`, with `files` available to import.
fn compile_with(name: &str, source: &str, files: &[(&str, &str)]) -> Compilation {
    let mut imports = ImportSources::default();
    for (path, text) in files {
        imports.insert(*path, *text);
    }
    compile(
        &SourceDocument::new(source, name),
        &CompileOptions {
            imports,
            ..CompileOptions::default()
        },
    )
}

/// Each error as the whole small document it is: the claim, then the rule and
/// the advice under it. What a reader is told is the sum of the three, so a
/// test that reads only the first line tests less than it looks like it does.
fn errors(compilation: &Compilation) -> Vec<String> {
    compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_score::Severity::Error)
        .map(|diagnostic| {
            let mut lines = vec![diagnostic.message.clone()];
            if let Some(note) = diagnostic.note.as_deref() {
                lines.push(format!("note: {note}"));
            }
            if let Some(help) = diagnostic.help.as_deref() {
                lines.push(format!("help: {help}"));
            }
            lines.join("\n")
        })
        .collect()
}

fn snapshot(compilation: Compilation) -> ScoreSnapshot {
    let messages = errors(&compilation);
    compilation
        .into_snapshot()
        .unwrap_or_else(|| panic!("expected a snapshot; errors: {messages:?}"))
}

const MOTIFS: &str = " motif rise() { c5/4 d5/4 e5/4 g5/4 } ";

fn piece(body: &str) -> String {
    format!(
        "piece \"P\" {{ {body} tempo 1/4 = 60; meter 4/4; key c major; score {{ part p {{ voice v {{ use rise(); }} }} }} }}"
    )
}

/// A path is joined onto the importer's directory, with `.` and `..` doing
/// what they say — and no filesystem consulted, so the same graph resolves
/// the same way everywhere.
#[test]
fn a_written_path_joins_onto_the_file_that_wrote_it() {
    let cases = [
        ("pieces/01.musa", "../library/patches.musa", "library/patches.musa"),
        ("pieces/01.musa", "./shared.musa", "pieces/shared.musa"),
        ("a/b/c.musa", "../../top.musa", "top.musa"),
        ("piece.musa", "library.musa", "library.musa"),
        ("/abs/pieces/01.musa", "../library/m.musa", "/abs/library/m.musa"),
    ];
    for (importer, written, expected) in cases {
        assert_eq!(
            resolve_import(importer, written),
            expected,
            "`{written}` from `{importer}`"
        );
    }
}

/// The property everything else rests on: importing a motif is writing it in
/// place. The two pieces below must compile to the same notes.
#[test]
fn an_imported_motif_sounds_exactly_as_it_would_written_in_place() {
    let imported = snapshot(compile_with(
        "p.musa",
        &piece("import \"lib.musa\";"),
        &[("lib.musa", MOTIFS)],
    ));
    let local = snapshot(compile_with(
        "p.musa",
        &piece("motif rise() { c5/4 d5/4 e5/4 g5/4 }"),
        &[],
    ));
    let events = |score: &ScoreSnapshot| {
        score
            .parts()
            .iter()
            .flat_map(|(_, part)| part.voices().map(|(_, voice)| voice))
            .flat_map(|voice| voice.events().iter())
            .map(|event| (event.onset, event.kind.clone(), event.notated_duration.value))
            .collect::<Vec<_>>()
    };
    assert_eq!(events(&imported), events(&local));
}

/// The same file reached twice is read once and shared — importing a library
/// that your library already imported is not a duplicate declaration.
#[test]
fn a_file_reached_twice_is_read_once() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"lib.musa\"; import \"also.musa\";"),
        &[("lib.musa", MOTIFS), ("also.musa", " import \"lib.musa\"; ")],
    );
    assert_eq!(errors(&compilation), Vec::<String>::new());
    assert_eq!(snapshot(compilation).motifs().len(), 1, "one declaration, not two");
}

/// A cycle is reported with the files in it, so the fix is visible from the
/// message rather than from a stack trace.
#[test]
fn an_import_cycle_is_reported_with_its_files() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"a.musa\";"),
        &[("a.musa", " import \"b.musa\"; "), ("b.musa", " import \"a.musa\"; ")],
    );
    let messages = errors(&compilation);
    assert!(
        messages.iter().any(|message| message.contains("import each other")
            && message.contains("a.musa")
            && message.contains("b.musa")),
        "expected a cycle naming both files, got {messages:?}"
    );
}

/// A file that is not there is named, at the `import` that asked for it.
#[test]
fn a_missing_import_names_the_path_it_looked_for() {
    let compilation = compile_with("pieces/01.musa", &piece("import \"../library/gone.musa\";"), &[]);
    assert!(
        errors(&compilation)
            .first()
            .is_some_and(|first| first.starts_with("cannot find `library/gone.musa`")),
        "the missing file is the first thing said; what it would have declared follows"
    );
}

/// The smallest adapter that answers: whatever the region held, unchanged.
///
/// Small on purpose — what is under test is which file the import found, not
/// what the module in it does, so the module does as little as a module can.
const ECHO: &str = "\n    let level = \"readable\";\n    let expand = fn (region: Syntax(TokenTree)) -> Result(Syntax(TokenTree), Pair(Syntax(TokenTree), Text)) { Ok(region) };\n\n";

/// A syntax import resolves by the path its statement *has*, and a path in
/// quotes is the path without them.
///
/// The regression is the quotes. A relative path is written quoted and a
/// string token's text carries its quote characters, so a phase that spelled
/// the path a second time out of the statement's tokens asked for a key with
/// `"` in it, while whoever owns the filesystem keys the file under the path
/// the statement reads. The two never met: a module sitting exactly where the
/// import said was reported as one this compilation cannot read, which put
/// every relative adapter out of reach of the CLI and the desktop both.
#[test]
fn a_syntax_import_written_in_quotes_reads_the_module_at_that_path() {
    // The key the filesystem side computes for this import, and therefore the
    // one the compiler has to look the module up under.
    let resolved = resolve_import("pieces/01.musa", "../adapters/echo.musa");
    assert_eq!(
        resolved, "adapters/echo.musa",
        "the path a reader of the import expects"
    );
    let compilation = compile_with(
        "pieces/01.musa",
        "piece \"P\" {\n    import syntax \"../adapters/echo.musa\" as echo;\n\n    let held = syntax echo { 1 };\n\n    \
         tempo 1/4 = 60;\n    meter 4/4;\n    key c major;\n    score { part p { voice v { c5/1 } } }\n}\n",
        &[(resolved.as_str(), ECHO)],
    );
    let messages = errors(&compilation);
    assert!(
        !messages
            .iter()
            .any(|message| message.contains("is not a module this compilation can read")),
        "the module is at the path the import names, got {messages:?}"
    );
    assert_eq!(
        messages,
        Vec::<String>::new(),
        "and the region it reads expands, rather than the import quietly going missing"
    );
}

/// An imported file declares no piece. A piece in one is rejected — the point
/// of the rule is that a score cannot be imported and silently ignored.
#[test]
fn a_piece_cannot_be_imported() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"other.musa\";"),
        &[(
            "other.musa",
            "piece \"Other\" { tempo 1/4 = 60; meter 4/4; key c major; score { part p { voice v { c5/1 } } } }",
        )],
    );
    let messages = errors(&compilation);
    assert!(
        messages.iter().any(|message| message.contains("declares a piece")),
        "expected a rejection naming the piece, got {messages:?}"
    );
}

/// Names are flat, and a collision is an error rather than a silent winner.
#[test]
fn two_declarations_of_one_name_is_an_error_not_a_shadow() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"lib.musa\"; motif rise() { c5/1 }"),
        &[("lib.musa", MOTIFS)],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`rise` is declared twice")),
        "expected a collision naming the motif, got {messages:?}"
    );
}

/// A library ships building blocks. Wiring belongs to the piece, which is the
/// only thing that knows what parts exist.
#[test]
fn a_library_studio_may_not_wire_a_score_it_cannot_see() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"lib.musa\";"),
        &[("lib.musa", " motif rise() { c5/1 } studio { assign p -> reed; } ")],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`assign` belongs to the piece")),
        "expected the library's `assign` to be refused, got {messages:?}"
    );
}

/// A syntax error inside a library is reported against the file it is in,
/// because a byte offset from another document would point at the wrong
/// bytes of this one.
#[test]
fn a_broken_library_is_reported_by_name() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"lib.musa\";"),
        &[("lib.musa", " motif rise( { c5/1 } ")],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.starts_with("`lib.musa` does not compile")),
        "expected the library's name in the message, got {messages:?}"
    );
}

#[test]
fn an_unknown_standard_module_reports_its_stable_virtual_uri() {
    let compilation = compile_with("p.musa", &piece("import std::unknown; motif rise() { c5/1 }"), &[]);
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("musa-stdlib:/std/unknown.musa")),
        "expected the virtual URI in the diagnostic, got {messages:?}"
    );
}

/// Flat binding is affordable because the collision it risks is reported, and
/// reported with enough to act on: which two modules, and which name.
#[test]
fn two_modules_exporting_one_name_are_both_named() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"a.musa\"; import \"b.musa\";"),
        &[
            ("a.musa", " fn rise() -> EventTrack(WrittenTime) { music { c5/4 } } "),
            ("b.musa", " fn rise() -> EventTrack(WrittenTime) { music { g5/4 } } "),
        ],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`a.musa` and `b.musa` both declare `rise`") && message.contains("`as`")),
        "expected both modules named and `as` offered, got {messages:?}"
    );
}

/// And `as` resolves it: the qualified import is reached through its alias and
/// by nothing else, so the bare name means one thing again.
#[test]
fn an_alias_resolves_a_collision_by_qualifying_one_import() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"a.musa\"; import \"b.musa\" as low; motif fall() { use low.rise(); }"),
        &[
            ("a.musa", " fn rise() -> EventTrack(WrittenTime) { music { c5/4 } } "),
            ("b.musa", " fn rise() -> EventTrack(WrittenTime) { music { g5/4 } } "),
        ],
    );
    assert_eq!(errors(&compilation), Vec::<String>::new());
    let snapshot = snapshot(compilation);
    assert!(
        snapshot.motifs().iter().any(|motif| motif.name == "fall"),
        "the aliased module's function is reachable through its alias"
    );
}

/// The other half of the same rule: an alias is a qualification, not a second
/// spelling. What it renames stops answering to its bare name.
#[test]
fn a_qualified_import_does_not_also_bind_flat() {
    let compilation = compile_with(
        "p.musa",
        &piece("import \"a.musa\" as high; motif fall() { use rise(); }"),
        &[("a.musa", " fn rise() -> EventTrack(WrittenTime) { music { c5/4 } } ")],
    );
    let messages = errors(&compilation);
    assert!(
        messages.iter().any(|message| message.contains("cannot find `rise`")),
        "expected the bare name to be gone, got {messages:?}"
    );
}

/// A document with `head` above its piece, so a law can write root
/// declarations that the [`piece`] helper's body position cannot hold.
fn document(head: &str) -> String {
    format!(
        "{head}\npiece \"P\" {{ tempo 1/4 = 60; meter 4/4; key c major; score {{ part p {{ voice v {{ c5/1 }} }} }} }}\n"
    )
}

/// `private` is a marker the compiler carries all the way to the kernel's
/// visibility rule, and the rule is about *files*: `01-surface.md` §1.3 makes a
/// marked declaration "nameable from a sibling definition in its own module and
/// from nowhere else", and a Musa module is a file.
///
/// The file is in the sentence on purpose. The point of refusing here rather
/// than answering "cannot find" is to tell a reader the name is real and
/// maintained somewhere else, which is worth nothing unless the report says
/// where.
#[test]
fn a_private_definition_is_out_of_reach_across_an_import_and_the_refusal_names_the_file() {
    let compilation = compile_with(
        "p.musa",
        &document("import \"lib.musa\";\nlet borrowed: Nat = held;"),
        &[("lib.musa", " private let held: Nat = 3; ")],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`held` is private to `lib.musa`")),
        "expected the refusal to name the file, got {messages:?}"
    );
    assert!(
        messages.iter().all(|message| !message.contains("cannot find `held`")),
        "the name exists and the report must not say otherwise, got {messages:?}"
    );
}

/// The same rule over a `data` family, which is what sealing became: the type
/// crosses the import and its constructors do not, so a client writes the type
/// in a signature and receives a value from what the file exports.
///
/// All the cases or none of them (§1.3's `mixed-visibility`), so the family
/// below marks its one case and the type stays public.
#[test]
fn a_private_case_lets_the_type_cross_the_import_and_keeps_the_constructor_home() {
    const SEALED: &str = "
        enum Register { private Made(Nat) }
        fn made(count: Nat) -> Register { Register::Made(count) }
        fn count_of(register: Register) -> Nat { match register { Register::Made(count) -> count } }
    ";
    let reached = compile_with(
        "p.musa",
        &document("import \"lib.musa\";\nlet mine: Register = made(3);\nlet counted: Nat = count_of(mine);"),
        &[("lib.musa", SEALED)],
    );
    assert_eq!(
        errors(&reached),
        Vec::<String>::new(),
        "the type and the functions over it are public"
    );

    let minted = compile_with(
        "p.musa",
        &document("import \"lib.musa\";\nlet mine: Register = Register::Made(3);"),
        &[("lib.musa", SEALED)],
    );
    let messages = errors(&minted);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`Register.Made`") && message.contains("`lib.musa`")),
        "expected the constructor refused and the file named, got {messages:?}"
    );
}

/// And inside its own file the marker changes nothing: a sibling definition
/// names it with no ceremony, which is what makes the sealed file writable at
/// all.
#[test]
fn a_private_declaration_is_an_ordinary_name_inside_its_own_file() {
    let compilation = compile_with(
        "p.musa",
        &document("import \"lib.musa\";\nlet borrowed: Nat = doubled;"),
        &[("lib.musa", " private let held: Nat = 3; let doubled: Nat = held; ")],
    );
    assert_eq!(errors(&compilation), Vec::<String>::new());
}

/// A document's lexical root and the `piece` inside it are two sources of one
/// file, so a voice reaches the file's own `private` declarations. A number
/// keyed on the source rather than the file would break exactly this.
#[test]
fn a_voice_names_the_private_declarations_of_its_own_file() {
    let source = "private fn hidden() -> EventTrack(WrittenTime) { music { c5/2 d5/2 } }\n\
         piece \"P\" { tempo 1/4 = 60; meter 4/4; key c major; \
         score { part p { voice v { use hidden(); } } } }\n";
    let compilation = compile_with("p.musa", source, &[]);
    assert_eq!(errors(&compilation), Vec::<String>::new());
    let snapshot = snapshot(compilation);
    let sounded: usize = snapshot
        .parts()
        .iter()
        .flat_map(|(_, part)| part.voices())
        .map(|(_, voice)| voice.events().len())
        .sum();
    assert_eq!(sounded, 2, "the private function's two notes are the voice's");
}

/// An alias is a spelling and not a scope. `import p::q as alias;` files the
/// names under `alias.name`, and a `private` one is refused under the qualified
/// spelling for the reason it is refused under the bare one.
#[test]
fn an_aliased_import_hides_what_the_bare_one_hides() {
    let compilation = compile_with(
        "p.musa",
        &document("import \"lib.musa\" as low;\nlet borrowed: Nat = low.held;"),
        &[("lib.musa", " private let held: Nat = 3; ")],
    );
    let messages = errors(&compilation);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`low.held` is private to `lib.musa`")),
        "expected the alias to reach the same refusal, got {messages:?}"
    );
}

/// The corpus this prompt is measured on. `stdlib/src/context.musa` was
/// rewritten by 162 to seal by marking rather than by listing, and its
/// registers and spelling functions are the three declarations that sealing was
/// protecting.
#[test]
fn the_standard_context_keeps_its_registers_and_spellings_to_itself() {
    for hidden in ["c_major_home", "c_major_spell", "c_major_voicing"] {
        let compilation = compile_with(
            "p.musa",
            &document(&format!("import std::context;\nlet borrowed = {hidden};")),
            &[],
        );
        let messages = errors(&compilation);
        assert!(
            messages
                .iter()
                .any(|message| message.contains(&format!("`{hidden}` is private to"))
                    && message.contains("context.musa")),
            "expected `{hidden}` refused and its file named, got {messages:?}"
        );
    }
    let published = compile_with(
        "p.musa",
        &document("import std::context;\nlet home: TonalContext = c_major;"),
        &[],
    );
    assert_eq!(
        errors(&published),
        Vec::<String>::new(),
        "what the file publishes is still reachable"
    );
}
