//! Thin command-line interface over `musa-project`.
//!
//! The CLI calls public facades only; it never chains compiler, renderer, or
//! engine calls itself (design roadmap §15.8). Everything below is argument
//! parsing, one `ProjectSession` call, and printing — if a subcommand here
//! ever needs to orchestrate, the orchestration belongs in `musa-project`.

mod ignore;

use std::process::ExitCode;

use musa_project::{
    AnalysisKind, AnalysisRequest, AnalysisScope, DawExportOptions, DawProfile, ExportArtifact, ExportRequest, Logging,
    MidiMode, MusicalTime, ProjectCommand, ProjectSession, Realization, TransportRequest, asset_inventory,
    fetch_packages, lock_assets, verify_packages,
};

fn main() -> ExitCode {
    install_renderer();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args = install_logging(args);
    // One span for the whole invocation, named by the subcommand, so every
    // compile, render, and device negotiation below it is filed under the
    // thing that was typed.
    let span = tracing::info_span!("musa", command = args.first().map_or("<none>", String::as_str));
    let _entered = span.enter();
    match args.first().map(String::as_str) {
        Some("format") => cmd_format(args.get(1..).unwrap_or_default()),
        Some("check") => with_seed(args.get(1..).unwrap_or_default(), cmd_check),
        Some("explain") => cmd_explain(args.get(1..).unwrap_or_default()),
        Some("render") => with_seed(args.get(1..).unwrap_or_default(), cmd_render),
        Some("play") => cmd_play(args.get(1..).unwrap_or_default()),
        Some("events") => with_seed(args.get(1..).unwrap_or_default(), cmd_events),
        Some("analyze") => with_seed(args.get(1..).unwrap_or_default(), cmd_analyze),
        Some("assets") => cmd_assets(args.get(1..).unwrap_or_default()),
        Some("fetch") => cmd_fetch(args.get(1..).unwrap_or_default()),
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
    println!("  musa check <file.musa>… [--fix]         every problem in a piece, with its place");
    println!("      --fix                                  apply warnings' certain fixes in place");
    println!("  musa explain <code>                    the rule behind a diagnostic code");
    println!("  musa format [<file|folder>…] [--check] format in place; no path means here");
    println!("      --diff                               print the diff instead of writing");
    println!("      --insert-bars                        insert only compiler-proved `|` assertions");
    println!("      -f                                   format even what `.musaignore` excludes");
    println!("  musa render <file.musa> --to <target>  mei | lilypond | musicxml | midi | wav");
    println!("      --to plan | performance              the debug dumps, to stdout");
    println!("      --to daw --profile logic | garageband a whole bundle, into a directory");
    println!("          --replace                        replace a bundle already sitting there");
    println!("      --mode score | performance           for --to midi (default: score)");
    println!("      -o <path>                            where to write it (`-` for stdout)");
    println!("  musa play <file.musa> [--loop]         live playback through the audio engine");
    println!("  musa events <file.musa> [--normalized] print the piece as events interchange text");
    println!("  musa events --check <file.musa.events> parse, check, and evaluate events text");
    println!("      a file whose first line is `% musa-events-1` is Musa too: check, format");
    println!("      and render read one wherever they read a piece");
    println!("  musa analyze <file.musa> --kind <kind> observe a score without changing it");
    println!("      --kind facts | chords | tonal | cadences | voice-leading | counterpoint");
    println!("      --profile <profile>   which style's rules to read by, for the last two kinds");
    println!("      --cantus <voice>      which voice is the cantus firmus, for a species profile");
    println!("      --format text | json                 how to print the report (default: text)");
    println!("      --part <name> [--voice <name>]       read one part, or one of its voices");
    println!("      --from <n> --to <n>                  read only [from, to), in whole notes");
    println!("      --segmentation attacks | beats | harmony-lane    what sounds together");
    println!("      --key \"<tonic> <mode>\"               read it in the key you hear");
    println!("  musa assets list <project|piece>          list immutable asset facts");
    println!("  musa assets verify <project|piece>        verify the offline locked closure");
    println!("  musa assets lock <project|piece>          explicitly rewrite local asset locks");
    println!("  musa fetch <project|piece>                fetch exact pinned source packages");
    println!("      --locked                              verify lock/cache only; no network or writes");
    println!("  --seed <n>  on check, render and events: which performance to compile");
    println!();
    println!("Everywhere:");
    println!("  -v, -vv, -vvv   say more about what musa is doing, on stderr");
    println!("  -q              say only what failed");
    println!("  MUSA_LOG        a filter, in place of the dial: `MUSA_LOG=musa_compiler=debug`");
}

/// Materialize the project's exact package graph and rewrite its package lock.
fn cmd_fetch(args: &[String]) -> ExitCode {
    let locked = args.iter().any(|argument| argument == "--locked");
    let paths: Vec<&str> = args
        .iter()
        .filter(|argument| argument.as_str() != "--locked")
        .map(String::as_str)
        .collect();
    let [path] = paths.as_slice() else {
        eprintln!("error: fetch needs a project folder or piece");
        return ExitCode::FAILURE;
    };
    let result = if locked {
        verify_packages(path)
    } else {
        fetch_packages(path)
    };
    match result {
        Ok(count) => {
            if locked {
                eprintln!("verified {count} exact package root(s) offline");
            } else {
                eprintln!("fetched {count} exact package(s)");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Inspect or explicitly regenerate a project's immutable local asset lock.
fn cmd_assets(args: &[String]) -> ExitCode {
    let Some(action) = args.first().map(String::as_str) else {
        eprintln!("error: assets needs an action (list | verify | lock)");
        return ExitCode::FAILURE;
    };
    let Some(path) = args.get(1).map(String::as_str) else {
        eprintln!("error: assets {action} needs a project folder or piece");
        return ExitCode::FAILURE;
    };
    let inventory = match action {
        "list" | "verify" => asset_inventory(path),
        "lock" => lock_assets(path),
        other => {
            eprintln!("error: `{other}` is not an asset action (list | verify | lock)");
            return ExitCode::FAILURE;
        }
    };
    let inventory = match inventory {
        Ok(inventory) => inventory,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    for fact in inventory.facts() {
        println!(
            "{}\t{}\t{}\t{}",
            fact.status,
            fact.kind.map_or_else(|| "unknown".to_owned(), |kind| kind.to_string()),
            fact.digest.as_deref().unwrap_or("-"),
            fact.path
        );
        if let Some(detail) = fact.detail.as_deref() {
            eprintln!("{}: {detail}", fact.path);
        }
    }
    if action == "lock" && inventory.is_verified() {
        eprintln!("locked {} asset(s)", inventory.facts().len());
    }
    if inventory.is_verified() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Turn musa's logs on, and leave the subcommand its own arguments.
///
/// The verbosity flags are taken out before dispatch, exactly as `--seed` is
/// and for the same reason: every subcommand accepts them, and none of them
/// should have to know that. They are read from the whole command line rather
/// than from a fixed position, so `musa -v check x.musa` and `musa check
/// x.musa -v` both work — a person reaching for more detail reaches for it
/// wherever the cursor is.
///
/// Logs go to stderr. `musa render -o -` and `musa events` write to stdout,
/// so a log line on that stream would corrupt a score.
fn install_logging(args: Vec<String>) -> Vec<String> {
    let mut verbosity: u8 = 0;
    let mut quiet = false;
    let mut rest = Vec::with_capacity(args.len());
    for arg in args {
        match arg.as_str() {
            "-q" | "--quiet" => quiet = true,
            "--verbose" => verbosity = verbosity.saturating_add(1),
            // `-vvv` is three of the same flag written once, which is the
            // convention everywhere else and is what a person types.
            other if repeated_v(other) > 0 => verbosity = verbosity.saturating_add(repeated_v(other)),
            _ => rest.push(arg),
        }
    }
    let _installed = Logging::new().verbosity(verbosity).quiet(quiet).install();
    rest
}

/// How many `v`s `arg` is, as in `-vv`; zero if it is anything else.
///
/// A separate question from "is this a flag" because `-v` and `--seed` and a
/// file called `-vx` all start with a dash, and only the first is a dial.
fn repeated_v(arg: &str) -> u8 {
    let Some(letters) = arg.strip_prefix('-') else { return 0 };
    if letters.is_empty() || !letters.bytes().all(|byte| byte == b'v') {
        return 0;
    }
    u8::try_from(letters.len()).unwrap_or(u8::MAX)
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
/// Which performance to compile (`docs/rules/events/11-realization.md`). Absent, the
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
    let mut profile = "logic";
    let mut replace = false;
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
            "--profile" => {
                profile = args.get(index.saturating_add(1)).map_or("logic", String::as_str);
                index = index.saturating_add(2);
            }
            "--replace" => {
                replace = true;
                index = index.saturating_add(1);
            }
            "-o" | "--output" => {
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
    // A bundle is a directory of several artifacts, not one; it leaves the
    // `ExportRequest` road here rather than pretending to be a file.
    if target == "daw" {
        return cmd_render_daw(path, profile, replace, output, realization);
    }
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

/// `musa render <file> --to daw --profile logic|garageband -o <dir>`
///
/// One directory, written whole or not at all. What is printed is the report
/// the project handed back: the files it wrote, and everything the target
/// formats could not say.
fn cmd_render_daw(
    path: &str,
    profile: &str,
    replace: bool,
    output: Option<&str>,
    realization: &Realization,
) -> ExitCode {
    let profile = match profile {
        "logic" => DawProfile::Logic,
        "garageband" => DawProfile::GarageBand,
        other => {
            eprintln!("error: --profile {other} is not a workstation (logic | garageband)");
            return ExitCode::FAILURE;
        }
    };
    let Some(output) = output else {
        eprintln!("error: --to daw writes a directory; say where with -o <dir>");
        return ExitCode::FAILURE;
    };
    let session = match open(path, realization) {
        Ok(session) => session,
        Err(code) => return code,
    };
    let mut options = DawExportOptions::new(profile);
    if replace {
        options = options.replacing();
    }
    let report = match session.export_daw_bundle(options, std::path::Path::new(output)) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error: {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    // The losses first, on stderr, so a piped file list stays a file list and
    // what the bundle could not carry is read even when it is.
    for loss in report.losses() {
        eprintln!("warning: {}: {}", loss.kind(), loss.message());
    }
    println!("{}", report.destination().display());
    for file in report.files() {
        println!("  {} ({} bytes)", file.path(), file.bytes());
    }
    ExitCode::SUCCESS
}

/// `musa analyze <file.musa> --kind <kind> [--format text|json] [--part <name>
/// [--voice <name>]] [--from <n> --to <n>]`
///
/// Reading only. The exit code says whether the *request* could be answered,
/// never what the report contains: an analysis is not a check, and a piece
/// with nothing in the window is a piece with nothing in the window
/// (`docs/rules/language/07-analysis.md` §1).
fn cmd_analyze(args: &[String], realization: &Realization) -> ExitCode {
    let mut path: Option<&str> = None;
    let mut kind = "facts";
    let mut format = "text";
    let mut part: Option<&str> = None;
    let mut voice: Option<&str> = None;
    let mut segmentation = "attacks";
    let mut key: Option<&str> = None;
    let mut profile: Option<&str> = None;
    let mut cantus: Option<&str> = None;
    let mut from: Option<&str> = None;
    let mut to: Option<&str> = None;
    let mut index = 0;
    while index < args.len() {
        let Some(arg) = args.get(index).map(String::as_str) else {
            break;
        };
        match arg {
            "--kind" => {
                kind = args.get(index.saturating_add(1)).map_or("facts", String::as_str);
                index = index.saturating_add(2);
            }
            "--format" => {
                format = args.get(index.saturating_add(1)).map_or("text", String::as_str);
                index = index.saturating_add(2);
            }
            "--part" => {
                part = args.get(index.saturating_add(1)).map(String::as_str);
                index = index.saturating_add(2);
            }
            "--voice" => {
                voice = args.get(index.saturating_add(1)).map(String::as_str);
                index = index.saturating_add(2);
            }
            "--segmentation" => {
                segmentation = args.get(index.saturating_add(1)).map_or("attacks", String::as_str);
                index = index.saturating_add(2);
            }
            "--key" => {
                key = args.get(index.saturating_add(1)).map(String::as_str);
                index = index.saturating_add(2);
            }
            "--profile" => {
                profile = args.get(index.saturating_add(1)).map(String::as_str);
                index = index.saturating_add(2);
            }
            "--cantus" => {
                cantus = args.get(index.saturating_add(1)).map(String::as_str);
                index = index.saturating_add(2);
            }
            "--from" => {
                from = args.get(index.saturating_add(1)).map(String::as_str);
                index = index.saturating_add(2);
            }
            "--to" => {
                to = args.get(index.saturating_add(1)).map(String::as_str);
                index = index.saturating_add(2);
            }
            other => {
                path = Some(other);
                index = index.saturating_add(1);
            }
        }
    }
    let Some(path) = path else {
        eprintln!("error: analyze needs a file");
        return ExitCode::FAILURE;
    };
    let Some(kind) = AnalysisKind::named(kind) else {
        eprintln!(
            "error: `{kind}` is not an analysis ({})",
            AnalysisKind::ALL
                .iter()
                .map(|kind| kind.as_str())
                .collect::<Vec<&str>>()
                .join(" | ")
        );
        return ExitCode::FAILURE;
    };
    let Some(segmentation) = musa_project::Segmentation::named(segmentation) else {
        eprintln!("error: `{segmentation}` is not a segmentation (attacks | beats | harmony-lane)");
        return ExitCode::FAILURE;
    };
    let mut request = AnalysisRequest::new(kind).segmenting(segmentation);
    if let Some(written) = profile {
        let Some(parsed) = musa_project::AnalysisProfile::named(written) else {
            eprintln!(
                "error: `{written}` is not a profile ({})",
                musa_project::AnalysisProfile::ALL
                    .iter()
                    .map(|profile| profile.as_str())
                    .collect::<Vec<&str>>()
                    .join(" | ")
            );
            return ExitCode::FAILURE;
        };
        request = request.under(parsed);
    }
    if let Some(name) = cantus {
        request = request.designating(name.to_owned());
    }
    if let Some(written) = key {
        let Some(parsed) = musa_project::Key::parse(written) else {
            eprintln!("error: `{written}` is not a key: write `c major` or `a minor`");
            return ExitCode::FAILURE;
        };
        request = request.in_key(parsed);
    }
    request = match (part, voice) {
        (None, None) => request,
        (Some(part), None) => request.scoped(AnalysisScope::Part(part.to_owned())),
        (Some(part), Some(voice)) => request.scoped(AnalysisScope::Voice {
            part: part.to_owned(),
            voice: voice.to_owned(),
        }),
        // A voice is a voice *of* a part, so naming one alone names nothing.
        (None, Some(_)) => {
            eprintln!("error: --voice needs --part: a voice is named within a part");
            return ExitCode::FAILURE;
        }
    };
    match (from, to) {
        (None, None) => {}
        (Some(from), Some(to)) => {
            let (Some(from), Some(to)) = (MusicalTime::parse(from), MusicalTime::parse(to)) else {
                eprintln!("error: --from and --to are whole notes, written `3` or `7/8`");
                return ExitCode::FAILURE;
            };
            request = request.within(from, to);
        }
        _ => {
            eprintln!("error: --from and --to go together: a window has two ends");
            return ExitCode::FAILURE;
        }
    }
    let session = match open(path, realization) {
        Ok(session) => session,
        Err(code) => return code,
    };
    let facts = match session.analyze(&request) {
        Ok(facts) => facts,
        Err(error) => {
            eprintln!("error: {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match format {
        "json" => println!("{}", facts.to_json()),
        "text" => print_analysis(&facts),
        other => {
            eprintln!("error: --format {other} is not a format (text | json)");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// Print a report the way a person reads one: what ran, what it assumed, and
/// one line per finding in the report's own order.
fn print_analysis(facts: &musa_project::AnalysisFacts) {
    println!("{} — {}", facts.kind, facts.method);
    if let Some(ref profile) = facts.profile {
        println!("  reading by {profile}");
    }
    for assumption in &facts.assumptions {
        println!("  assuming {assumption}");
    }
    println!();
    for finding in &facts.findings {
        let place = format!("{}:{}", finding.bar, fraction(finding.beat));
        let where_seen = match finding.evidence {
            musa_project::EvidenceFacts::Event(ref note) => {
                format!("{}/{}, line {}", note.part, note.voice, note.line)
            }
            musa_project::EvidenceFacts::Passage { ref notes, .. } => match notes.first() {
                Some(note) => format!("{} notes from line {}", notes.len(), note.line),
                None => String::new(),
            },
            musa_project::EvidenceFacts::Annotation { line, .. } => format!("line {line}"),
            musa_project::EvidenceFacts::InForce => String::new(),
        };
        let row = format!(
            "  {place:>7}  {:<9} {:<16} {:<44} {where_seen}",
            finding.standing, finding.code, finding.summary,
        );
        println!("{}", row.trim_end());
        // Only the criteria that failed. A reading is argued with where it is
        // weak, and printing the satisfied ones too would bury that under the
        // ones nobody disputes.
        for ground in finding.grounds.iter().filter(|ground| !ground.satisfied) {
            println!("             but not: {} ({})", ground.criterion, ground.cites);
        }
        // The strength, always, and separately from the departure: the motion
        // is a fact about the notes, and how firmly a style holds it is the
        // style's claim rather than the music's.
        if let Some(ref rule) = finding.rule {
            println!("             {} — {} ({})", rule.strength, rule.states, rule.cites);
        }
    }
    println!();
    let count = u32::try_from(facts.findings.len()).unwrap_or(u32::MAX);
    println!("{count} {}", plural(count, "finding"));
}

/// `1` rather than `1/1`, because a beat is usually a whole number and
/// `1/1` reads as a mistake.
fn fraction(value: musa_project::Fraction) -> String {
    if value.denominator == 1 {
        value.numerator.to_string()
    } else {
        format!("{}/{}", value.numerator, value.denominator)
    }
}

/// `musa events <file.musa> [--normalized] [-o <path>]` /
/// `musa events --check <file.musa.events>`
///
/// One direction only: events text is a projection of a piece, and a
/// `.musa.events` file is never read back into a document (AGENTS.md — the source
/// is canonical). `--check` is a reader, not an importer.
fn cmd_events(args: &[String], realization: &Realization) -> ExitCode {
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
        eprintln!("error: events needs a file");
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
        return match musa_project::check_events(&text) {
            Ok(report) => {
                println!(
                    "{path}: ok — piece {:?}, {} occurrences, duration {}",
                    report.name, report.occurrences, report.duration
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
    let request = ExportRequest::Events { normalized };
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

/// What formatting asks of one file — the three things the flags choose between.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Formatting {
    /// Rewrite the file and save it.
    Write,
    /// Say whether it is canonical, and change nothing.
    Check,
    /// Print what `Write` would change, and change nothing.
    Diff,
}

/// What formatting one file found — the three things the summary counts.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Formatted {
    /// Already canonical.
    Unchanged,
    /// Rewritten, or — under `--check` and `--diff` — waiting to be.
    Changed,
    /// Could not be opened or saved. Already reported.
    Failed,
}

/// `musa format [<file|folder>…] [--check | --diff] [-f]`
///
/// A folder is formatted whole and no argument means this one, the way `ruff
/// format`, `black` and `cargo fmt` work: the file worth formatting is rarely
/// the file you happen to have open, and a formatter you must aim one file at
/// a time is one that runs on a subset of the project forever.
fn cmd_format(args: &[String]) -> ExitCode {
    let formatting = if args.iter().any(|arg| arg == "--diff") {
        Formatting::Diff
    } else if args.iter().any(|arg| arg == "--check") {
        Formatting::Check
    } else {
        Formatting::Write
    };
    let forced = args.iter().any(|arg| arg == "-f" || arg == "--force");
    let insert_bars = args.iter().any(|arg| arg == "--insert-bars");
    let mut arguments: Vec<&str> = args
        .iter()
        .filter(|arg| !arg.starts_with("--") && *arg != "-f")
        .map(String::as_str)
        .collect();
    if arguments.is_empty() {
        arguments.push(".");
    }
    let mut walked = Walked::default();
    for argument in arguments {
        if let Err(error) = walked.extend(argument, forced) {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    }
    if walked.paths.is_empty() {
        if walked.ignored > 0 {
            eprintln!("error: every `.musa` file found is excluded by `.musaignore`");
        } else {
            eprintln!("error: no `.musa` file to format");
        }
        return ExitCode::FAILURE;
    }
    let (mut changed, mut unchanged, mut failed) = (0u32, 0u32, 0u32);
    for path in &walked.paths {
        match format_one(path, formatting, insert_bars) {
            Formatted::Changed => changed = changed.saturating_add(1),
            Formatted::Unchanged => unchanged = unchanged.saturating_add(1),
            Formatted::Failed => failed = failed.saturating_add(1),
        }
    }
    if walked.paths.len() > 1 || walked.ignored > 0 {
        eprintln!(
            "{}",
            format_summary(formatting, changed, unchanged, failed, walked.ignored)
        );
    }
    // Under `--check` and `--diff` a difference is the answer, and the answer
    // is no.
    if failed > 0 || (formatting != Formatting::Write && changed > 0) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// One file, formatted or asked about.
fn format_one(path: &str, formatting: Formatting, insert_bars: bool) -> Formatted {
    let Ok(mut session) = open(path, &Realization::deterministic()) else {
        return Formatted::Failed;
    };
    // A file that does not parse is not formatted. The tree is lossless, so
    // the formatter could produce *something* — but the something is a guess
    // at what half-written text meant, written over the text its author was
    // in the middle of. The rule for fixes is the rule here: a file
    // that does not compile is a conversation, not a draft. It matters more
    // now that one command formats a whole folder at once.
    if session.snapshot().diagnostics().iter().any(unparsed) {
        eprintln!("error: {path}: does not parse, so it is left alone");
        return Formatted::Failed;
    }
    // `--check` and `--diff` ask rather than edit: an edit would be
    // autosaved, and a check that leaves a `.recovery` copy behind is not a
    // check. The diff is the check with its evidence attached — a formatter
    // you cannot preview is a formatter you cannot trust.
    if formatting != Formatting::Write {
        let formatted = if insert_bars {
            match session.barline_rewrite() {
                Ok(Some(rewrite)) => rewrite.source().to_owned(),
                Ok(None) => return Formatted::Unchanged,
                Err(blocker) => {
                    eprintln!("error: {path}: cannot insert bar lines: {blocker}");
                    return Formatted::Failed;
                }
            }
        } else {
            session.formatted_source()
        };
        let snapshot = session.snapshot();
        let source = snapshot.source();
        if formatted == source {
            return Formatted::Unchanged;
        }
        if formatting == Formatting::Diff {
            let diff = unified_diff(path, source, &formatted);
            if diff.is_empty() {
                // Lines compare equal; the difference is the final newline,
                // which a line diff has nowhere to put.
                println!("{path}: differs only in the final newline");
            } else {
                print!("{diff}");
            }
        } else {
            eprintln!("{path}: not formatted");
        }
        return Formatted::Changed;
    }
    let command = if insert_bars {
        ProjectCommand::InsertBarlines {
            revision: session.snapshot().revision(),
        }
    } else {
        ProjectCommand::Format
    };
    let update = match session.apply(command) {
        Ok(update) => update,
        Err(error) => {
            eprintln!("error: {path}: {error}");
            return Formatted::Failed;
        }
    };
    if !update.source_changed {
        return Formatted::Unchanged;
    }
    match session.apply(ProjectCommand::Save) {
        Ok(_) => {
            println!(
                "{path}: {}",
                if insert_bars { "inserted bar lines" } else { "formatted" }
            );
            Formatted::Changed
        }
        Err(error) => {
            eprintln!("error: {path}: {error}");
            Formatted::Failed
        }
    }
}

/// Whether a diagnostic says the text is not a piece yet.
fn unparsed(diagnostic: &musa_project::Diagnostic) -> bool {
    diagnostic.severity == musa_project::Severity::Error && diagnostic.code == "syntax"
}

/// The one line a run over many files ends with.
fn format_summary(formatting: Formatting, changed: u32, unchanged: u32, failed: u32, ignored: u32) -> String {
    let (verb, rest) = if formatting == Formatting::Write {
        ("formatted", "already formatted")
    } else {
        ("not formatted", "formatted")
    };
    // What was passed over is said out loud. A run that quietly skipped
    // fifteen files reads exactly like a run that had nothing to skip.
    let passed = if ignored > 0 {
        format!(", {ignored} {} ignored", plural(ignored, "path"))
    } else {
        String::new()
    };
    let unread = if failed > 0 {
        format!(", {failed} {} left alone", plural(failed, "file"))
    } else {
        String::new()
    };
    format!(
        "{changed} {} {verb}, {unchanged} {rest}{passed}{unread}",
        plural(changed, "file")
    )
}

/// What the arguments named: the files to format, and how many paths the
/// `.musaignore` took off the list.
#[derive(Default)]
struct Walked {
    paths: Vec<String>,
    ignored: u32,
}

impl Walked {
    /// Add what one argument names: the file itself, or the folder's files.
    ///
    /// A folder is walked, not opened as a project: formatting is a fact about
    /// text alone, so a file no manifest lists is still a file to format.
    /// Hidden entries, `target`, and symbolic links are passed over — a
    /// formatter that writes through a link edits a file nobody named — and so
    /// is whatever the nearest [`ignore::Ignore`] excludes.
    ///
    /// A file named directly is checked against the list too, and `forced`
    /// (`-f`) is the only way past it. An excluded file is excluded because its
    /// shape is a specification — a generator compares it byte for byte, or a
    /// diagnostic's snapshot pins its byte positions — and that is as true of
    /// the file a script names as of the file a walk finds. The list is still
    /// consulted here rather than in [`format_one`], so that what it passed
    /// over is counted once, in the run's summary.
    fn extend(&mut self, argument: &str, forced: bool) -> Result<(), String> {
        let root = std::path::Path::new(argument);
        if !root.is_dir() {
            // A bare `piece.musa` has an empty parent, and `.` is where it
            // sits. Probing `./piece.musa` keeps the lexical strip in
            // `excludes` working, which is why the folder is joined back on
            // rather than passed alongside.
            let named = root.parent().is_some_and(|parent| !parent.as_os_str().is_empty());
            let folder = if named { root.parent() } else { None };
            let folder = folder.unwrap_or_else(|| std::path::Path::new("."));
            let probe = if named { root.to_path_buf() } else { folder.join(root) };
            if !forced && ignore::Ignore::found_at(folder).excludes(&probe) {
                self.ignored = self.ignored.saturating_add(1);
                return Ok(());
            }
            self.paths.push(argument.to_owned());
            return Ok(());
        }
        let ignore = ignore::Ignore::found_at(root);
        let first = self.paths.len();
        self.walk(root, &ignore)
            .map_err(|error| format!("{argument}: {error}"))?;
        self.paths.get_mut(first..).unwrap_or_default().sort();
        Ok(())
    }

    fn walk(&mut self, folder: &std::path::Path, ignore: &ignore::Ignore) -> std::io::Result<()> {
        for entry in std::fs::read_dir(folder)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') || name == "target" {
                continue;
            }
            let kind = entry.file_type()?;
            let source = kind.is_file() && is_document(&name);
            if !kind.is_dir() && !source {
                continue;
            }
            if ignore.excludes(&path) {
                // A folder counts once, as the one path the list named. Its
                // contents are not walked to be counted more precisely: an
                // ignored folder is ignored, which is the point of naming it.
                self.ignored = self.ignored.saturating_add(1);
                continue;
            }
            if kind.is_dir() {
                self.walk(&path, ignore)?;
            } else {
                let shown = path.display().to_string();
                self.paths.push(shown.strip_prefix("./").unwrap_or(&shown).to_owned());
            }
        }
        Ok(())
    }
}

/// Whether a walk should treat this file name as a musa document.
///
/// Both alternatives (`docs/rules/language/01-surface.md` §7), because both are
/// documents a bulk `musa format` is responsible for: a `.musa.events` file
/// left out of the walk is a file `--check` calls clean and a later edit makes
/// dirty without anything noticing.
///
/// Matched on the whole name rather than on `Path::extension`, which reads
/// `twinkle.musa.events` as a `events` file and would need the double
/// extension taken apart by hand to say otherwise.
fn is_document(name: &std::ffi::OsStr) -> bool {
    let name = name.to_string_lossy();
    name.ends_with(".musa") || name.ends_with(".musa.events")
}

/// A diagnostic rendered with source context by miette.
///
/// Every field of the session's diagnostic reaches the terminal: the code in
/// the header, each label under the line it points at, the help below the
/// snippet. The note and the fixes are printed after the report rather than
/// inside it, because miette has one advice slot and they are three different
/// kinds of advice — what to do, why the rule exists, and the exact edit.
///
/// A cause is one of these too, nested under the report it caused, carrying
/// its *own* source. That is what puts a caret in the adapter module's file
/// underneath a diagnostic about the import that named it — and it is why
/// `related` exists rather than a longer message.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct CliDiagnostic {
    code: String,
    message: String,
    help: Option<String>,
    src: miette::NamedSource<String>,
    labels: Vec<miette::LabeledSpan>,
    error: bool,
    causes: Vec<Self>,
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

    fn related(&self) -> Option<Box<dyn Iterator<Item = &dyn miette::Diagnostic> + '_>> {
        if self.causes.is_empty() {
            return None;
        }
        Some(Box::new(self.causes.iter().map(|cause| {
            let cause: &dyn miette::Diagnostic = cause;
            cause
        })))
    }
}

/// Render one diagnostic and everything hanging off it.
///
/// The snapshot is here for the causes: a cause names a document this piece
/// imports, and the snapshot is what holds that document's text
/// ([`ProjectSnapshot::cause_source`](musa_project::ProjectSnapshot::cause_source)).
fn report(path: &str, snapshot: &musa_project::ProjectSnapshot<'_>, diagnostic: &musa_project::Diagnostic) {
    let source = snapshot.source();
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
        causes: diagnostic
            .causes
            .iter()
            .map(|cause| caused(cause, snapshot.cause_source(&cause.document)))
            .collect(),
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
    // A cause has no fixes by construction — an edit against a file the
    // composer cannot see is worse than none — so its footer is the note
    // alone, named by the place it is about. The document *and* the position,
    // because two faults in one module are the ordinary case and the document
    // alone would leave a reader guessing which report each note follows.
    for cause in &diagnostic.causes {
        if let Some(note) = cause.note.as_deref() {
            eprintln!("  note ({}): {note}", where_of(cause));
        }
    }
    let footer = diagnostic.note.is_some()
        || !diagnostic.fixes.is_empty()
        || diagnostic.causes.iter().any(|cause| cause.note.is_some());
    if footer {
        eprintln!();
    }
}

/// One cause as a nested miette report over the document it is about.
///
/// Without that document's text there is no snippet to draw, so the labels are
/// dropped and the report is the words alone: a caret over a source miette does
/// not have would land in the composer's file, which is the one thing the whole
/// mechanism exists to prevent.
fn caused(cause: &musa_project::Cause, source: Option<&str>) -> CliDiagnostic {
    let text = source.unwrap_or_default();
    let labels = source
        .map(|source| {
            cause
                .labels
                .iter()
                .filter_map(|label| {
                    let start = offset_of(source, label.at?)?;
                    let end = offset_of(source, label.to?)?.max(start);
                    Some(miette::LabeledSpan::new_with_span(
                        Some(label.text.clone()),
                        miette::SourceSpan::from((miette::SourceOffset::from(start), end.saturating_sub(start))),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    CliDiagnostic {
        code: cause.code.clone(),
        message: cause.message.clone(),
        help: cause.help.clone(),
        src: miette::NamedSource::new(&cause.document, text.to_owned()),
        labels,
        error: true,
        // One level. The only thing that produces a cause is reading an
        // adapter module, and a module may not import.
        causes: Vec::new(),
    }
}

/// Where a cause is, for its note's footer line: the document, and the place
/// within it when the fault has one.
///
/// The whole resolved key rather than its last segment, because a terminal is
/// not a list of four-line entries and the reader may be about to open the
/// file. A fault that is about the module as a whole — one that does not parse
/// — has no position and says the document alone.
fn where_of(cause: &musa_project::Cause) -> String {
    let at = cause
        .labels
        .iter()
        .find(|label| label.primary)
        .or_else(|| cause.labels.first())
        .and_then(|label| label.at);
    at.map_or_else(
        || cause.document.clone(),
        |at| format!("{} {}:{}", cause.document, at.line, at.column),
    )
}

/// The byte offset of a 1-based line and character column in `source`.
///
/// The inverse of [`position_of`], and it counts characters for the same
/// reason that one does: a column is what a reader counts across the line,
/// whatever the characters cost to store.
fn offset_of(source: &str, position: musa_project::Position) -> Option<usize> {
    let wanted = usize::try_from(position.line).ok()?.checked_sub(1)?;
    let column = usize::try_from(position.column).ok()?.checked_sub(1)?;
    let mut start = 0usize;
    for (number, line) in source.split_inclusive('\n').enumerate() {
        if number == wanted {
            let within = line
                .char_indices()
                .nth(column)
                .map_or_else(|| line.len(), |(offset, _)| offset);
            return start.checked_add(within);
        }
        start = start.checked_add(line.len())?;
    }
    None
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

/// `musa check <file|folder>...` — full semantic check.
fn cmd_check(args: &[String], realization: &Realization) -> ExitCode {
    let fix = args.iter().any(|arg| arg == "--fix");
    let mut status = ExitCode::SUCCESS;
    let mut files: u32 = 0;
    let mut tally = Tally::default();
    for argument in args.iter().filter(|arg| !arg.starts_with("--")) {
        let listed = match expand(argument) {
            Ok(listed) => listed,
            Err(code) => {
                status = code;
                continue;
            }
        };
        for path in listed {
            files = files.saturating_add(1);
            if cmd_check_one(&path, realization, &mut tally, fix) == ExitCode::FAILURE {
                status = ExitCode::FAILURE;
            }
        }
    }
    if files == 0 {
        eprintln!("error: check needs a file or a project folder");
        return ExitCode::FAILURE;
    }
    if let Some(summary) = tally.summary() {
        eprintln!("{summary}");
    }
    status
}

/// The files one argument names: itself, or everything a project folder holds.
///
/// The order is the project's own — the manifest's running order, then its
/// material — so `musa check` on an album reads it the way the album is meant
/// to be read, and the material it all rests on is checked too. A folder that
/// holds no piece is not a project, and says so.
fn expand(argument: &str) -> Result<Vec<String>, ExitCode> {
    let root = std::path::Path::new(argument);
    if !root.is_dir() {
        return Ok(vec![argument.to_owned()]);
    }
    let mut project = musa_project::Project::open(root).map_err(|error| {
        eprintln!("error: {error}");
        ExitCode::FAILURE
    })?;
    let snapshot = project.snapshot();
    let contents = snapshot.contents();
    Ok(contents
        .map(|contents| {
            contents
                .pieces
                .iter()
                .chain(&contents.material)
                .map(|entry| root.join(&entry.file).display().to_string())
                .collect()
        })
        .unwrap_or_default())
}

fn cmd_check_one(path: &str, realization: &Realization, tally: &mut Tally, fix: bool) -> ExitCode {
    let mut session = match open(path, realization) {
        Ok(session) => session,
        Err(code) => return code,
    };
    let (compiles, fix_edits) = {
        let snapshot = session.snapshot();
        for diagnostic in snapshot.diagnostics() {
            tally.count(diagnostic);
            report(path, &snapshot, diagnostic);
        }
        let edits = if fix { warning_fix_edits(&snapshot) } else { Vec::new() };
        (snapshot.compiles(), edits)
    };
    if !fix_edits.is_empty() {
        apply_warning_fixes(&mut session, path, fix_edits);
    }
    if compiles {
        println!("{path}: ok");
        // What a piece is filed under and what it reads: the two facts a
        // directory project adds, and the two a reader would otherwise have
        // to reconstruct from the `import` statements themselves.
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

/// Every warning's certain fix, as session edits.
///
/// Warnings only, and each fix is offered only because it is certain:
/// a file that does not compile is a conversation, not a draft, so
/// errors are reported and never rewritten.
fn warning_fix_edits(snapshot: &musa_project::ProjectSnapshot<'_>) -> Vec<musa_project::TextEdit> {
    snapshot
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.severity == musa_project::Severity::Warning)
        .filter_map(|diagnostic| diagnostic.fixes.first())
        .flat_map(|fix| {
            fix.edits
                .iter()
                .map(|edit| musa_project::TextEdit::new(edit.span, edit.replacement.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Apply the fixes in place, then save. The session's own edit command is
/// what applies them, which keeps the source canonical and the autosave
/// honest.
fn apply_warning_fixes(session: &mut ProjectSession, path: &str, edits: Vec<musa_project::TextEdit>) {
    let applied = edits.len();
    if let Err(error) = session.apply(ProjectCommand::ApplyEdits(edits)) {
        eprintln!("error: {path}: could not apply fixes: {error}");
        return;
    }
    if let Err(error) = session.apply(ProjectCommand::Save) {
        eprintln!("error: {path}: could not save fixes: {error}");
        return;
    }
    println!("{path}: applied {applied} fix(es)");
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

/// One line of a diff: kept, removed, or added.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DiffLine {
    /// In both texts (context).
    Kept,
    /// In the old text only.
    Removed,
    /// In the new text only.
    Added,
}

/// A unified diff of `before` into `after`, line by line, with three lines of
/// context.
///
/// The alignment is a longest-common-subsequence over lines: the formatter
/// only moves whitespace, so a minimal line alignment keeps a spacing change
/// a one-line hunk instead of a rewritten file. No crate in the dependency
/// lists (roadmap §15) does this, and the table is small — musa sources are
/// hundreds of lines, not millions.
fn unified_diff(path: &str, before: &str, after: &str) -> String {
    const CONTEXT: usize = 3;
    let old: Vec<&str> = before.lines().collect();
    let new: Vec<&str> = after.lines().collect();
    let width = new.len().saturating_add(1);
    let mut lengths = vec![0_u32; old.len().saturating_add(1).saturating_mul(width)];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            let value = if old.get(i) == new.get(j) {
                cell(&lengths, width, i.saturating_add(1), j.saturating_add(1)).saturating_add(1)
            } else {
                cell(&lengths, width, i.saturating_add(1), j).max(cell(&lengths, width, i, j.saturating_add(1)))
            };
            set_cell(&mut lengths, width, i, j, value);
        }
    }

    // Walk the table into lines, remembering each line's number in the text
    // it came from (1-based, as diff prints them; 0 means "not from there").
    let mut lines: Vec<(DiffLine, &str, usize, usize)> = Vec::new();
    let (mut i, mut j) = (0_usize, 0_usize);
    while let (Some(&old_line), Some(&new_line)) = (old.get(i), new.get(j)) {
        if old_line == new_line {
            lines.push((DiffLine::Kept, old_line, i.saturating_add(1), j.saturating_add(1)));
            i = i.saturating_add(1);
            j = j.saturating_add(1);
        } else if cell(&lengths, width, i.saturating_add(1), j) >= cell(&lengths, width, i, j.saturating_add(1)) {
            lines.push((DiffLine::Removed, old_line, i.saturating_add(1), 0));
            i = i.saturating_add(1);
        } else {
            lines.push((DiffLine::Added, new_line, 0, j.saturating_add(1)));
            j = j.saturating_add(1);
        }
    }
    while let Some(&old_line) = old.get(i) {
        lines.push((DiffLine::Removed, old_line, i.saturating_add(1), 0));
        i = i.saturating_add(1);
    }
    while let Some(&new_line) = new.get(j) {
        lines.push((DiffLine::Added, new_line, 0, j.saturating_add(1)));
        j = j.saturating_add(1);
    }

    let mut hunks = String::new();
    let mut cursor = 0_usize;
    while let Some(first_change) = lines
        .iter()
        .enumerate()
        .skip(cursor)
        .find(|(_, (kind, ..))| *kind != DiffLine::Kept)
        .map(|(index, _)| index)
    {
        let start = first_change.saturating_sub(CONTEXT);
        // Take in later changes while the context between them would touch.
        let mut last_change = first_change;
        let mut scan = first_change.saturating_add(1);
        while let Some((kind, ..)) = lines.get(scan) {
            if *kind == DiffLine::Kept && scan.saturating_sub(last_change) > CONTEXT.saturating_mul(2) {
                break;
            }
            if *kind != DiffLine::Kept {
                last_change = scan;
            }
            scan = scan.saturating_add(1);
        }
        let stop = last_change.saturating_add(CONTEXT).saturating_add(1).min(lines.len());
        let Some(hunk) = lines.get(start..stop) else { break };
        let old_count = hunk.iter().filter(|(kind, ..)| *kind != DiffLine::Added).count();
        let new_count = hunk.iter().filter(|(kind, ..)| *kind != DiffLine::Removed).count();
        let old_start = hunk
            .iter()
            .find_map(|(kind, _, old_no, _)| (*kind != DiffLine::Added).then_some(*old_no))
            .unwrap_or(0);
        let new_start = hunk
            .iter()
            .find_map(|(kind, _, _, new_no)| (*kind != DiffLine::Removed).then_some(*new_no))
            .unwrap_or(0);
        let _header = std::fmt::Write::write_fmt(
            &mut hunks,
            format_args!("@@ -{old_start},{old_count} +{new_start},{new_count} @@\n"),
        );
        for (kind, text, ..) in hunk {
            let marker = match kind {
                DiffLine::Kept => ' ',
                DiffLine::Removed => '-',
                DiffLine::Added => '+',
            };
            let _line = std::fmt::Write::write_fmt(&mut hunks, format_args!("{marker}{text}\n"));
        }
        cursor = stop;
    }
    // No hunks, no headers: an empty answer is how the caller tells "the
    // lines agree" (the final newline may still differ, and says so itself).
    if hunks.is_empty() {
        return hunks;
    }
    format!("--- {path}\n+++ {path}\n{hunks}")
}

/// One cell of the alignment table, 0 at the borders and beyond.
fn cell(lengths: &[u32], width: usize, i: usize, j: usize) -> u32 {
    lengths
        .get(i.saturating_mul(width).saturating_add(j))
        .copied()
        .unwrap_or(0)
}

/// Write one cell of the alignment table.
fn set_cell(lengths: &mut [u32], width: usize, i: usize, j: usize, value: u32) {
    if let Some(cell) = lengths.get_mut(i.saturating_mul(width).saturating_add(j)) {
        *cell = value;
    }
}
