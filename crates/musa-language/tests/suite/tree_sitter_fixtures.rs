//! The tree-sitter drift law.
//!
//! `editors/tree-sitter-musa` is a second reader of this language, and a
//! second reader owns no vocabulary: this test writes the *real* lexer's
//! token stream for every `examples/*.musa`, and the *real* parser's verdict
//! on every `examples/broken/*.musa`, into the grammar's committed test
//! data. The grammar's own `test/compare-tokens.js` then holds its parse to
//! both, token for token. A grammar that disagrees with the lexer about one
//! token fails loudly here, not in a composer's editor.
//!
//! Run `UPDATE_FIXTURES=1 cargo test -p musa-language --test
//! tree_sitter_fixtures` to refresh.

// Fixture generation writes files and panics on a stale copy by design: a
// generator that shrugs is a generator that drifts.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use musa_language::{SyntaxKind, Token, lex, parse};

/// The repository root's `examples/`.
fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// The grammar project's test data, the directory this test owns.
fn test_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../editors/tree-sitter-musa/test")
}

/// Write `contents` to `path`, or — when the fixture is committed and stale —
/// say exactly which file drifted and how to refresh it.
fn write_or_compare(path: &Path, contents: &str) {
    let current = std::fs::read_to_string(path).ok();
    if current.as_deref() == Some(contents) {
        return;
    }
    assert!(
        current.is_none() || std::env::var_os("UPDATE_FIXTURES").is_some(),
        "{} is stale — rerun with UPDATE_FIXTURES=1 and commit the result",
        path.display()
    );
    std::fs::create_dir_all(path.parent().expect("fixture directory")).expect("create fixture directory");
    std::fs::write(path, contents).expect("write fixture");
}

/// Every `.musa` directly under `dir`, sorted so the output is stable.
fn musa_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|_| panic!("read {}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "musa"))
        .collect();
    files.sort();
    files
}

/// A JSON string literal's interior: the texts here are source slices, so
/// quotes, backslashes, and control characters are all possible.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character.is_control() => {
                writeln!(out, "\\u{:04x}", u32::from(character)).expect("write to a string");
            }
            character => out.push(character),
        }
    }
    out
}

/// The tree-sitter leaf name a token kind becomes in the grammar.
///
/// Keywords and punctuation are anonymous tokens in `grammar.js`, named by
/// their text; literals, names, units, and comments are named tokens. The
/// match is total on purpose: a kind the lexer grows without the grammar
/// following is a compile error here, which is the earliest the law can
/// speak.
fn tree_sitter_name(kind: SyntaxKind) -> &'static str {
    match kind {
        SyntaxKind::LineComment | SyntaxKind::BlockComment => "comment",
        SyntaxKind::Identifier => "identifier",
        SyntaxKind::Integer => "integer",
        SyntaxKind::Float => "float",
        SyntaxKind::Rational => "rational",
        SyntaxKind::String => "string",
        SyntaxKind::PitchLiteral => "pitch_literal",
        SyntaxKind::IntervalLiteral => "interval_literal",
        SyntaxKind::UnitHz | SyntaxKind::UnitMs | SyntaxKind::UnitS | SyntaxKind::UnitDb => "unit",
        SyntaxKind::LBrace => "{",
        SyntaxKind::RBrace => "}",
        SyntaxKind::LBracket => "[",
        SyntaxKind::RBracket => "]",
        SyntaxKind::LParen => "(",
        SyntaxKind::RParen => ")",
        SyntaxKind::Semicolon => ";",
        SyntaxKind::Comma => ",",
        SyntaxKind::Colon => ":",
        SyntaxKind::Arrow => "->",
        SyntaxKind::PipeForward => "|>",
        SyntaxKind::Equals => "=",
        SyntaxKind::Minus => "-",
        SyntaxKind::Tilde => "~",
        SyntaxKind::Dot => ".",
        SyntaxKind::Slash => "/",
        SyntaxKind::Pipe => "|",
        SyntaxKind::Greater => ">",
        SyntaxKind::Less => "<",
        SyntaxKind::Caret => "^",
        SyntaxKind::Hash => "#",
        SyntaxKind::Dollar => "$",
        SyntaxKind::TemplateKw => "template",
        SyntaxKind::SignatureKw => "signature",
        SyntaxKind::StructureKw => "structure",
        SyntaxKind::DataKw => "data",
        SyntaxKind::ModuleKw => "module",
        SyntaxKind::MakeKw => "make",
        SyntaxKind::AsKw => "as",
        SyntaxKind::PieceKw => "piece",
        SyntaxKind::TempoKw => "tempo",
        SyntaxKind::MeterKw => "meter",
        SyntaxKind::KeyKw => "key",
        SyntaxKind::SubtitleKw => "subtitle",
        SyntaxKind::ComposerKw => "composer",
        SyntaxKind::ArrangerKw => "arranger",
        SyntaxKind::CopyrightKw => "copyright",
        SyntaxKind::MotifKw => "motif",
        SyntaxKind::ScoreKw => "score",
        SyntaxKind::PartKw => "part",
        SyntaxKind::VoiceKw => "voice",
        SyntaxKind::ClefKw => "clef",
        SyntaxKind::UseKw => "use",
        SyntaxKind::ImportKw => "import",
        SyntaxKind::SyntaxKw => "syntax",
        SyntaxKind::ModKw => "mod",
        SyntaxKind::TransposeKw => "transpose",
        SyntaxKind::DownKw => "down",
        SyntaxKind::UpKw => "up",
        SyntaxKind::RestKw => "rest",
        SyntaxKind::RepeatKw => "repeat",
        SyntaxKind::SlurKw => "slur",
        SyntaxKind::DynamicKw => "dynamic",
        SyntaxKind::TupletKw => "tuplet",
        SyntaxKind::PerformanceKw => "performance",
        SyntaxKind::ProfileKw => "profile",
        SyntaxKind::MarkKw => "mark",
        SyntaxKind::GrooveKw => "groove",
        SyntaxKind::GraceKw => "grace",
        SyntaxKind::StudioKw => "studio",
        SyntaxKind::PatchKw => "patch",
        SyntaxKind::ModulateKw => "modulate",
        SyntaxKind::BusKw => "bus",
        SyntaxKind::AssignKw => "assign",
        SyntaxKind::RouteKw => "route",
        SyntaxKind::SendKw => "send",
        SyntaxKind::MasterKw => "master",
        SyntaxKind::AtKw => "at",
        SyntaxKind::OutputKw => "output",
        SyntaxKind::PitchKw => "pitch",
        SyntaxKind::StretchKw => "stretch",
        SyntaxKind::RetrogradeKw => "retrograde",
        SyntaxKind::InvertKw => "invert",
        SyntaxKind::AroundKw => "around",
        SyntaxKind::WithKw => "with",
        SyntaxKind::NoteKw => "note",
        SyntaxKind::PhraseKw => "phrase",
        SyntaxKind::SectionKw => "section",
        SyntaxKind::HarmonyKw => "harmony",
        SyntaxKind::LibraryKw => "library",
        SyntaxKind::CrescendoKw => "crescendo",
        SyntaxKind::DiminuendoKw => "diminuendo",
        SyntaxKind::ToKw => "to",
        SyntaxKind::BarKw => "bar",
        SyntaxKind::AssertKw => "assert",
        SyntaxKind::SenzaKw => "senza",
        SyntaxKind::EndingKw => "ending",
        SyntaxKind::FragmentKw => "fragment",
        SyntaxKind::MobileKw => "mobile",
        SyntaxKind::ImproviseKw => "improvise",
        SyntaxKind::OverKw => "over",
        SyntaxKind::LetKw => "let",
        SyntaxKind::FnKw => "fn",
        SyntaxKind::MusicKw => "music",
        SyntaxKind::KernelKw => "kernel",
        SyntaxKind::OptionKw => "Option",
        SyntaxKind::ListKw => "List",
        SyntaxKind::ResultKw => "Result",
        SyntaxKind::MatchKw => "match",
        SyntaxKind::IfKw => "if",
        SyntaxKind::ElseKw => "else",
        SyntaxKind::SomeKw => "Some",
        SyntaxKind::NoneKw => "None",
        SyntaxKind::OkKw => "Ok",
        SyntaxKind::ErrKw => "Err",
        SyntaxKind::TrueKw => "true",
        SyntaxKind::FalseKw => "false",
        SyntaxKind::ScaleKw => "scale",
        SyntaxKind::DegreeKw => "degree",
        SyntaxKind::FrameKw => "frame",
        SyntaxKind::InKw => "in",
        SyntaxKind::StepKw => "step",
        SyntaxKind::ChordKw => "chord",
        SyntaxKind::StackKw => "stack",
        // Whitespace is not a token the grammar sees (`extras`), and the
        // kinds below never leave the lexer for a valid example: a `bpm` or
        // an error token here means the grammar is behind the language,
        // which is what the panic says.
        SyntaxKind::Whitespace | SyntaxKind::UnitBpm | SyntaxKind::Error => {
            panic!("`{kind:?}` cannot be named: the grammar has no rule for it")
        }
        // Node kinds are not tokens; a token stream never contains one.
        SyntaxKind::Root
        | SyntaxKind::PieceDecl
        | SyntaxKind::TempoStmt
        | SyntaxKind::MeterStmt
        | SyntaxKind::KeyStmt
        | SyntaxKind::FrontMatterStmt
        | SyntaxKind::MotifDecl
        | SyntaxKind::ScoreDecl
        | SyntaxKind::PartDecl
        | SyntaxKind::ClefStmt
        | SyntaxKind::VoiceDecl
        | SyntaxKind::NoteStmt
        | SyntaxKind::RestStmt
        | SyntaxKind::ChordStmt
        | SyntaxKind::UseStmt
        | SyntaxKind::TransposeStmt
        | SyntaxKind::RepeatStmt
        | SyntaxKind::SlurStmt
        | SyntaxKind::DynamicStmt
        | SyntaxKind::TupletStmt
        | SyntaxKind::StretchStmt
        | SyntaxKind::RetrogradeStmt
        | SyntaxKind::InvertStmt
        | SyntaxKind::WithClause
        | SyntaxKind::OverrideStmt
        | SyntaxKind::PhraseStmt
        | SyntaxKind::MarkStmt
        | SyntaxKind::GraceStmt
        | SyntaxKind::GraceNote
        | SyntaxKind::SectionStmt
        | SyntaxKind::HarmonyDecl
        | SyntaxKind::HarmonyStmt
        | SyntaxKind::Position
        | SyntaxKind::PitchClass
        | SyntaxKind::ChordSymbol
        | SyntaxKind::LibraryDecl
        | SyntaxKind::ImportStmt
        | SyntaxKind::HairpinStmt
        | SyntaxKind::Duration
        | SyntaxKind::ArticulationList
        | SyntaxKind::PerformanceDecl
        | SyntaxKind::ProfileDecl
        | SyntaxKind::MarkRule
        | SyntaxKind::DynamicRule
        | SyntaxKind::GrooveRule
        | SyntaxKind::GraceRule
        | SyntaxKind::SettingStmt
        | SyntaxKind::ProfileStmt
        | SyntaxKind::StudioDecl
        | SyntaxKind::PatchDecl
        | SyntaxKind::BusDecl
        | SyntaxKind::SignalBinding
        | SyntaxKind::ChainStmt
        | SyntaxKind::SignalChain
        | SyntaxKind::CallExpr
        | SyntaxKind::ArgList
        | SyntaxKind::Arg
        | SyntaxKind::ValueLiteral
        | SyntaxKind::NameRef
        | SyntaxKind::ModulateStmt
        | SyntaxKind::ParamPath
        | SyntaxKind::AssignStmt
        | SyntaxKind::RouteStmt
        | SyntaxKind::SendStmt
        | SyntaxKind::Block
        | SyntaxKind::BarStmt
        | SyntaxKind::AssertStmt
        | SyntaxKind::SenzaStmt
        | SyntaxKind::EndingStmt
        | SyntaxKind::FragmentDecl
        | SyntaxKind::MobileStmt
        | SyntaxKind::ImproviseStmt
        | SyntaxKind::LetDecl
        | SyntaxKind::FnDecl
        | SyntaxKind::Param
        | SyntaxKind::ParamList
        | SyntaxKind::TypeExpr
        | SyntaxKind::TypeName
        | SyntaxKind::FunctionType
        | SyntaxKind::ProductType
        | SyntaxKind::OptionType
        | SyntaxKind::ListType
        | SyntaxKind::ResultType
        | SyntaxKind::NameExpr
        | SyntaxKind::LiteralExpr
        | SyntaxKind::ParenExpr
        | SyntaxKind::ProductExpr
        | SyntaxKind::ListExpr
        | SyntaxKind::OptionExpr
        | SyntaxKind::ResultExpr
        | SyntaxKind::ApplyExpr
        | SyntaxKind::LambdaExpr
        | SyntaxKind::ExprArgList
        | SyntaxKind::ExprArg
        | SyntaxKind::MatchExpr
        | SyntaxKind::IfExpr
        | SyntaxKind::RecordUpdateExpr
        | SyntaxKind::FieldUpdate
        | SyntaxKind::MatchArm
        | SyntaxKind::Pattern
        | SyntaxKind::MusicExpr
        | SyntaxKind::ScaleExpr
        | SyntaxKind::KeyExpr
        | SyntaxKind::StepExpr
        | SyntaxKind::InScaleStmt
        | SyntaxKind::ChordExpr
        | SyntaxKind::StackStmt
        | SyntaxKind::TemplateDecl
        | SyntaxKind::MakeStmt
        | SyntaxKind::SignatureDecl
        | SyntaxKind::SignatureMember
        | SyntaxKind::StructureDecl
        | SyntaxKind::BlockExpr
        | SyntaxKind::ModDecl
        | SyntaxKind::DataDecl
        | SyntaxKind::TypeParams
        | SyntaxKind::TypeParam
        | SyntaxKind::DataVariant
        | SyntaxKind::DataField
        | SyntaxKind::AppliedType
        | SyntaxKind::DataMember
        | SyntaxKind::PitchExpr
        | SyntaxKind::KernelQuote
        | SyntaxKind::KernelHole
        | SyntaxKind::SyntaxRegion
        | SyntaxKind::SyntaxGroup => panic!("`{kind:?}` is a node, not a token"),
    }
}

/// The lexer's stream for one file, as the JSON the grammar's token
/// comparison reads. Whitespace is the only thing dropped: it is `extras`
/// in the grammar, not a leaf.
fn token_manifest(relative: &Path, source: &str) -> String {
    let tokens = lex(source);
    let mut json = format!(
        "{{\n  \"file\": \"{}\",\n  \"tokens\": [\n",
        escape(&relative.to_string_lossy())
    );
    let significant: Vec<&Token> = tokens
        .tokens()
        .iter()
        .filter(|token| token.kind != SyntaxKind::Whitespace)
        .collect();
    for (index, token) in significant.iter().enumerate() {
        let text = &source[usize::from(token.range.start())..usize::from(token.range.end())];
        let comma = if index.saturating_add(1) == significant.len() {
            ""
        } else {
            ","
        };
        writeln!(
            json,
            "    {{ \"kind\": \"{}\", \"text\": \"{}\" }}{comma}",
            tree_sitter_name(token.kind),
            escape(text),
        )
        .expect("write to a string");
    }
    json.push_str("  ]\n}\n");
    json
}

/// Which files are the *kernel* alternative, by `musa-language`'s reckoning.
///
/// The drift law for `docs/rules/language/01-surface.md` §7. The grammar has its own
/// rule for the top-level alternative and this crate has [`alternative`], and
/// the one way they can disagree is the one way that matters: a file read as
/// kernel by one and surface by the other opens as a page of red in an editor
/// and compiles fine on the command line.
///
/// Both verdicts are committed, not just the positive one. A rule that
/// recognized *too much* — every file starting with `%`, say — would pass a
/// one-sided law and break every `.musa` file in the repository.
fn kernel_manifest(files: &[(String, String)]) -> String {
    let mut json = format!(
        "{{\n  \"marker\": \"{}\",\n  \"files\": [\n",
        escape(musa_language::KERNEL_MARKER)
    );
    for (index, (relative, source)) in files.iter().enumerate() {
        let kernel = musa_language::alternative(source) == musa_language::DocumentAlternative::Kernel;
        let comma = if index.saturating_add(1) == files.len() {
            ""
        } else {
            ","
        };
        writeln!(
            json,
            "    {{ \"file\": \"{}\", \"kernel\": {kernel} }}{comma}",
            escape(relative),
        )
        .expect("write to a string");
    }
    json.push_str("  ]\n}\n");
    json
}

/// Every `.musa.kernel` directly under `dir`, sorted.
fn kernel_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|_| panic!("read {}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.to_string_lossy().ends_with(".musa.kernel"))
        .collect();
    files.sort();
    files
}

/// The parser's verdict on the broken fixtures: which of them are *syntact*
/// broken. Most of `examples/broken/` is semantically broken and
/// syntactically fine — the grammar must agree, fixture by fixture.
fn broken_manifest(fixtures: &[(String, String)]) -> String {
    let mut json = String::from("{\n  \"fixtures\": [\n");
    for (index, (name, source)) in fixtures.iter().enumerate() {
        let syntax_errors = !parse(source).errors().is_empty();
        let comma = if index.saturating_add(1) == fixtures.len() {
            ""
        } else {
            ","
        };
        writeln!(
            json,
            "    {{ \"name\": \"{}\", \"syntax_errors\": {syntax_errors} }}{comma}",
            escape(name),
        )
        .expect("write to a string");
    }
    json.push_str("  ]\n}\n");
    json
}

#[test]
fn the_lexers_tokens_are_the_grammars_test_data() {
    let examples = examples_dir();
    let data = test_data_dir();

    // Valid fixtures: the top level, and the album's pieces — everything
    // that is meant to compile.
    let mut valid: Vec<(PathBuf, PathBuf)> = musa_files(&examples)
        .into_iter()
        .map(|path| {
            (
                path.strip_prefix(&examples).expect("under examples").to_path_buf(),
                path,
            )
        })
        .collect();
    let album = examples.join("album/pieces");
    if album.is_dir() {
        valid.extend(musa_files(&album).into_iter().map(|path| {
            (
                path.strip_prefix(&examples).expect("under examples").to_path_buf(),
                path,
            )
        }));
    }
    for (relative, path) in &valid {
        let source = std::fs::read_to_string(path).expect("fixture text");
        let name = path.file_stem().expect("file name").to_string_lossy().into_owned();
        write_or_compare(
            &data.join(format!("tokens/{name}.json")),
            &token_manifest(relative, &source),
        );
    }

    // Broken fixtures: the real parser's syntax verdict, fixture by fixture.
    let broken: Vec<(String, String)> = musa_files(&examples.join("broken"))
        .into_iter()
        .map(|path| {
            (
                path.file_stem().expect("file name").to_string_lossy().into_owned(),
                std::fs::read_to_string(&path).expect("fixture text"),
            )
        })
        .collect();
    write_or_compare(&data.join("broken.json"), &broken_manifest(&broken));

    // The two alternatives, from this crate's side: every kernel file is one,
    // and every surface file is not.
    let mut alternatives: Vec<(String, String)> = kernel_files(&examples.join("kernel"))
        .into_iter()
        .map(|path| {
            (
                path.strip_prefix(&examples)
                    .expect("under examples")
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read_to_string(&path).expect("fixture text"),
            )
        })
        .collect();
    alternatives.extend(valid.iter().map(|(relative, path)| {
        (
            relative.to_string_lossy().into_owned(),
            std::fs::read_to_string(path).expect("fixture text"),
        )
    }));
    write_or_compare(&data.join("kernel.json"), &kernel_manifest(&alternatives));
}
