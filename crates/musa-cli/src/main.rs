//! Thin command-line interface over `musa-project`.
//!
//! The CLI must call public facades only; it never recreates compiler
//! orchestration (design roadmap §15.8). Until `musa-project` exists
//! (prompt 14), `format` and `check` call `musa_language` directly — the
//! call sites are one-liners so the swap is mechanical.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("format") => cmd_format(args.get(1..).unwrap_or_default()),
        Some("check") => cmd_check(args.get(1..).unwrap_or_default()),
        _ => {
            print_usage();
            if args.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

fn print_usage() {
    println!("musa — notation-first music language and workbench");
    println!();
    println!("Commands:");
    println!("  musa check <file.musa>                 parse + compile diagnostics");
    println!("  musa format <file.musa> [--check]      format in place (--check to diff)");
    println!("  musa render <file.musa> --to <target>  mei | lilypond | musicxml | midi | wav (planned)");
    println!("  musa play <file.musa>                  live playback (planned)");
}

/// Read a `.musa` file, or report the I/O error and give up.
fn read_source(path: &str) -> Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|error| {
        eprintln!("error: cannot read {path}: {error}");
        ExitCode::FAILURE
    })
}

/// `musa format <file> [--check]`
fn cmd_format(args: &[String]) -> ExitCode {
    let check_only = args.iter().any(|arg| arg == "--check");
    let Some(path) = args.iter().find(|arg| !arg.starts_with("--")) else {
        eprintln!("error: format needs a file");
        return ExitCode::FAILURE;
    };
    let source = match read_source(path) {
        Ok(source) => source,
        Err(code) => return code,
    };
    let document = musa_language::parse(&source);
    let formatted = musa_language::format(&document);
    if formatted.text() == source {
        return ExitCode::SUCCESS;
    }
    if check_only {
        eprintln!("{path}: not formatted");
        return ExitCode::FAILURE;
    }
    match std::fs::write(path, formatted.text()) {
        Ok(()) => {
            println!("{path}: formatted");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: cannot write {path}: {error}");
            ExitCode::FAILURE
        }
    }
}

/// A `SyntaxError` rendered with source context by miette.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("{message}")]
struct CliDiagnostic {
    message: String,
    #[source_code]
    src: miette::NamedSource<String>,
    #[label("here")]
    span: miette::SourceSpan,
}

/// `musa check <file>` — lex + parse + diagnostics (semantic checks arrive
/// with prompt 05 and extend this command).
fn cmd_check(args: &[String]) -> ExitCode {
    let Some(path) = args.iter().find(|arg| !arg.starts_with("--")) else {
        eprintln!("error: check needs a file");
        return ExitCode::FAILURE;
    };
    let source = match read_source(path) {
        Ok(source) => source,
        Err(code) => return code,
    };
    let document = musa_language::parse(&source);
    if document.errors().is_empty() {
        println!("{path}: ok");
        return ExitCode::SUCCESS;
    }
    for error in document.errors() {
        let start = usize::from(error.range().start());
        let len = usize::from(error.range().end()).saturating_sub(start);
        let diagnostic = CliDiagnostic {
            message: error.message().to_string(),
            src: miette::NamedSource::new(path, source.clone()),
            span: miette::SourceSpan::from((miette::SourceOffset::from(start), len)),
        };
        eprintln!("{:?}", miette::Report::new(diagnostic));
    }
    ExitCode::FAILURE
}
