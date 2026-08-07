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
        Some("render") => cmd_render(args.get(1..).unwrap_or_default()),
        Some("play") => cmd_play(args.get(1..).unwrap_or_default()),
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
    println!("  musa render <file.musa> --to <target>  plan (debug) | mei | lilypond | performance | wav");
    println!("  musa play <file.musa> [--loop]         live playback through the audio engine");
}

mod orchestrate;

fn cmd_render(args: &[String]) -> ExitCode {
    let mut path: Option<&str> = None;
    let mut target = "plan";
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
    let source = match musa_compiler::SourceDocument::open(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let compilation = musa_compiler::compile(&source, &musa_compiler::CompileOptions::default());
    let Some(score) = compilation.into_snapshot() else {
        eprintln!("error: {path}: compilation failed");
        return ExitCode::FAILURE;
    };
    match target {
        "plan" => match musa_render::plan_notation(&score, &musa_render::NotationOptions::default()) {
            Ok(plan) => {
                println!("{plan:#?}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        "performance" => {
            match musa_compiler::lower_performance(&score, &musa_compiler::PerformanceOptions::default()) {
                Ok(plan) => {
                    print_performance(&plan);
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        "mei" => render_backend(path, &score, musa_render::NotationTarget::Mei, output, "mei"),
        "lilypond" => render_backend(path, &score, musa_render::NotationTarget::LilyPond, output, "ly"),
        "wav" => match orchestrate::render_to_wav(&score) {
            Ok(rendered) => write_bytes(path, &rendered.bytes, output, "wav"),
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        other => {
            eprintln!("error: --to {other} is not implemented yet (plan | mei | lilypond | performance | wav)");
            ExitCode::FAILURE
        }
    }
}

fn render_backend(
    path: &str,
    score: &musa_compiler::ScoreSnapshot,
    target: musa_render::NotationTarget,
    output: Option<&str>,
    extension: &str,
) -> ExitCode {
    match musa_render::render_notation(score, target, &musa_render::NotationOptions::default()) {
        Ok(rendered) => write_output(path, rendered.text(), output, extension),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The `--to performance` debug dump (snapshot surface until audio exists).
fn print_performance(plan: &musa_compiler::PerformancePlan) {
    for lane in plan.lanes() {
        println!("lane {}:", lane.name());
        for event in lane.events() {
            match event {
                musa_compiler::PerformanceEvent::NoteOn { frame, note, instance } => {
                    println!(
                        "  on  {frame} {} {:.2}Hz event-{:x} i{}",
                        note.pitch, note.frequency, note.event.0, instance.0
                    );
                }
                musa_compiler::PerformanceEvent::NoteOff { frame, instance } => {
                    println!("  off {frame} i{}", instance.0);
                }
                musa_compiler::PerformanceEvent::Parameter { frame, target, value } => {
                    println!("  par {frame} p{} {value}", target.0);
                }
            }
        }
    }
}

/// Write rendered output: stdout for `-o -`, `<name>.<ext>` beside the input
/// by default, or the given path.
fn write_output(input: &str, text: &str, output: Option<&str>, extension: &str) -> ExitCode {
    let destination = match output {
        Some("-") => None,
        Some(path) => Some(path.to_string()),
        None => Some(
            std::path::Path::new(input)
                .with_extension(extension)
                .to_string_lossy()
                .to_string(),
        ),
    };
    match destination {
        None => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Some(path) => match std::fs::write(&path, text) {
            Ok(()) => {
                println!("wrote {path}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: cannot write {path}: {error}");
                ExitCode::FAILURE
            }
        },
    }
}

/// `musa play <file> [--loop]`: compile, prepare, open the engine, install,
/// play to completion (Ctrl-C terminates the process). Thin by design;
/// prompt 19 moves orchestration into `musa-project`.
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
    let source = match musa_compiler::SourceDocument::open(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let compilation = musa_compiler::compile(&source, &musa_compiler::CompileOptions::default());
    let Some(score) = compilation.into_snapshot() else {
        eprintln!("error: {path}: compilation failed");
        return ExitCode::FAILURE;
    };
    let plan = match orchestrate::prepare_playback(&score) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let total = plan.total_frames();
    let engine = match musa_engine::AudioEngine::open(musa_engine::EngineConfig::default()) {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    if engine.install(plan).is_err() || engine.command(musa_engine::TransportCommand::Play).is_err() {
        eprintln!("error: engine command queue is full");
        return ExitCode::FAILURE;
    }
    if looping {
        let _ignored = engine.command(musa_engine::TransportCommand::SetLoop { start: 0, end: total });
    }
    println!("playing {path}{}…", if looping { " (looping)" } else { "" });
    while engine.is_playing() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    println!("done");
    ExitCode::SUCCESS
}

/// Write binary output (WAV) to `-o` or the input path with `extension`.
fn write_bytes(input: &str, bytes: &[u8], output: Option<&str>, extension: &str) -> ExitCode {
    let destination = match output {
        Some(path) => path.to_string(),
        None => std::path::Path::new(input)
            .with_extension(extension)
            .to_string_lossy()
            .to_string(),
    };
    match std::fs::write(&destination, bytes) {
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

/// `musa check <file>...` — full semantic check through `musa_compiler`.
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
    let source = match musa_compiler::SourceDocument::open(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let compilation = musa_compiler::compile(&source, &musa_compiler::CompileOptions::default());
    for diagnostic in compilation.diagnostics() {
        let severity = match diagnostic.severity {
            musa_compiler::Severity::Error => "error",
            musa_compiler::Severity::Warning => "warning",
        };
        let rendered = CliDiagnostic {
            severity,
            message: diagnostic.message.clone(),
            src: miette::NamedSource::new(path, source.text().to_string()),
            span: diagnostic.span.map(|span| {
                miette::SourceSpan::from((
                    miette::SourceOffset::from(usize::try_from(span.start).unwrap_or(0)),
                    usize::try_from(span.end.saturating_sub(span.start)).unwrap_or(0),
                ))
            }),
        };
        eprintln!("{:?}", miette::Report::new(rendered));
    }
    if compilation.has_errors() {
        ExitCode::FAILURE
    } else {
        println!("{path}: ok");
        ExitCode::SUCCESS
    }
}
