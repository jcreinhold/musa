//! Thin command-line interface over `musa-project`.
//!
//! The CLI calls public facades only; it never chains compiler, renderer, or
//! engine calls itself (design roadmap §15.8). Everything below is argument
//! parsing, one `ProjectSession` call, and printing — if a subcommand here
//! ever needs to orchestrate, the orchestration belongs in `musa-project`.

use std::process::ExitCode;

use musa_project::{ExportArtifact, ExportRequest, MidiMode, ProjectCommand, ProjectSession, TransportRequest};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("format") => cmd_format(args.get(1..).unwrap_or_default()),
        Some("check") => cmd_check(args.get(1..).unwrap_or_default()),
        Some("render") => cmd_render(args.get(1..).unwrap_or_default()),
        Some("play") => cmd_play(args.get(1..).unwrap_or_default()),
        Some("kernel") => cmd_kernel(args.get(1..).unwrap_or_default()),
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
    println!(
        "  musa render <file.musa> --to <target>  plan (debug) | mei | lilypond | musicxml | performance | wav | midi"
    );
    println!("      --mode score|performance             for --to midi (default: score)");
    println!("  musa play <file.musa> [--loop]         live playback through the audio engine");
    println!("  musa kernel <file.musa> [--normalized] print the piece as kernel interchange text");
    println!("  musa kernel --check <file.kernel>      parse, check, and evaluate kernel text");
}

/// Open a project, or report why not.
fn open(path: &str) -> Result<ProjectSession, ExitCode> {
    ProjectSession::open(path).map_err(|error| {
        eprintln!("error: {error}");
        ExitCode::FAILURE
    })
}

/// `musa render <file> --to <target> [-o <path>]`
fn cmd_render(args: &[String]) -> ExitCode {
    let mut path: Option<&str> = None;
    let mut target = "plan";
    let mut mode = "score";
    let mut output: Option<&str> = None;
    let mut index = 0;
    while index < args.len() {
        let Some(arg) = args.get(index).map(String::as_str) else {
            break;
        };
        match arg {
            "--to" => {
                target = args.get(index.saturating_add(1)).map_or("plan", String::as_str);
                index = index.saturating_add(2);
            }
            "--mode" => {
                mode = args.get(index.saturating_add(1)).map_or("score", String::as_str);
                index = index.saturating_add(2);
            }
            "-o" => {
                output = args.get(index.saturating_add(1)).map(String::as_str);
                index = index.saturating_add(2);
            }
            other => {
                path = Some(other);
                index = index.saturating_add(1);
            }
        }
    }
    let Some(path) = path else {
        eprintln!("error: render needs a file");
        return ExitCode::FAILURE;
    };
    let request = match target {
        "plan" => ExportRequest::NotationPlanDump,
        "performance" => ExportRequest::PerformanceDump,
        "mei" => ExportRequest::Mei,
        "lilypond" => ExportRequest::LilyPond,
        "musicxml" => ExportRequest::MusicXml,
        "wav" => ExportRequest::Wav,
        "midi" => match mode {
            "score" => ExportRequest::Midi(MidiMode::Score),
            "performance" => ExportRequest::Midi(MidiMode::Performance),
            other => {
                eprintln!("error: --mode {other} is not a MIDI mode (score | performance)");
                return ExitCode::FAILURE;
            }
        },
        other => {
            eprintln!(
                "error: --to {other} is not implemented yet (plan | mei | lilypond | musicxml | performance | wav | midi)"
            );
            return ExitCode::FAILURE;
        }
    };
    let session = match open(path) {
        Ok(session) => session,
        Err(code) => return code,
    };
    let artifact = match session.export(request) {
        Ok(artifact) => artifact,
        Err(error) => {
            eprintln!("error: {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    // The debug dumps go to stdout by default; the file formats go to a file.
    match request {
        ExportRequest::NotationPlanDump | ExportRequest::PerformanceDump if output.is_none() => {
            println!(
                "{}",
                artifact
                    .as_text()
                    .unwrap_or_default()
                    .trim_end_matches(char::is_whitespace)
            );
            ExitCode::SUCCESS
        }
        ExportRequest::Mei
        | ExportRequest::LilyPond
        | ExportRequest::MusicXml
        | ExportRequest::Wav
        | ExportRequest::PerformanceDump
        | ExportRequest::NotationPlanDump
        | _ => write_artifact(path, &artifact, output, request.extension()),
    }
}

/// `musa kernel <file.musa> [--normalized] [-o <path>]` /
/// `musa kernel --check <file.kernel>`
///
/// One direction only: kernel text is a projection of a piece, and a
/// `.kernel` file is never read back into a document (AGENTS.md — the source
/// is canonical). `--check` is a reader, not an importer.
fn cmd_kernel(args: &[String]) -> ExitCode {
    let mut path: Option<&str> = None;
    let mut output: Option<&str> = None;
    let mut normalized = false;
    let mut check = false;
    let mut index = 0;
    while index < args.len() {
        let Some(arg) = args.get(index).map(String::as_str) else {
            break;
        };
        match arg {
            "--normalized" => {
                normalized = true;
                index = index.saturating_add(1);
            }
            "--check" => {
                check = true;
                index = index.saturating_add(1);
            }
            "-o" => {
                output = args.get(index.saturating_add(1)).map(String::as_str);
                index = index.saturating_add(2);
            }
            other => {
                path = Some(other);
                index = index.saturating_add(1);
            }
        }
    }
    let Some(path) = path else {
        eprintln!("error: kernel needs a file");
        return ExitCode::FAILURE;
    };
    if check {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("error: cannot read {path}: {error}");
                return ExitCode::FAILURE;
            }
        };
        return match musa_project::check_kernel(&text) {
            Ok(report) => {
                println!(
                    "{path}: ok — piece {:?}, {} occurrences, extent {}",
                    report.name, report.occurrences, report.extent
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {path}: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let session = match open(path) {
        Ok(session) => session,
        Err(code) => return code,
    };
    let request = ExportRequest::Kernel { normalized };
    match session.export(request) {
        Ok(artifact) => {
            if output.is_none() {
                print!("{}", artifact.as_text().unwrap_or_default());
                ExitCode::SUCCESS
            } else {
                write_artifact(path, &artifact, output, request.extension())
            }
        }
        Err(error) => {
            eprintln!("error: {path}: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Write an export to `-o`, to stdout for `-o -`, or beside the input.
fn write_artifact(input: &str, artifact: &ExportArtifact, output: Option<&str>, extension: &str) -> ExitCode {
    if output == Some("-") {
        match artifact.as_text() {
            Some(text) => {
                println!("{text}");
                return ExitCode::SUCCESS;
            }
            None => {
                eprintln!("error: this format is binary; give -o <path>");
                return ExitCode::FAILURE;
            }
        }
    }
    let destination = output.map_or_else(
        || {
            std::path::Path::new(input)
                .with_extension(extension)
                .to_string_lossy()
                .into_owned()
        },
        ToOwned::to_owned,
    );
    match std::fs::write(&destination, artifact.as_bytes()) {
        Ok(()) => {
            println!("wrote {destination}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: cannot write {destination}: {error}");
            ExitCode::FAILURE
        }
    }
}

/// `musa play <file> [--loop]` — play to completion; Ctrl-C ends it early.
fn cmd_play(args: &[String]) -> ExitCode {
    let mut path: Option<&str> = None;
    let mut looping = false;
    for arg in args {
        match arg.as_str() {
            "--loop" => looping = true,
            other => path = Some(other),
        }
    }
    let Some(path) = path else {
        eprintln!("error: play needs a file");
        return ExitCode::FAILURE;
    };
    let mut session = match open(path) {
        Ok(session) => session,
        Err(code) => return code,
    };
    if let Err(error) = session.apply(ProjectCommand::Transport(TransportRequest::Play)) {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }
    if looping {
        let end = session.snapshot().playback().total_frames;
        let _ignored = session.apply(ProjectCommand::Transport(TransportRequest::SetLoop { start: 0, end }));
    }
    println!("playing {path}{}…", if looping { " (looping)" } else { "" });
    session.wait_for_playback();
    println!("done");
    ExitCode::SUCCESS
}

/// `musa format <file> [--check]`
fn cmd_format(args: &[String]) -> ExitCode {
    let check_only = args.iter().any(|arg| arg == "--check");
    let Some(path) = args.iter().find(|arg| !arg.starts_with("--")) else {
        eprintln!("error: format needs a file");
        return ExitCode::FAILURE;
    };
    let mut session = match open(path) {
        Ok(session) => session,
        Err(code) => return code,
    };
    let update = match session.apply(ProjectCommand::Format) {
        Ok(update) => update,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    if !update.source_changed {
        return ExitCode::SUCCESS;
    }
    if check_only {
        eprintln!("{path}: not formatted");
        return ExitCode::FAILURE;
    }
    match session.apply(ProjectCommand::Save) {
        Ok(_) => {
            println!("{path}: formatted");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// A diagnostic rendered with source context by miette.
#[derive(Debug, thiserror::Error)]
#[error("{severity}: {message}")]
struct CliDiagnostic {
    severity: &'static str,
    message: String,
    src: miette::NamedSource<String>,
    span: Option<miette::SourceSpan>,
}

impl miette::Diagnostic for CliDiagnostic {
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.src)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let span = self.span?;
        let label = miette::LabeledSpan::new_with_span(None, span);
        Some(Box::new(std::iter::once(label)))
    }

    fn severity(&self) -> Option<miette::Severity> {
        if self.severity == "error" {
            Some(miette::Severity::Error)
        } else {
            Some(miette::Severity::Warning)
        }
    }
}

/// `musa check <file>...` — full semantic check.
fn cmd_check(args: &[String]) -> ExitCode {
    let mut status = ExitCode::SUCCESS;
    let mut files: u32 = 0;
    for path in args.iter().filter(|arg| !arg.starts_with("--")) {
        files = files.saturating_add(1);
        if cmd_check_one(path) == ExitCode::FAILURE {
            status = ExitCode::FAILURE;
        }
    }
    if files == 0 {
        eprintln!("error: check needs a file");
        return ExitCode::FAILURE;
    }
    status
}

fn cmd_check_one(path: &str) -> ExitCode {
    let session = match open(path) {
        Ok(session) => session,
        Err(code) => return code,
    };
    let snapshot = session.snapshot();
    for diagnostic in snapshot.diagnostics() {
        let severity = match diagnostic.severity {
            musa_project::Severity::Error => "error",
            musa_project::Severity::Warning => "warning",
        };
        let rendered = CliDiagnostic {
            severity,
            message: diagnostic.message.clone(),
            src: miette::NamedSource::new(path, snapshot.source().to_owned()),
            span: diagnostic.span.map(|span| {
                miette::SourceSpan::from((
                    miette::SourceOffset::from(usize::try_from(span.start).unwrap_or(0)),
                    usize::try_from(span.end.saturating_sub(span.start)).unwrap_or(0),
                ))
            }),
        };
        eprintln!("{:?}", miette::Report::new(rendered));
    }
    if snapshot.compiles() {
        println!("{path}: ok");
        // What a piece is filed under and what it reads: the two facts a
        // directory project adds, and the two a reader would otherwise have
        // to reconstruct from the `use` statements themselves.
        if let Some(project) = session.project() {
            let name = project.name.as_deref().unwrap_or("untitled");
            println!("  project: {name} ({})", project.root.display());
        }
        for import in session.imports() {
            println!("  imports: {}", import.display());
        }
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
