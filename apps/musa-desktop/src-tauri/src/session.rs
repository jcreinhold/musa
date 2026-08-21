//! The session, on its own thread, and the handle the commands talk to.
//!
//! `ProjectSession` owns an audio engine, and an audio engine owns a CPAL
//! stream, which is not `Send` on any platform we target. Tauri's managed
//! state must be `Send + Sync`, so the session cannot be managed state. It
//! lives on one thread it never leaves, and the shell talks to it over a
//! channel — which is also the natural place to run the position clock,
//! because that thread already knows whether the transport is playing.
//!
//! Replies cross the channel as JSON rather than as a `ProjectSnapshot`,
//! because a snapshot borrows the session. That is not a workaround: the
//! shell's job is command adaptation (roadmap §15.9), and JSON is the shape
//! the webview was always going to get. `musa-project` owns that shape —
//! mirroring `ScoreFacts` into a DTO here would create a second wire format
//! that can silently disagree with the one the UI fixtures are generated
//! from, which is the drift the DTO rule exists to prevent.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::Duration;

use musa_project::{
    EditCommand, ExportRequest, PlaybackState, Project, ProjectCommand, Template, TransportRequest, Utf16Offsets,
};
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::dto::{ErrorDto, ErrorKindDto, ExportedDto};

/// How often the position is reported while playing (`06-frame-budgets.md` §3).
/// At rest the thread blocks: there is no timer anywhere in the application.
const POSITION_INTERVAL: Duration = Duration::from_millis(100);

/// How often the MIDI queue is drained while note entry is on.
///
/// Only while entry is on: a keyboard nobody is entering with costs nothing,
/// and the thread goes back to blocking the moment entry is switched off.
/// Fifteen milliseconds is below the chord window the session groups with, so
/// grouping — not polling — is what decides when a chord is ready.
const MIDI_INTERVAL: Duration = Duration::from_millis(15);

/// The event names the webview subscribes to.
pub const SNAPSHOT_EVENT: &str = "musa://snapshot";
pub const POSITION_EVENT: &str = "musa://position";
pub const TRANSPORT_EVENT: &str = "musa://transport";
pub const MIDI_EVENT: &str = "musa://midi";

/// One unit of work for the session thread.
enum Job {
    /// Open a piece, or a folder of them.
    Open(PathBuf),
    New(Template, Option<PathBuf>),
    /// Turn to another file of the project already open.
    Show(String),
    /// Write every piece of it that has unsaved edits.
    SaveAll,
    Apply(Request),
    Transport(TransportRequest),
    Export(ExportRequest, PathBuf),
    /// Ask what an edit would change, without making it.
    Impact(EditCommand),
    /// Re-read the current snapshot without changing anything.
    Snapshot,
    /// Start or stop reading the MIDI keyboard, and say where the caret is.
    ///
    /// The caret rides along because what a played note is *spelled* as
    /// depends on the key in force where it lands, and a piece modulates. It
    /// is an engraved event id, the identity the page already selects by.
    Midi(bool, Option<String>),
    /// Read the last valid score, and report what one analysis saw.
    ///
    /// A job like the others because it reads the session's score, and
    /// deliberately not a mutating one: an analysis mints no revision and
    /// raises no diagnostic (`docs/rules/desktop/08-elaboration.md` §5).
    Analyze(String),
}

/// An `apply` that is either a document command or history navigation.
pub(crate) enum Request {
    Command(ProjectCommand),
    Undo,
    Redo,
}

type Reply = Result<Value, ErrorDto>;

/// The handle the Tauri commands hold. Cheap to clone, `Send + Sync`.
pub struct SessionHandle {
    jobs: Sender<(Job, Sender<Reply>)>,
}

impl SessionHandle {
    /// Start the session thread and return its handle.
    ///
    /// The thread outlives every command; it ends when the handle is dropped,
    /// which happens when the application does.
    pub fn spawn(app: AppHandle) -> Self {
        let (jobs, inbox) = channel();
        thread::Builder::new()
            .name("musa-session".to_owned())
            .spawn(move || run(&app, &inbox))
            .ok();
        Self { jobs }
    }

    fn ask(&self, job: Job) -> Reply {
        let (reply, answer) = channel();
        self.jobs
            .send((job, reply))
            .map_err(|_| ErrorDto::shell(ErrorKindDto::Backend, "The session is not running"))?;
        answer
            .recv()
            .map_err(|_| ErrorDto::shell(ErrorKindDto::Backend, "The session stopped responding"))?
    }

    pub(crate) fn open(&self, path: PathBuf) -> Reply {
        self.ask(Job::Open(path))
    }

    pub(crate) fn new_project(&self, template: Template, path: Option<PathBuf>) -> Reply {
        self.ask(Job::New(template, path))
    }

    pub(crate) fn show(&self, file: String) -> Reply {
        self.ask(Job::Show(file))
    }

    pub(crate) fn save_all(&self) -> Reply {
        self.ask(Job::SaveAll)
    }

    pub(crate) fn apply(&self, request: Request) -> Reply {
        self.ask(Job::Apply(request))
    }

    pub(crate) fn transport(&self, request: TransportRequest) -> Reply {
        self.ask(Job::Transport(request))
    }

    pub(crate) fn export(&self, request: ExportRequest, path: PathBuf) -> Reply {
        self.ask(Job::Export(request, path))
    }

    pub(crate) fn snapshot(&self) -> Reply {
        self.ask(Job::Snapshot)
    }

    pub(crate) fn edit_impact(&self, command: EditCommand) -> Reply {
        self.ask(Job::Impact(command))
    }

    pub(crate) fn listen_to_midi(&self, listening: bool, caret: Option<String>) -> Reply {
        self.ask(Job::Midi(listening, caret))
    }

    pub(crate) fn analyze(&self, kind: String) -> Reply {
        self.ask(Job::Analyze(kind))
    }
}

/// The session thread.
fn run(app: &AppHandle, inbox: &Receiver<(Job, Sender<Reply>)>) {
    let mut session: Option<Project> = None;
    let mut playing = false;
    let mut listening = false;
    let mut caret: Option<String> = None;
    loop {
        // Only a playing transport and an open MIDI keyboard need a clock. At
        // rest this blocks, so the application has no timer running and no
        // wakeups to account for.
        let interval = match (playing, listening) {
            (true, true) => POSITION_INTERVAL.min(MIDI_INTERVAL),
            (true, false) => POSITION_INTERVAL,
            (false, true) => MIDI_INTERVAL,
            (false, false) => Duration::ZERO,
        };
        let received = if interval.is_zero() {
            match inbox.recv() {
                Ok(job) => Some(job),
                Err(_) => return,
            }
        } else {
            match inbox.recv_timeout(interval) {
                Ok(job) => Some(job),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        };

        if let Some((job, reply)) = received {
            if let Job::Midi(wanted, ref at) = job {
                listening = wanted;
                caret.clone_from(at);
            }
            let mutating = !matches!(job, Job::Snapshot | Job::Impact(_) | Job::Midi(..) | Job::Analyze(_));
            let answer = perform(&mut session, job);
            let changed = mutating && answer.is_ok();
            // A dropped receiver means the webview went away mid-command;
            // the work is already done and there is nobody to tell.
            reply.send(answer).ok();
            if changed && let Some(open) = session.as_mut() {
                emit(app, SNAPSHOT_EVENT, &snapshot_json(open));
            }
        }

        if listening && let Some(open) = session.as_mut() {
            for entry in open.current_mut().midi_entry(caret.as_deref()) {
                emit(app, MIDI_EVENT, &entry);
            }
        }

        if let Some(open) = session.as_mut() {
            let state = open.snapshot().playback();
            if state.playing {
                emit(app, POSITION_EVENT, &state);
            }
            if state.playing != playing {
                playing = state.playing;
                emit(app, TRANSPORT_EVENT, &state);
            }
        }
    }
}

fn emit<T: serde::Serialize>(app: &AppHandle, event: &str, payload: &T) {
    // A failed emit means the window is gone; the session keeps its state and
    // the next window gets a fresh snapshot.
    app.emit(event, payload).ok();
}

/// The snapshot as the webview receives it.
///
/// `musa-project` owns the shape *and* the unit: `to_wire` states every span
/// in UTF-16 code units, which is what `CodeMirror` and every JavaScript string
/// index count in (`musa_project::Utf16Offsets`).
fn snapshot_json(session: &mut Project) -> Value {
    session.snapshot().to_wire()
}

/// Run one job against the session.
fn perform(session: &mut Option<Project>, job: Job) -> Reply {
    match job {
        Job::Open(path) => {
            let opened = Project::open(&path).map_err(|error| ErrorDto::from(&error))?;
            Ok(replace(session, opened))
        }
        Job::New(template, path) => {
            let created = match path {
                Some(path) => Project::create(&path, template).map_err(|e| ErrorDto::from(&e))?,
                None => Project::new_piece(template, "Untitled"),
            };
            Ok(replace(session, created))
        }
        Job::Show(file) => {
            let open = session.as_mut().ok_or_else(no_project)?;
            open.show(&file).map_err(|error| ErrorDto::from(&error))?;
            Ok(snapshot_json(open))
        }
        Job::SaveAll => {
            let open = session.as_mut().ok_or_else(no_project)?;
            open.save_all().map_err(|error| ErrorDto::from(&error))?;
            Ok(snapshot_json(open))
        }
        Job::Apply(request) => {
            let open = session.as_mut().ok_or_else(no_project)?;
            // Saving is the one document command that can change what the
            // contents page says about the *file*: a piece written for the
            // first time is a file the directory did not have.
            let wrote = matches!(request, Request::Command(ProjectCommand::Save));
            let piece = open.current_mut();
            let outcome = match request {
                // The webview states an edit's range in its own measure; the
                // session applies it to a Rust string, which is measured in
                // bytes. This is the inbound half of the same contract
                // `snapshot_json` keeps on the way out.
                Request::Command(ProjectCommand::ApplyEdits(edits)) => {
                    let offsets = Utf16Offsets::new(piece.snapshot().source());
                    piece.apply(ProjectCommand::ApplyEdits(
                        edits.into_iter().map(|edit| to_bytes(edit, &offsets)).collect(),
                    ))
                }
                Request::Command(command) => piece.apply(command),
                Request::Undo => piece.undo(),
                Request::Redo => piece.redo(),
            };
            outcome.map_err(|error| ErrorDto::from(&error))?;
            if wrote {
                open.rescan();
            }
            Ok(snapshot_json(open))
        }
        Job::Transport(request) => {
            let open = session.as_mut().ok_or_else(no_project)?;
            open.current_mut()
                .apply(ProjectCommand::Transport(request))
                .map_err(|e| ErrorDto::from(&e))?;
            Ok(snapshot_json(open))
        }
        Job::Export(request, path) => {
            let open = session.as_ref().ok_or_else(no_project)?;
            let artifact = open.current().export(request).map_err(|error| ErrorDto::from(&error))?;
            std::fs::write(&path, artifact.as_bytes())
                .map_err(|error| ErrorDto::shell(ErrorKindDto::File, format!("{}: {error}", path.display())))?;
            serde_json::to_value(ExportedDto::from(path))
                .map_err(|error| ErrorDto::shell(ErrorKindDto::Backend, error.to_string()))
        }
        Job::Impact(command) => {
            let open = session.as_ref().ok_or_else(no_project)?;
            let impact = open
                .current()
                .edit_impact(&command)
                .map_err(|error| ErrorDto::from(&error))?;
            serde_json::to_value(impact).map_err(|error| ErrorDto::shell(ErrorKindDto::Backend, error.to_string()))
        }
        Job::Snapshot => session.as_mut().map(snapshot_json).ok_or_else(no_project),
        Job::Analyze(kind) => {
            let open = session.as_ref().ok_or_else(no_project)?;
            let kind = musa_project::AnalysisKind::named(&kind)
                .ok_or_else(|| ErrorDto::shell(ErrorKindDto::Backend, format!("`{kind}` is not an analysis")))?;
            open.current()
                .analyze_wire(&musa_project::AnalysisRequest::new(kind))
                .map_err(|error| ErrorDto::from(&error))
        }
        Job::Midi(listening, _) => {
            let open = session.as_mut().ok_or_else(no_project)?;
            if listening {
                open.current_mut().listen_to_midi();
            }
            Ok(snapshot_json(open))
        }
    }
}

/// One text edit, restated in the measure the session applies it in.
fn to_bytes(edit: musa_project::TextEdit, offsets: &Utf16Offsets) -> musa_project::TextEdit {
    musa_project::TextEdit::new(
        musa_project::Span {
            start: offsets.to_bytes(edit.span.start),
            end: offsets.to_bytes(edit.span.end),
        },
        edit.replacement,
    )
}

/// Install a newly opened project and answer with its snapshot.
fn replace(session: &mut Option<Project>, opened: Project) -> Value {
    let installed = session.insert(opened);
    snapshot_json(installed)
}

fn no_project() -> ErrorDto {
    ErrorDto::shell(ErrorKindDto::Nothing, "No piece is open")
}

/// Re-exported for the position event's payload type.
pub type Position = PlaybackState;

#[cfg(test)]
mod session_laws {
    use super::{Job, Request, perform};
    use crate::dto::{CommandDto, ErrorKindDto};
    use musa_project::Project;
    use serde_json::Value;

    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    const PIECE: &str = concat!(
        "piece \"Test\" {\n",
        "    tempo quarter = 120;\n",
        "    meter 4/4;\n",
        "    key c major;\n\n",
        "    score {\n",
        "        part piano {\n",
        "            voice upper {\n",
        "                c4/4\n",
        "                e4/4\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}\n",
    );

    fn opened() -> Option<Project> {
        Some(Project::from_text(PIECE, "test.musa"))
    }

    /// A field of a snapshot, or `Null` when the wire format has lost it —
    /// which every assertion below then reports as the mismatch it is.
    fn field<'a>(snapshot: &'a Value, name: &str) -> &'a Value {
        static MISSING: Value = Value::Null;
        snapshot.get(name).unwrap_or(&MISSING)
    }

    fn apply(session: &mut Option<Project>, source: &str) -> super::Reply {
        let command = CommandDto::SetSource {
            source: source.to_owned(),
        };
        perform(session, Job::Apply(command.into_request()))
    }

    /// Every place in the wire format where a `{start, end}` object appears,
    /// paired with the field it appeared under.
    fn start_end_objects(value: &Value, under: &str, found: &mut Vec<(String, u32)>) {
        match *value {
            Value::Object(ref fields) => {
                let pair = fields.len() == 2 && fields.contains_key("start") && fields.contains_key("end");
                if pair {
                    let start = fields.get("start").and_then(Value::as_u64).unwrap_or(0);
                    found.push((under.to_owned(), u32::try_from(start).unwrap_or(0)));
                    return;
                }
                for (name, field) in fields {
                    start_end_objects(field, name, found);
                }
            }
            Value::Array(ref items) => {
                for item in items {
                    start_end_objects(item, under, found);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }

    /// The claim [`crate::offsets::translate_spans`] rests on: in the wire
    /// format a `{start, end}` object is *always* a source span, so
    /// translating them all translates exactly the right set. If a future
    /// field of that shape means something else — a frame range, a bar range
    /// — this fails, and the walk has to stop being structural.
    #[test]
    fn every_start_end_object_in_the_wire_format_is_a_source_span() -> Result {
        let mut session = opened();
        let snapshot: Value = apply(&mut session, PIECE)?;
        let mut found = Vec::new();
        start_end_objects(&snapshot, "", &mut found);
        let spans = ["declaration", "definitionSpan", "span", "supportedBy", "useSite"];
        assert!(!found.is_empty(), "the piece serialized to no spans at all");
        for (name, _) in found {
            assert!(
                spans.contains(&name.as_str()),
                "`{name}` is a `{{start, end}}` object that is not a source span; the walk can no longer be structural"
            );
        }
        Ok(())
    }

    /// The contract `docs/rules/desktop/03-interaction.md` §7 fixes: the webview
    /// is handed UTF-16 code units, never bytes.
    ///
    /// Stated without arithmetic: an em dash and a hyphen are both one code
    /// unit, and differ only in how many bytes they take (three against one).
    /// So in the measure the webview reads, the two comments are the same
    /// length and every span in the piece below them must land in exactly the
    /// same place. Untranslated, each em dash pushes them two further along.
    #[test]
    fn a_wider_character_does_not_move_the_spans_below_it() -> Result {
        let heading = |dash: &str| PIECE.replace("piece", &format!("// a heading {dash} and its piece\npiece"));
        let mut session = opened();
        let wide: Value = apply(&mut session, &heading("—"))?;
        let narrow: Value = apply(&mut session, &heading("-"))?;

        let (mut from_wide, mut from_narrow) = (Vec::new(), Vec::new());
        start_end_objects(&wide, "", &mut from_wide);
        start_end_objects(&narrow, "", &mut from_narrow);
        assert!(!from_wide.is_empty(), "the piece serialized to no spans at all");
        assert_eq!(from_wide, from_narrow);
        Ok(())
    }

    /// Roadmap §14.7, and `05-states.md` §4: a source that does not compile is
    /// not a failed command. The snapshot that crosses the bridge still
    /// carries the last score that did compile, so the engraving the user is
    /// looking at does not blink out while they are mid-word.
    #[test]
    fn invalid_source_keeps_the_last_valid_score_on_the_wire() -> Result {
        let mut session = opened();
        let valid: Value = apply(&mut session, PIECE)?;
        let revision = field(&valid, "revision").clone();

        let broken: Value = apply(&mut session, "piece \"Test\" { score {")?;

        assert_eq!(
            field(&broken, "source"),
            "piece \"Test\" { score {",
            "the text is the user's"
        );
        assert_ne!(*field(&broken, "revision"), revision, "an edit is still a new revision");
        assert_eq!(
            field(&broken, "compiles"),
            &Value::Bool(false),
            "and it is flagged as stale"
        );
        assert_eq!(
            field(&broken, "score"),
            field(&valid, "score"),
            "but the score the interface renders does not change"
        );
        assert!(
            field(&broken, "diagnostics")
                .as_array()
                .is_some_and(|list| !list.is_empty()),
            "and the problem is named"
        );
        Ok(())
    }

    /// `snapshot` is a read: it must not be mistaken for an edit, or the
    /// window opening would spend a revision.
    #[test]
    fn reading_the_snapshot_changes_nothing() -> Result {
        let mut session = opened();
        let first = perform(&mut session, Job::Snapshot)?;
        let again = perform(&mut session, Job::Snapshot)?;
        assert_eq!(first, again);
        Ok(())
    }

    /// Before a piece is open, every command that needs one says so in the
    /// same voice, so the launch state has one message to render.
    #[test]
    fn commands_without_a_piece_report_nothing_open() {
        let mut nothing: Option<Project> = None;
        let jobs = [Job::Snapshot, Job::Apply(Request::Undo), Job::Apply(Request::Redo)];
        for job in jobs {
            let kind = perform(&mut nothing, job).err().map(|error| error.kind);
            assert!(matches!(kind, Some(ErrorKindDto::Nothing)), "a command needs a piece");
        }
    }

    /// An export lands where it was asked to, and reports that path back so
    /// the interface can name the file it just wrote.
    #[test]
    fn export_writes_the_file_it_names() -> Result {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("test.mei");
        let mut session = opened();
        let reported = perform(
            &mut session,
            Job::Export(musa_project::ExportRequest::Mei, path.clone()),
        )?;
        assert_eq!(field(&reported, "path"), &Value::from(path.display().to_string()));
        assert!(std::fs::read_to_string(&path)?.contains("<mei"));
        Ok(())
    }
}
