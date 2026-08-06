//! Thin command-line interface over `musa-project`.
//!
//! The CLI must call public facades only; it never recreates compiler
//! orchestration (design roadmap §15.8).

/// Entry point. Until prompts 04–13 wire the real subcommands, this prints
/// the planned command surface and exits successfully.
fn main() {
    println!("musa — notation-first music language and workbench");
    println!();
    println!("Commands (planned):");
    println!("  musa check <file.musa>                 parse + compile diagnostics");
    println!("  musa format <file.musa>                format in place (--check to diff)");
    println!("  musa render <file.musa> --to <target>  mei | lilypond | musicxml | midi | wav");
    println!("  musa play <file.musa>                  live playback");
}
