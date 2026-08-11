//! What musa says about itself, and who decides (roadmap §15.7, §15.8).
//!
//! Musa's crates emit `tracing` spans and events. Nothing is printed until a
//! *subscriber* is installed, and installing one is a process-wide decision:
//! there is exactly one, it is global, and the first installation wins. That
//! is a decision for a program, not for a library — an application that
//! embeds `musa-project` may already have its own, and musa must not take it
//! away.
//!
//! So the policy lives here and the installation lives in each `main`.
//! [`Logging`] knows what musa should say; `musa`, `musa-lsp`, and
//! `musa-desktop` each call [`Logging::install`] once, at start-up, and each
//! passes whatever its own command line said.
//!
//! # Where it goes
//!
//! Standard error, always, with no way to ask for anything else. `musa render
//! --to lilypond -o -` writes a score to stdout and `musa-lsp` writes JSON-RPC
//! there; a log line on that stream is a corrupted file or a broken protocol.
//! A destination that cannot be set cannot be set wrongly.
//!
//! # How much
//!
//! `MUSA_LOG` holds a filter in the syntax `RUST_LOG` uses —
//! `musa_compiler=debug`, `warn,musa_project::session=trace` — and names only
//! this program, so turning musa's logs on does not turn on every dependency
//! that happens to use `tracing`, and turning some other tool's logs on does
//! not flood this one.
//!
//! When `MUSA_LOG` is unset, the verbosity dial chooses the filter instead: no
//! flag means warnings from musa's own crates and silence elsewhere, `-v`
//! means info, `-vv` debug, `-vvv` trace. `MUSA_LOG` replaces the dial rather
//! than combining with it, because a filter and a level are different
//! instruments and the filter is the specific one.

use std::io::IsTerminal as _;

use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

/// The environment variable that holds musa's log filter.
///
/// Not `RUST_LOG`: that name is a shared namespace, and a person debugging one
/// Rust program should not have to read another's trace to do it.
pub const FILTER_VARIABLE: &str = "MUSA_LOG";

/// The targets musa's own crates log under.
///
/// A crate name with `-` becomes a target with `_`, and every one of them
/// begins with `musa`, so one prefix is the whole workspace. Filtering on it —
/// rather than on the bare level — is what keeps a dependency's `info` out of
/// a musician's terminal when they ask musa to be talkative.
const OUR_TARGETS: &str = "musa";

/// How much musa should say about what it is doing.
///
/// A policy, not an installation: constructing one prints nothing. Pass it to
/// [`install`](Self::install) from a `main` to make it the process's.
///
/// ```
/// use musa_project::Logging;
///
/// // A CLI that counted two `-v` flags and no `--quiet`.
/// let _installed = Logging::new().verbosity(2).install();
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Logging {
    verbosity: u8,
    quiet: bool,
}

impl Logging {
    /// The default policy: warnings from musa's crates, nothing else.
    ///
    /// This is deliberately not silence. Musa swallows a small number of
    /// recoverable failures — a MIDI port that would not open, a recovery copy
    /// that could not be written, an unreadable value in `musa.toml` — and
    /// each one warns. A default that hid them would make the warning a lie.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Raise the level: 1 is info, 2 is debug, 3 or more is trace.
    ///
    /// Takes the flag *count*, so a shell can pass `-vvv` through as `3`
    /// without deciding what it means.
    #[must_use]
    pub fn verbosity(mut self, count: u8) -> Self {
        self.verbosity = count;
        self
    }

    /// Say only what has actually failed.
    ///
    /// Wins over [`verbosity`](Self::verbosity): asking for quiet and for
    /// detail in the same breath is a mistake, and the safe reading of a
    /// mistake is the quieter one.
    #[must_use]
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// The filter this policy asks for, before the environment is consulted.
    ///
    /// Separate from [`install`](Self::install) so the dial can be tested
    /// without a global: installation happens once per process and cannot be
    /// undone, so a law about *what* is filtered must be able to ask without
    /// installing anything.
    pub(crate) fn dial(self) -> String {
        let level = if self.quiet {
            "error"
        } else {
            match self.verbosity {
                0 => "warn",
                1 => "info",
                2 => "debug",
                _ => "trace",
            }
        };
        format!("{OUR_TARGETS}={level}")
    }

    /// The filter this policy asks for given `configured`, the value of
    /// `MUSA_LOG`.
    ///
    /// `configured` is a parameter rather than a read, so the whole policy is
    /// a pure function of two arguments and can be stated as laws. Setting an
    /// environment variable to test one would mutate process-wide state that
    /// every other test in the binary shares.
    ///
    /// An empty variable reads as "not set". A shell that exports
    /// `MUSA_LOG=""` has said nothing, and the alternative — an empty filter,
    /// which admits nothing at all — would be a silent way to lose every
    /// warning.
    pub(crate) fn filter_from(self, configured: Option<&str>) -> String {
        match configured {
            Some(value) if !value.trim().is_empty() => value.to_owned(),
            Some(_) | None => self.dial(),
        }
    }

    /// The filter actually used: `MUSA_LOG` if it is set and non-empty,
    /// otherwise the dial.
    fn filter(self) -> String {
        self.filter_from(std::env::var(FILTER_VARIABLE).ok().as_deref())
    }

    /// Install this policy as the process-wide subscriber, on standard error.
    ///
    /// Returns `false` when a subscriber was already installed. That is not an
    /// error: a host application may have installed its own, a second shell
    /// may be running in-process, or a test may have called this before. Musa
    /// does not fight for the global, and calling this repeatedly is safe.
    ///
    /// An unparseable `MUSA_LOG` also returns `false`, after one line on
    /// stderr saying so. A malformed filter is a typo in a debugging session,
    /// not a reason to refuse to run the program.
    pub fn install(self) -> bool {
        let filter = self.filter();
        let Ok(env_filter) = tracing_subscriber::EnvFilter::try_new(&filter) else {
            eprintln!("warning: {FILTER_VARIABLE} is not a valid log filter: `{filter}`; logging is off");
            return false;
        };
        let layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(std::io::stderr().is_terminal())
            .with_target(true);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(layer)
            .try_init()
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::Logging;

    // These are string laws on purpose. What `musa=warn` *means* is
    // `EnvFilter`'s documented contract and tracing's to keep; what musa
    // decides is which filter it asks for, and that is exactly this string.
    // A test that installed a subscriber to watch an event arrive would be
    // testing the library through a global that every other test shares.

    #[test]
    fn the_dial_climbs_and_stops_at_trace() {
        assert_eq!(Logging::new().dial(), "musa=warn");
        assert_eq!(Logging::new().verbosity(1).dial(), "musa=info");
        assert_eq!(Logging::new().verbosity(2).dial(), "musa=debug");
        assert_eq!(Logging::new().verbosity(3).dial(), "musa=trace");
        assert_eq!(Logging::new().verbosity(9).dial(), "musa=trace");
    }

    #[test]
    fn quiet_wins_over_every_amount_of_detail() {
        assert_eq!(Logging::new().quiet(true).verbosity(3).dial(), "musa=error");
    }

    #[test]
    fn a_set_variable_replaces_the_dial_and_an_empty_one_does_not() {
        let noisy = Logging::new().verbosity(3);
        assert_eq!(noisy.filter_from(Some("musa_compiler=trace")), "musa_compiler=trace");
        assert_eq!(noisy.filter_from(Some("")), "musa=trace");
        assert_eq!(noisy.filter_from(Some("  ")), "musa=trace");
        assert_eq!(noisy.filter_from(None), "musa=trace");
    }

    #[test]
    fn every_filter_musa_writes_is_one_env_filter_accepts() {
        for logging in [
            Logging::new(),
            Logging::new().verbosity(1),
            Logging::new().verbosity(2),
            Logging::new().verbosity(3),
            Logging::new().quiet(true),
        ] {
            let filter = logging.filter_from(None);
            assert!(
                tracing_subscriber::EnvFilter::try_new(&filter).is_ok(),
                "musa wrote `{filter}`, which it would then have to refuse"
            );
        }
    }
}
