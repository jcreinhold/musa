//! Thin command-line interface over `musa-project`.
//!
//! The CLI calls public facades only; it never chains compiler, renderer, or
//! engine calls itself (design roadmap §15.8). Everything below is argument
//! parsing, one `ProjectSession` call, and printing — if a subcommand here
//! ever needs to orchestrate, the orchestration belongs in `musa-project`.

use std::process::ExitCode;

use musa_project::{
    ExportArtifact, ExportRequest, MidiMode, ProjectCommand, ProjectSession, Realization, TransportRequest,
};

fn main() -> ExitCode {
    install_renderer();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("format") => cmd_format(args.get(1..).unwrap_or_default()),
        Some("check") => with_seed(args.get(1..).unwrap_or_default(), cmd_check),
        Some("explain") => cmd_explain(args.get(1..).unwrap_or_default()),
        Some("render") => with_seed(args.get(1..).unwrap_or_default(), cmd_render),
        Some("play") => cmd_play(args.get(1..).unwrap_or_default()),
        Some("kernel") => with_seed(args.get(1..).unwrap_or_default(), cmd_kernel),
        Some(other) if !other.starts_with('-') => {
            eprintln!("error: `{other}` is not a musa command");
            eprintln!();
            print_usage();
            ExitCode::FAILURE
        }
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

/// Fix how diagnostics are drawn, rather than letting the terminal decide.
///
/// miette's defaults adapt to the width of whoever is watching, which is the
/// right instinct for a human and the wrong one for a golden file: the same
/// piece would render differently in CI and in a narrow pane. A fixed width
/// and a colour decision made from `is_terminal` give one drawing that is
/// legible at a glance and stable under `insta`.
fn install_renderer() {
    let colour = std::io::IsTerminal::is_terminal(&std::io::stderr());
    let _installed = miette::set_hook(Box::new(move |_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(false)
                .unicode(true)
                .color(colour)
                .context_lines(1)
                .width(96)
                .build(),
        )
    }));
}

fn print_usage() {
    println!("musa — notation-first music language and workbench");
    println!();
    println!("Commands:");
    println!("  musa check <file.musa>…                every problem in a piece, with its place");
    println!("  musa explain <code>                    the rule behind a diagnostic code");
    println!("  musa format <file.musa> [--check]      format in place (--check to fail instead)");
    println!("  musa render <file.musa> --to <target>  mei | lilypond | musicxml | midi | wav");
    println!("      --to plan | performance              the debug dumps, to stdout");
    println!("      --mode score | performance           for --to midi (default: score)");
    println!("      -o <path>                            where to write it (`-` for stdout)");
    println!("  musa play <file.musa> [--loop]         live playback through the audio engine");
    println!("  musa kernel <file.musa> [--normalized] print the piece as kernel interchange text");
    println!("  musa kernel --check <file.kernel>      parse, check, and evaluate kernel text");
    println!("  --seed <n>  on check, render and kernel: which performance to compile");
}

/// Run a subcommand that compiles, with `--seed` already read.
fn with_seed(args: &[String], run: fn(&[String], &Realization) -> ExitCode) -> ExitCode {
    let (realization, rest) = take_seed(args);
    run(&rest, &realization)
}

/// Open a project under `realization`, or report why not.
fn open(path: &str, realization: &Realization) -> Result<ProjectSession, ExitCode> {
    let mut session = ProjectSession::open(path).map_err(|error| {
        eprintln!("error: {error}");
        ExitCode::FAILURE
    })?;
    // A determinate piece is unaffected, so this costs one recompile of a
    // piece that has nothing to decide and buys not having two open paths.
    let _realized = session.realize(realization.clone());
    Ok(session)
}

/// Take `--seed N` out of `args`, leaving the subcommand's own arguments.
///
/// Which performance to compile (`docs/kernel/11-realization.md`). Absent, the
/// realization is `deterministic()` — and a piece that leaves nothing open
/// compiles to the same bytes under every seed, which is a test rather than a
/// claim. It is removed here so no subcommand's parser has to know the flag
/// takes a value and mistake the number for a file.
fn take_seed(args: &[String]) -> (Realization, Vec<String>) {
    let mut realization = Realization::deterministic();
    let mut rest = Vec::with_capacity(args.len());
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        if arg == "--seed" {
            if let Some(seed) = args.get(index.saturating_add(1)).and_then(|text| text.parse().ok()) {
                realization = Realization::seeded(seed);
            }
            index = index.saturating_add(2);
            continue;
        }
        rest.push(arg.clone());
        index = index.saturating_add(1);
    }
    (realization, rest)
}

/// `musa render <file> --to <target> [-o <path>]`
fn cmd_render(args: &[String], realization: &Realization) -> ExitCode {
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
    let session = match open(path, realization) {
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
    // What the target could not say, said once — before the bytes go
    // anywhere, so it is read whether the file is written or piped.
    for warning in artifact.warnings() {
        eprintln!("warning: {warning}");
    }
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
fn cmd_kernel(args: &[String], realization: &Realization) -> ExitCode {
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
                println!(
                    "  realization: {}",
                    report.realization.as_deref().unwrap_or("not stated by this file")
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {path}: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let session = match open(path, realization) {
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
    let mut session = match open(path, &Realization::deterministic()) {
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
    let mut session = match open(path, &Realization::deterministic()) {
        Ok(session) => session,
        Err(code) => return code,
    };
    // `--check` asks rather than edits: an edit would be autosaved, and a
    // check that leaves a `.recovery` copy behind is not a check.
    if check_only {
        if session.is_formatted() {
            return ExitCode::SUCCESS;
        }
        eprintln!("{path}: not formatted");
        return ExitCode::FAILURE;
    }
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
///
/// Every field of the session's diagnostic reaches the terminal: the code in
/// the header, each label under the line it points at, the help below the
/// snippet. The note and the fixes are printed after the report rather than
/// inside it, because miette has one advice slot and they are three different
/// kinds of advice — what to do, why the rule exists, and the exact edit.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct CliDiagnostic {
    code: String,
    message: String,
    help: Option<String>,
    src: miette::NamedSource<String>,
    labels: Vec<miette::LabeledSpan>,
    error: bool,
}

impl miette::Diagnostic for CliDiagnostic {
    fn code(&self) -> Option<Box<dyn std::fmt::Display + '_>> {
        Some(Box::new(&self.code))
    }

    fn help(&self) -> Option<Box<dyn std::fmt::Display + '_>> {
        let help = self.help.as_ref()?;
        Some(Box::new(help))
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.src)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        if self.labels.is_empty() {
            return None;
        }
        Some(Box::new(self.labels.iter().cloned()))
    }

    fn severity(&self) -> Option<miette::Severity> {
        if self.error {
            Some(miette::Severity::Error)
        } else {
            Some(miette::Severity::Warning)
        }
    }
}

/// Render one diagnostic and everything hanging off it.
fn report(path: &str, source: &str, diagnostic: &musa_project::Diagnostic) {
    let labels = diagnostic
        .labels
        .iter()
        .map(|label| {
            miette::LabeledSpan::new_with_span(
                Some(label.text.clone()),
                miette::SourceSpan::from((
                    miette::SourceOffset::from(usize::try_from(label.span.start).unwrap_or(0)),
                    usize::try_from(label.span.end.saturating_sub(label.span.start)).unwrap_or(0),
                )),
            )
        })
        .collect();
    let rendered = CliDiagnostic {
        code: diagnostic.code.clone(),
        message: diagnostic.message.clone(),
        help: diagnostic.help.clone(),
        src: miette::NamedSource::new(path, source.to_owned()),
        labels,
        error: diagnostic.severity == musa_project::Severity::Error,
    };
    eprintln!("{:?}", miette::Report::new(rendered));
    if let Some(note) = diagnostic.note.as_deref() {
        eprintln!("  note: {note}");
    }
    for fix in &diagnostic.fixes {
        eprintln!("  fix: {}", fix.title);
        for edit in &fix.edits {
            let at = position_of(source, edit.span.start);
            match (edit.replacement.is_empty(), edit.span.start == edit.span.end) {
                (true, _) => eprintln!("       {at}: delete"),
                (false, true) => eprintln!("       {at}: insert `{}`", edit.replacement),
                (false, false) => eprintln!("       {at}: write `{}`", edit.replacement),
            }
        }
    }
    if diagnostic.note.is_some() || !diagnostic.fixes.is_empty() {
        eprintln!();
    }
}

/// `line:column` for a byte offset, for the fix footer.
///
/// The labels get theirs from miette, which has the source; this is the one
/// place the CLI states a position itself.
fn position_of(source: &str, offset: u32) -> String {
    let at = usize::try_from(offset).unwrap_or(0).min(source.len());
    let head = source.get(..at).unwrap_or(source);
    let line = head.matches('\n').count().saturating_add(1);
    let column = head
        .rsplit_once('\n')
        .map_or(head, |(_, tail)| tail)
        .chars()
        .count()
        .saturating_add(1);
    format!("{line}:{column}")
}

/// How many problems, of which kinds.
#[derive(Default)]
struct Tally {
    errors: u32,
    warnings: u32,
}

impl Tally {
    fn count(&mut self, diagnostic: &musa_project::Diagnostic) {
        match diagnostic.severity {
            musa_project::Severity::Error => self.errors = self.errors.saturating_add(1),
            musa_project::Severity::Warning => self.warnings = self.warnings.saturating_add(1),
        }
    }

    /// `3 problems (2 errors, 1 warning)`, or nothing when the run was clean.
    ///
    /// Printed even when the run fails, because "how much is left" is the
    /// first thing a reader wants after a wall of output.
    fn summary(&self) -> Option<String> {
        let total = self.errors.saturating_add(self.warnings);
        if total == 0 {
            return None;
        }
        let mut parts = Vec::new();
        if self.errors > 0 {
            parts.push(format!("{} {}", self.errors, plural(self.errors, "error")));
        }
        if self.warnings > 0 {
            parts.push(format!("{} {}", self.warnings, plural(self.warnings, "warning")));
        }
        Some(format!("{total} {} ({})", plural(total, "problem"), parts.join(", ")))
    }
}

fn plural(count: u32, word: &str) -> String {
    if count == 1 {
        word.to_owned()
    } else {
        format!("{word}s")
    }
}

/// `musa check <file>...` — full semantic check.
fn cmd_check(args: &[String], realization: &Realization) -> ExitCode {
    let mut status = ExitCode::SUCCESS;
    let mut files: u32 = 0;
    let mut tally = Tally::default();
    for path in args.iter().filter(|arg| !arg.starts_with("--")) {
        files = files.saturating_add(1);
        if cmd_check_one(path, realization, &mut tally) == ExitCode::FAILURE {
            status = ExitCode::FAILURE;
        }
    }
    if files == 0 {
        eprintln!("error: check needs a file");
        return ExitCode::FAILURE;
    }
    if let Some(summary) = tally.summary() {
        eprintln!("{summary}");
    }
    status
}

fn cmd_check_one(path: &str, realization: &Realization, tally: &mut Tally) -> ExitCode {
    let session = match open(path, realization) {
        Ok(session) => session,
        Err(code) => return code,
    };
    let snapshot = session.snapshot();
    for diagnostic in snapshot.diagnostics() {
        tally.count(diagnostic);
        report(path, snapshot.source(), diagnostic);
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

/// `musa explain <code>` — the long form of a diagnostic code.
fn cmd_explain(args: &[String]) -> ExitCode {
    let Some(code) = args.first() else {
        println!("Diagnostic codes:");
        for code in musa_project::codes() {
            println!("  {code}");
        }
        println!();
        println!("Run `musa explain <code>` for the rule behind one.");
        return ExitCode::SUCCESS;
    };
    match musa_project::explain(code) {
        Some(explanation) => {
            println!("{code}");
            println!();
            println!("{explanation}");
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("error: `{code}` is not a diagnostic code");
            eprintln!("       run `musa explain` for the list");
            ExitCode::FAILURE
        }
    }
}
