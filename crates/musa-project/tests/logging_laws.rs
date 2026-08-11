//! What musa says about itself, checked by listening to it.
//!
//! The filter *policy* is a pure function and is tested where it lives
//! (`src/logging.rs`). These are the laws that need a real subscriber: that
//! the pipeline actually opens a span per compilation and names the document
//! in it, and that installing a subscriber twice is safe.

// A law that trips is a bug in the code it protects; panicking is the report.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, SubscriberExt as _};
use tracing_subscriber::registry::LookupSpan;

/// One span the pipeline opened: its name, and its fields as written down.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Opened {
    name: &'static str,
    fields: Vec<(String, String)>,
}

/// A subscriber layer that keeps every span it is told about.
///
/// No filter of its own: these laws ask what the *pipeline* emits, and a
/// filter between the two would make a silent pipeline and a strict filter
/// indistinguishable.
#[derive(Clone, Default)]
struct Listener {
    opened: Arc<Mutex<Vec<Opened>>>,
}

impl<S: tracing::Subscriber + for<'a> LookupSpan<'a>> tracing_subscriber::Layer<S> for Listener {
    fn on_new_span(&self, attributes: &tracing::span::Attributes<'_>, _id: &tracing::Id, _context: Context<'_, S>) {
        let mut fields = Fields(Vec::new());
        attributes.record(&mut fields);
        self.opened.lock().unwrap().push(Opened {
            name: attributes.metadata().name(),
            fields: fields.0,
        });
    }
}

/// Every field of a span, as `(name, value)` text.
struct Fields(Vec<(String, String)>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.push((field.name().to_owned(), format!("{value:?}")));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name().to_owned(), value.to_owned()));
    }
}

/// Run `work` with a listening subscriber, and report the spans it opened.
fn listening_to<T>(work: impl FnOnce() -> T) -> Vec<Opened> {
    let listener = Listener::default();
    let subscriber = tracing_subscriber::registry().with(listener.clone());
    tracing::subscriber::with_default(subscriber, work);
    let opened = listener.opened.lock().unwrap().clone();
    opened
}

/// What a span recorded for `field`, if it recorded one.
fn field<'a>(span: &'a Opened, name: &str) -> Option<&'a str> {
    span.fields
        .iter()
        .find(|(written, _)| written == name)
        .map(|(_, value)| value.as_str())
}

const PIECE: &str = r#"piece "Listening" {
    meter 4/4;
    key c major;
    score { part piano { voice right { c4/4 e4/4 g4/4 e4/4 } } }
}
"#;

/// Compiling a piece opens exactly one `compile` span, and that span names the
/// document.
///
/// The whole point of the span is that a session compiling several documents
/// produces several readable accounts rather than one interleaved one. A
/// second `compile` span per compilation would break that as surely as none.
#[test]
fn a_compilation_opens_one_compile_span_naming_its_document() {
    let opened = listening_to(|| {
        let _session = musa_project::ProjectSession::from_text(PIECE, "listening.musa");
    });
    let compiles: Vec<&Opened> = opened.iter().filter(|span| span.name == "compile").collect();
    assert_eq!(compiles.len(), 1, "one compilation, one span: {opened:#?}");
    assert_eq!(field(compiles[0], "document"), Some("listening.musa"));
}

/// Every session command runs inside an `apply` span that names the command.
///
/// Named by the command's own word rather than by its `Debug`: `SetSource`
/// carries a whole document, and a log that printed one would be unreadable
/// exactly when a person needs to read it.
#[test]
fn a_session_command_opens_an_apply_span_naming_the_command_and_not_the_document() {
    let opened = listening_to(|| {
        let mut session = musa_project::ProjectSession::from_text(PIECE, "listening.musa");
        let _update = session.apply(musa_project::ProjectCommand::SetSource(PIECE.to_owned()));
    });
    let applies: Vec<&Opened> = opened.iter().filter(|span| span.name == "apply").collect();
    assert_eq!(applies.len(), 1, "one command, one span: {opened:#?}");
    assert_eq!(field(applies[0], "command"), Some("set-source"));
    for (_, value) in &applies[0].fields {
        assert!(
            !value.contains("piece"),
            "the source itself must not be in the span: {value}"
        );
    }
}

/// Installing a subscriber twice is safe, and the second call says it did
/// nothing.
///
/// Three shells install one; a test binary may install one; an embedding
/// application may already have installed its own. Every one of those has to
/// be survivable, because the global can only be claimed once and musa is not
/// always the first to reach it.
#[test]
fn installing_a_second_time_reports_that_it_did_not() {
    // Whether the *first* call wins depends on what else this process already
    // did, which is why the law is about the second one: after any install,
    // another is a no-op rather than a panic.
    let _first = musa_project::Logging::new().install();
    assert!(
        !musa_project::Logging::new().install(),
        "a subscriber is installed by now, so this one cannot be"
    );
}
