//! What musa says about itself, checked by listening to it.
//!
//! The filter *policy* is a pure function and is tested where it lives
//! (`src/logging.rs`). These are the laws that need a real subscriber: that
//! the pipeline actually opens a span per compilation and names the document
//! in it, and that installing a subscriber twice is safe.

// A law that trips is a bug in the code it protects; panicking is the report.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use std::cell::RefCell;
use std::sync::Once;

use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, SubscriberExt as _};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt as _;

/// One span the pipeline opened: its name, and its fields as written down.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Opened {
    name: &'static str,
    fields: Vec<(String, String)>,
}

thread_local! {
    /// The spans opened on this thread, while this thread is listening.
    ///
    /// A span is built by the thread that opens it, so the thread doing the
    /// work is the thread [`Listener`] hears from, and a thread-local is both
    /// the exact scope a law wants and free of any lock the rest of the binary
    /// would queue behind.
    static OPENED_HERE: RefCell<Option<Vec<Opened>>> = const { RefCell::new(None) };
}

/// A subscriber layer that keeps every span opened by a listening thread.
///
/// No filter of its own: these laws ask what the *pipeline* emits, and a
/// filter between the two would make a silent pipeline and a strict filter
/// indistinguishable.
struct Listener;

impl<S: tracing::Subscriber + for<'a> LookupSpan<'a>> tracing_subscriber::Layer<S> for Listener {
    fn on_new_span(&self, attributes: &tracing::span::Attributes<'_>, _id: &tracing::Id, _context: Context<'_, S>) {
        OPENED_HERE.with_borrow_mut(|opened| {
            let Some(opened) = opened else { return };
            let mut fields = Fields(Vec::new());
            attributes.record(&mut fields);
            opened.push(Opened {
                name: attributes.metadata().name(),
                fields: fields.0,
            });
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

/// Make [`Listener`] this process's subscriber, once.
///
/// The obvious way to write these laws is
/// [`with_default`](tracing::subscriber::with_default), which installs a
/// subscriber for one thread and one scope. It does not work here, and the
/// reason is worth writing down, because a threaded test runner turns it into
/// an intermittent failure that looks like a bug in the pipeline.
///
/// Whether a span is built at all is decided by a *process-wide* cache, one
/// `Interest` per `info_span!` site in the source. The first thread anywhere in
/// the binary to reach a site fills that entry in, and — while only one
/// subscriber is registered, `tracing_core::callsite`'s `Rebuilder::JustOne`
/// path — it fills it in by asking *its own* current subscriber. A sibling test
/// reaching `compile` first, on a thread that has no subscriber, therefore
/// records `Interest::never` for `compile` on behalf of the whole process, and
/// a listener on another thread then watches a compilation open no span at all.
/// Nothing local can repair that: the poisoning thread is any of the other
/// hundred-odd tests in this binary, at any instant, and it wins whenever it
/// gets there first. Serialising these laws against each other does not help,
/// and neither does forcing a rebuild — both were measured.
///
/// So the subscriber is global and permanent instead. One registered subscriber
/// that answers "yes, every site" means every thread that fills the cache in
/// fills it in the same way, whichever thread that turns out to be, and the
/// question of who got there first stops having an answer that matters. What a
/// law wants to know — which spans *this* thread opened — is then a thread-local
/// (`OPENED_HERE`) rather than a subscriber, which is what it always was.
fn listen() {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        tracing_subscriber::registry()
            .with(Listener)
            .try_init()
            .expect("this file installs the only subscriber in this binary");
    });
}

/// Run `work` with this thread listening, and report the spans it opened.
fn listening_to<T>(work: impl FnOnce() -> T) -> Vec<Opened> {
    listen();
    OPENED_HERE.set(Some(Vec::new()));
    let _value = work();
    OPENED_HERE.take().expect("just set it")
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
    let compile = compiles.first().expect("just counted one");
    assert_eq!(field(compile, "document"), Some("listening.musa"));
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
    let apply = applies.first().expect("just counted one");
    assert_eq!(field(apply, "command"), Some("set-source"));
    for (_, value) in &apply.fields {
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
    // another is a no-op rather than a panic. Here the answer happens to be
    // known — `listen` has claimed the global on this binary's behalf, and this
    // law is the case of an embedding application that got there first.
    listen();
    let _first = musa_project::Logging::new().install();
    assert!(
        !musa_project::Logging::new().install(),
        "a subscriber is installed by now, so this one cannot be"
    );
}
