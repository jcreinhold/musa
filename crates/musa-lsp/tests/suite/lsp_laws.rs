//! End-to-end laws of the server, driven through the protocol itself.
//!
//! There is no smaller honest way to test a protocol adapter: each test runs
//! the whole server on one end of a `Connection::memory()` pair and plays
//! the client on the other, against the committed `examples/` fixtures — the
//! same executable specifications the rest of the suite compiles.

// Test helpers use expect()/panic! on statically-valid inputs: a failure is a
// bug in the test itself, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]
#![allow(clippy::missing_panics_doc)]
// The protocol's own WorkspaceEdit keys a HashMap by Uri; the tests only read
// it, and the type is the specification's, not ours to change.
#![allow(clippy::mutable_key_type)]

use std::path::PathBuf;
use std::str::FromStr as _;
use std::thread::JoinHandle;

use lsp_server::{Connection, Message, RequestId};
use lsp_types::notification::{DidChangeTextDocument, DidOpenTextDocument, Exit, Initialized, PublishDiagnostics};
use lsp_types::request::{
    CodeActionRequest, CodeLensRequest, Completion, DocumentSymbolRequest, ExecuteCommand, FoldingRangeRequest,
    Formatting, GotoDefinition, HoverRequest, Initialize, PrepareRenameRequest, References, Rename,
    SemanticTokensFullRequest, Shutdown, SignatureHelpRequest,
};
use lsp_types::{
    CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse, CodeLens, CodeLensParams,
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidOpenTextDocumentParams, DocumentSymbolParams,
    DocumentSymbolResponse, ExecuteCommandParams, FoldingRange, FoldingRangeKind, FoldingRangeParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, InitializedParams, Location, PartialResultParams,
    Position, PrepareRenameResponse, PublishDiagnosticsParams, ReferenceContext, ReferenceParams, RenameParams,
    SemanticTokensParams, SemanticTokensResult, SignatureHelp, SymbolKind, SymbolTag, TextDocumentContentChangeEvent,
    TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Uri, VersionedTextDocumentIdentifier,
    WorkDoneProgressParams,
};

const GLASS_MOUNTAIN: &str = include_str!("../../../../examples/glass-mountain.musa");
const ANNOTATED: &str = include_str!("../../../../examples/annotated.musa");
const MISSING_SEMICOLON: &str = include_str!("../../../../examples/broken/missing-semicolon.musa");
const STDLIB_PIECE: &str = "piece \"Standard library\" {
    import std::core;
    let answer: Nat = identity_nat(42);
    tempo 1/4 = 60;
    meter 4/4;
    key c major;
    score { part piano { voice melody { c4/1 } } }
}
";

/// The elaboration language, in the smallest piece that writes all of it: a
/// signature and a structure at the document root, a function with a default
/// parameter, a deprecated binding, a spelled pitch class, and a claim with a
/// realization policy. Every prompt-122 law about *declarations* reads this.
const TOOLING: &str = include_str!("tooling.musa");

/// An event track quote with holes in it — the one site whose completion is
/// narrower than the document's whole vocabulary.
const EVENTS_SPLICE: &str = include_str!("../../../../examples/events-splice.musa");

/// A small valid piece whose every position the tests can count by hand —
/// one full bar of 4/4.
const HOVER_PIECE: &str = "piece \"Hover\" {
    tempo 1/4 = 96;
    meter 4/4;
    key c major;

    score {
        part piano {
            voice melody {
                c4/4 e4/4 g4/4 e4/4
            }
        }
    }
}
";

/// Studio tooling is syntax-driven so it remains available while the call is
/// incomplete and the project has no current studio facts.
const STUDIO_TOOLING: &str = "piece \"Studio tooling\" {
    tempo 1/4 = 96;
    meter 4/4;
    score { part piano { voice melody { c4/1 } } }
    studio {
        patch soft {
            oscillator(sine) |> lowpass(cutoff: 1400 Hz, resonance: ) |> output;
        }
    }
}
";

/// The client end of the connection, with the responses' bookkeeping a test
/// should never have to write out.
struct Client {
    connection: Connection,
    next_id: i32,
    /// Notifications that arrived while a request was being answered.
    notifications: Vec<lsp_server::Notification>,
}

/// A running server: the client, the handshake's answer, and the thread.
struct Server {
    client: Client,
    capabilities: serde_json::Value,
    thread: Option<JoinHandle<Result<(), musa_lsp::ServerError>>>,
}

impl Server {
    /// Start the server and run the initialize handshake.
    fn start() -> Self {
        let (server_side, client_side) = Connection::memory();
        let thread = std::thread::spawn(move || musa_lsp::run(&server_side));
        let mut client = Client {
            connection: client_side,
            next_id: 0,
            notifications: Vec::new(),
        };
        let capabilities = client.request::<Initialize>(lsp_types::InitializeParams::default());
        client.notify::<Initialized>(InitializedParams {});
        Self {
            client,
            capabilities,
            thread: Some(thread),
        }
    }

    /// Shut down cleanly and assert the server ended without an error.
    fn stop(mut self) {
        self.client.request::<Shutdown>(());
        self.client.notify::<Exit>(());
        let thread = self.thread.take().expect("server thread");
        let outcome = thread.join().expect("server thread panicked");
        outcome.expect("server run failed");
    }

    /// Open `text` as `<name>.musa` and return the diagnostics the open
    /// published.
    fn open(&mut self, name: &str, text: &str) -> (Uri, PublishDiagnosticsParams) {
        let uri = uri(name);
        self.client.notify::<DidOpenTextDocument>(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "musa".to_owned(),
                version: 1,
                text: text.to_owned(),
            },
        });
        let published = self.client.notification::<PublishDiagnostics>();
        (uri, published)
    }

    /// Replace the document's whole text and return the diagnostics the
    /// change published.
    fn change(&mut self, uri: &Uri, text: String) -> PublishDiagnosticsParams {
        self.client
            .notify::<DidChangeTextDocument>(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text,
                }],
            });
        self.client.notification::<PublishDiagnostics>()
    }
}

impl Client {
    /// Send a request and read messages until its answer arrives, buffering
    /// the notifications that overtake it.
    fn request<R: lsp_types::request::Request>(&mut self, params: R::Params) -> serde_json::Value {
        let response = self.response::<R>(params);
        response
            .response_result
            .unwrap_or_else(|error| panic!("`{}` failed: {error:?}", R::METHOD))
    }

    /// Send a request and return the raw response — the tests that assert a
    /// *refusal* (rename's two refusals) need the error, not a panic.
    fn response<R: lsp_types::request::Request>(&mut self, params: R::Params) -> lsp_server::Response {
        let id = RequestId::from(self.next_id);
        self.next_id = self.next_id.checked_add(1).expect("request id");
        self.connection
            .sender
            .send(Message::Request(lsp_server::Request::new(
                id.clone(),
                R::METHOD.to_owned(),
                params,
            )))
            .expect("send request");
        loop {
            match self.connection.receiver.recv().expect("receive") {
                Message::Response(response) if response.id == id => return response,
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Response(_) | Message::Request(_) => {}
            }
        }
    }

    /// Send a notification.
    fn notify<N: lsp_types::notification::Notification>(&self, params: N::Params) {
        self.connection
            .sender
            .send(Message::Notification(lsp_server::Notification::new(
                N::METHOD.to_owned(),
                params,
            )))
            .expect("send notification");
    }

    /// Read until the next notification of kind `N`, buffering the rest.
    fn notification<N: lsp_types::notification::Notification>(&mut self) -> N::Params {
        loop {
            if let Some(at) = self
                .notifications
                .iter()
                .position(|notification| notification.method == N::METHOD)
            {
                return self
                    .notifications
                    .remove(at)
                    .extract::<N::Params>(N::METHOD)
                    .expect("parse notification");
            }
            match self.connection.receiver.recv().expect("receive") {
                Message::Notification(notification) if notification.method == N::METHOD => {
                    return notification
                        .extract::<N::Params>(N::METHOD)
                        .expect("parse notification");
                }
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Response(_) | Message::Request(_) => {}
            }
        }
    }
}

/// The URI a test document lives under. The server never reads the path; any
/// well-formed URI identifies a document.
fn uri(name: &str) -> Uri {
    Uri::from_str(&format!("file:///{name}.musa")).expect("uri")
}

/// The LSP position of `needle` in `source`, counted by hand. The fixtures
/// this is used on are ASCII on the lines in question, so bytes and UTF-16
/// code units agree.
fn at(source: &str, needle: &str) -> Position {
    let offset = source.find(needle).expect("needle in source");
    let before = source.get(..offset).expect("offset");
    let line = before.matches('\n').count();
    let column = before.rsplit('\n').next().expect("last line").len();
    Position::new(
        u32::try_from(line).expect("line"),
        u32::try_from(column).expect("column"),
    )
}

/// Position params boilerplate.
fn position_params(uri: &Uri, position: Position) -> TextDocumentPositionParams {
    TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position,
    }
}

/// The LSP position of the LAST occurrence of `needle` in `source`.
fn at_last(source: &str, needle: &str) -> Position {
    let offset = source.rfind(needle).expect("needle in source");
    let before = source.get(..offset).expect("offset");
    let line = before.matches('\n').count();
    let column = before.rsplit('\n').next().expect("last line").len();
    Position::new(
        u32::try_from(line).expect("line"),
        u32::try_from(column).expect("column"),
    )
}

/// `position`, `columns` further right on its line (ASCII fixtures only).
fn shifted(position: Position, columns: u32) -> Position {
    Position::new(position.line, position.character.saturating_add(columns))
}

/// Apply one text edit to `source`. The fixtures are ASCII on every edited
/// line, so a UTF-16 column is a byte column.
fn apply_edit(source: &str, edit: &lsp_types::TextEdit) -> String {
    let mut lines: Vec<String> = source.lines().map(str::to_owned).collect();
    if source.ends_with('\n') {
        lines.push(String::new());
    }
    let (row, start, end) = (
        usize::try_from(edit.range.start.line).expect("line"),
        usize::try_from(edit.range.start.character).expect("character"),
        usize::try_from(edit.range.end.character).expect("character"),
    );
    let line = lines.get_mut(row).expect("edited line exists");
    line.replace_range(start..end, &edit.new_text);
    lines.join("\n")
}

/// References params boilerplate.
fn reference_params(uri: &Uri, position: Position, include_declaration: bool) -> ReferenceParams {
    ReferenceParams {
        text_document_position: position_params(uri, position),
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext { include_declaration },
    }
}

/// Rename params boilerplate.
fn rename_params(uri: &Uri, position: Position, new_name: &str) -> RenameParams {
    RenameParams {
        text_document_position: position_params(uri, position),
        new_name: new_name.to_owned(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    }
}

#[test]
fn references_find_the_declaration_and_every_use() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    // The cursor asks from a use, and the declaration answers with itself.
    for cursor in [
        at(GLASS_MOUNTAIN, "sigh(e5);"),
        shifted(at(GLASS_MOUNTAIN, "motif sigh"), 6),
    ] {
        let answer = server
            .client
            .request::<References>(reference_params(&uri, cursor, true));
        let locations: Vec<Location> = serde_json::from_value(answer).expect("locations");
        let mut lines: Vec<u32> = locations.iter().map(|location| location.range.start.line).collect();
        lines.sort_unstable();
        let mut expected: Vec<u32> = [
            at(GLASS_MOUNTAIN, "motif sigh").line,
            at(GLASS_MOUNTAIN, "sigh(e5);").line,
            at_last(GLASS_MOUNTAIN, "sigh(e5);").line,
        ]
        .into_iter()
        .collect();
        expected.sort_unstable();
        assert_eq!(lines, expected, "references from {cursor:?}");
        // The spans are the *name tokens'* spans: four characters of `sigh`,
        // never the whole statement.
        for location in &locations {
            assert_eq!(location.range.end.character - location.range.start.character, 4);
        }
    }
    server.stop();
}

#[test]
fn references_can_exclude_the_declaration() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let answer = server
        .client
        .request::<References>(reference_params(&uri, at(GLASS_MOUNTAIN, "sigh(e5);"), false));
    let locations: Vec<Location> = serde_json::from_value(answer).expect("locations");
    assert_eq!(locations.len(), 2, "the two uses only: {locations:?}");
    server.stop();
}

#[test]
fn references_on_an_instrument_find_the_declaration_and_assignments() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let answer = server.client.request::<References>(reference_params(
        &uri,
        shifted(at(GLASS_MOUNTAIN, "instrument glass_pad"), 11),
        true,
    ));
    let locations: Vec<Location> = serde_json::from_value(answer).expect("locations");
    let mut lines: Vec<u32> = locations.iter().map(|location| location.range.start.line).collect();
    lines.sort_unstable();
    // Both parts are assigned to the pad: one declaration, two uses. The
    // `modulate lfo -> glass_pad.lowpass.cutoff` property path is not one.
    let mut expected: Vec<u32> = [
        at(GLASS_MOUNTAIN, "instrument glass_pad").line,
        at(GLASS_MOUNTAIN, "assign violin -> glass_pad").line,
        at(GLASS_MOUNTAIN, "assign strings -> glass_pad").line,
    ]
    .into_iter()
    .collect();
    expected.sort_unstable();
    assert_eq!(lines, expected, "{locations:?}");
    server.stop();
}

#[test]
fn prepare_rename_names_the_name_and_refuses_plain_text() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let answer = server
        .client
        .request::<PrepareRenameRequest>(position_params(&uri, at(GLASS_MOUNTAIN, "sigh(e5);")));
    let answer: Option<PrepareRenameResponse> = serde_json::from_value(answer).expect("prepare rename");
    let Some(PrepareRenameResponse::RangeWithPlaceholder { range, placeholder }) = answer else {
        panic!("expected a range with the name: {answer:?}");
    };
    assert_eq!(placeholder, "sigh");
    assert_eq!(range.start, at(GLASS_MOUNTAIN, "sigh(e5);"));
    assert_eq!(range.end, shifted(at(GLASS_MOUNTAIN, "sigh(e5);"), 4));
    // On a keyword there is nothing to prepare: null, the lawful answer.
    let answer = server
        .client
        .request::<PrepareRenameRequest>(position_params(&uri, at(GLASS_MOUNTAIN, "tempo")));
    assert!(answer.is_null(), "expected null on `tempo`: {answer}");
    server.stop();
}

#[test]
fn bundled_names_keep_source_maps_docs_and_read_only_identity() {
    let mut server = Server::start();
    let (uri, published) = server.open("stdlib", STDLIB_PIECE);
    assert!(published.diagnostics.is_empty(), "{published:?}");
    let position = at(STDLIB_PIECE, "identity_nat");

    let definition = server
        .client
        .request::<GotoDefinition>(lsp_types::GotoDefinitionParams {
            text_document_position_params: position_params(&uri, position),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });
    let definition: GotoDefinitionResponse = serde_json::from_value(definition).expect("a definition");
    let GotoDefinitionResponse::Scalar(location) = definition else {
        panic!("expected one definition");
    };
    assert_eq!(location.uri.as_str(), "musa-stdlib:/std/core.musa");

    let references = server
        .client
        .request::<References>(reference_params(&uri, position, true));
    let references: Vec<Location> = serde_json::from_value(references).expect("references");
    assert_eq!(
        references.len(),
        2,
        "external declaration and local use: {references:?}"
    );
    assert!(
        references
            .iter()
            .any(|location| location.uri.as_str() == "musa-stdlib:/std/core.musa")
    );

    let hover = server.client.request::<HoverRequest>(HoverParams {
        text_document_position_params: position_params(&uri, position),
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let hover: Hover = serde_json::from_value(hover).expect("hover");
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markdown");
    };
    assert!(content.value.contains("fn identity_nat"), "{}", content.value);
    assert!(content.value.contains("read-only"), "{}", content.value);
    // A link into the bundled source, at the line the declaration is on: the
    // text is compiled into the binary, so the link is the only way there.
    assert!(
        content.value.contains("(musa-stdlib:/std/core.musa#L"),
        "{}",
        content.value
    );

    let prepared = server
        .client
        .request::<PrepareRenameRequest>(position_params(&uri, position));
    assert!(prepared.is_null(), "bundled source must not prepare rename: {prepared}");
    let response = server
        .client
        .response::<Rename>(rename_params(&uri, position, "my_identity"));
    let error = response.response_result.expect_err("bundled rename must be refused");
    assert!(error.message.contains("read-only"), "{}", error.message);
    server.stop();
}

#[test]
fn exact_package_definitions_open_as_read_only_virtual_source() {
    let write = |path: &std::path::Path, text: &str| {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("fixture directory");
        }
        std::fs::write(path, text).expect("fixture file");
    };
    let git = |root: &std::path::Path, args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("Git fixture command");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).expect("Git UTF-8").trim().to_owned()
    };
    let remote = tempfile::tempdir().expect("remote");
    write(
        &remote.path().join("musa.toml"),
        "[package]\nname = \"tools\"\nlanguage_version = 1\n\n[build]\nsource = \"src\"\n",
    );
    write(&remote.path().join("src/lib.musa"), "mod math;\n");
    write(
        &remote.path().join("src/math.musa"),
        "fn doubled(n: Nat) -> Nat { n + n }\n",
    );
    git(remote.path(), &["init", "--quiet"]);
    git(remote.path(), &["config", "user.name", "Musa Test"]);
    git(remote.path(), &["config", "user.email", "musa@example.invalid"]);
    git(remote.path(), &["add", "."]);
    git(remote.path(), &["commit", "--quiet", "-m", "fixture"]);
    let revision = git(remote.path(), &["rev-parse", "HEAD"]);

    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("musa.toml"),
        &format!(
            "[project]\nname = \"LSP package\"\n\n[packages.tools]\ngit = \"{}\"\nrev = \"sha1:{revision}\"\n",
            remote.path().display()
        ),
    );
    let source = "piece \"Package\" {\n    import tools::math;\n    let four: Nat = doubled(2);\n    score { part p { voice v { c4/1 } } }\n}\n";
    let piece = project.path().join("piece.musa");
    write(&piece, source);
    musa_project::fetch_packages(project.path()).expect("fetch exact package");

    let mut server = Server::start();
    let uri = Uri::from_str(&format!("file://{}", piece.display())).expect("piece URI");
    server.client.notify::<DidOpenTextDocument>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "musa".to_owned(),
            version: 1,
            text: source.to_owned(),
        },
    });
    let published = server.client.notification::<PublishDiagnostics>();
    assert!(published.diagnostics.is_empty(), "{published:?}");
    let definition = server
        .client
        .request::<GotoDefinition>(lsp_types::GotoDefinitionParams {
            text_document_position_params: position_params(&uri, at(source, "doubled")),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });
    let definition: GotoDefinitionResponse = serde_json::from_value(definition).expect("definition");
    let GotoDefinitionResponse::Scalar(location) = definition else {
        panic!("expected one definition");
    };
    assert!(location.uri.as_str().starts_with("musa-package:/"));
    let answer = server.client.request::<ExecuteCommand>(ExecuteCommandParams {
        command: "musa.bundledSource".to_owned(),
        arguments: vec![serde_json::Value::String(location.uri.to_string())],
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    assert_eq!(answer.as_str(), Some("fn doubled(n: Nat) -> Nat { n + n }\n"));
    server.stop();
}

#[test]
fn rename_rewrites_exactly_the_recorded_spans() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let answer = server
        .client
        .request::<Rename>(rename_params(&uri, at(GLASS_MOUNTAIN, "sigh(e5);"), "lament"));
    let edit: lsp_types::WorkspaceEdit = serde_json::from_value(answer).expect("a workspace edit");
    let changes = edit.changes.as_ref().expect("changes");
    let edits = changes.get(&uri).expect("edits for the document");
    assert_eq!(changes.len(), 1, "one document, one entry: {changes:?}");
    assert_eq!(edits.len(), 3, "declaration plus two uses: {edits:?}");
    let mut text = GLASS_MOUNTAIN.to_owned();
    // Apply from the bottom up so earlier edits' offsets stay true.
    let mut ordered: Vec<lsp_types::TextEdit> = edits.clone();
    ordered.sort_by_key(|edit| std::cmp::Reverse(edit.range.start));
    for edit in &ordered {
        assert_eq!(edit.new_text, "lament");
        text = apply_edit(&text, edit);
    }
    assert_eq!(text, GLASS_MOUNTAIN.replace("sigh", "lament"));
    server.stop();
}

#[test]
fn rename_refuses_a_collision_in_the_namespace() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    // `violin` and `strings` are both parts: renaming one to the other is
    // the resolver's duplicates error, pre-empted.
    let response = server.client.response::<Rename>(rename_params(
        &uri,
        shifted(at(GLASS_MOUNTAIN, "part violin"), 5),
        "strings",
    ));
    let error = response.response_result.expect_err("the rename must be refused");
    assert!(error.message.contains("already names"), "{}", error.message);
    server.stop();
}

#[test]
fn rename_refuses_an_illegal_name_and_a_nameless_position() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let response = server
        .client
        .response::<Rename>(rename_params(&uri, at(GLASS_MOUNTAIN, "sigh(e5);"), "4x"));
    let error = response.response_result.expect_err("`4x` is not a name");
    assert!(error.message.contains("not a valid name"), "{}", error.message);
    // `4x` must not have edited anything: the document still compiles.
    let response = server
        .client
        .response::<Rename>(rename_params(&uri, at(GLASS_MOUNTAIN, "tempo"), "beat"));
    let error = response.response_result.expect_err("there is no name under `tempo`");
    assert!(error.message.contains("no named thing"), "{}", error.message);
    server.stop();
}

#[test]
fn a_broken_document_still_answers_references() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    // Break the tail — an unterminated string after the piece's end — so
    // the compile fails while every `sigh` position keeps its byte offset.
    let broken = format!("{GLASS_MOUNTAIN}\nsubtitle \"oops");
    let published = server.change(&uri, broken);
    assert!(!published.diagnostics.is_empty(), "the edit should not compile");
    let answer = server
        .client
        .request::<References>(reference_params(&uri, at(GLASS_MOUNTAIN, "sigh(e5);"), true));
    let locations: Vec<Location> = serde_json::from_value(answer).expect("locations");
    assert_eq!(locations.len(), 3, "stale-but-honest references: {locations:?}");
    server.stop();
}

#[test]
fn handshake_advertises_the_feature_set() {
    let server = Server::start();
    let result: lsp_types::InitializeResult =
        serde_json::from_value(server.capabilities.clone()).expect("initialize result");
    let capabilities = result.capabilities;
    assert!(matches!(
        capabilities.text_document_sync,
        Some(lsp_types::TextDocumentSyncCapability::Kind(
            lsp_types::TextDocumentSyncKind::FULL
        ))
    ));
    assert!(capabilities.hover_provider.is_some());
    assert!(capabilities.definition_provider.is_some());
    assert!(capabilities.document_symbol_provider.is_some());
    assert!(capabilities.code_action_provider.is_some());
    assert!(capabilities.document_formatting_provider.is_some());
    assert!(capabilities.completion_provider.is_some());
    assert!(capabilities.semantic_tokens_provider.is_some());
    assert!(capabilities.signature_help_provider.is_some());
    assert!(capabilities.code_lens_provider.is_some());
    let commands = capabilities
        .execute_command_provider
        .expect("the analysis command is executable");
    assert!(commands.commands.contains(&"musa.analyze".to_owned()), "{commands:?}");
    assert!(
        commands.commands.contains(&"musa.bundledSource".to_owned()),
        "{commands:?}"
    );
    assert!(
        commands.commands.contains(&"musa.adapterEdit".to_owned()),
        "{commands:?}"
    );
    server.stop();
}

#[test]
fn opening_a_valid_piece_publishes_no_diagnostics() {
    let mut server = Server::start();
    let (_, published) = server.open("glass-mountain", GLASS_MOUNTAIN);
    assert!(
        published.diagnostics.is_empty(),
        "unexpected: {:?}",
        published.diagnostics
    );
    server.stop();
}

#[test]
fn a_broken_file_publishes_teaching_diagnostics() {
    let mut server = Server::start();
    let (_, published) = server.open("missing-semicolon", MISSING_SEMICOLON);
    let [diagnostic] = published.diagnostics.as_slice() else {
        panic!("expected exactly one diagnostic: {:?}", published.diagnostics);
    };
    assert_eq!(
        diagnostic.code,
        Some(lsp_types::NumberOrString::String("syntax".to_owned()))
    );
    assert!(diagnostic.message.contains("missing `;`"), "{}", diagnostic.message);
    assert_eq!(diagnostic.severity, Some(lsp_types::DiagnosticSeverity::ERROR));
    server.stop();
}

#[test]
fn the_certain_fix_arrives_as_a_quick_fix() {
    let mut server = Server::start();
    let (uri, published) = server.open("missing-semicolon", MISSING_SEMICOLON);
    let diagnostic = published.diagnostics.first().expect("a diagnostic");
    let actions = server.client.request::<CodeActionRequest>(CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: diagnostic.range,
        context: lsp_types::CodeActionContext {
            diagnostics: vec![diagnostic.clone()],
            only: Some(vec![CodeActionKind::QUICKFIX]),
            trigger_kind: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    });
    let actions: CodeActionResponse = serde_json::from_value(actions).expect("code actions");
    let [CodeActionOrCommand::CodeAction(action)] = actions.as_slice() else {
        panic!("expected exactly one quick fix: {actions:?}");
    };
    assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
    assert_eq!(action.title, "add `;`");
    let edit = action.edit.as_ref().expect("a workspace edit");
    let edits = edit
        .changes
        .as_ref()
        .and_then(|changes| changes.get(&uri))
        .expect("edits for the document");
    let [edit] = edits.as_slice() else {
        panic!("expected one text edit: {edits:?}");
    };
    assert_eq!(edit.new_text, ";");
    server.stop();
}

#[test]
fn proved_barlines_arrive_as_one_utf16_safe_rewrite() {
    let source = HOVER_PIECE.replacen("piece \"Hover\" {", "piece \"Hover 🎵\" {", 1);
    let expected = musa_project::ProjectSession::from_text(&source, "expected.musa")
        .barline_rewrite()
        .expect("valid plan")
        .expect("one complete loose measure")
        .source()
        .to_owned();
    let mut server = Server::start();
    let (uri, _) = server.open("barline-utf16", &source);
    let actions = server.client.request::<CodeActionRequest>(CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: lsp_types::Range::new(Position::new(0, 0), Position::new(0, 0)),
        context: lsp_types::CodeActionContext {
            diagnostics: Vec::new(),
            only: Some(vec![CodeActionKind::REFACTOR_REWRITE]),
            trigger_kind: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    });
    let actions: CodeActionResponse = serde_json::from_value(actions).expect("code actions");
    let [CodeActionOrCommand::CodeAction(action)] = actions.as_slice() else {
        panic!("expected exactly one rewrite: {actions:?}");
    };
    assert_eq!(
        action.kind.as_ref().map(CodeActionKind::as_str),
        Some("refactor.rewrite.insertBarlines")
    );
    let edits = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .and_then(|changes| changes.get(&uri))
        .expect("document edits");
    let [edit] = edits.as_slice() else {
        panic!("one transactional edit: {edits:?}");
    };
    assert_eq!(edit.new_text, expected);
    assert_eq!(edit.range.start, Position::new(0, 0));
    assert_eq!(edit.range.end, utf16_end(&source));
    server.stop();
}

fn utf16_end(source: &str) -> Position {
    let line = u32::try_from(source.split('\n').count().saturating_sub(1)).unwrap_or(u32::MAX);
    let tail = source.rsplit_once('\n').map_or(source, |(_, tail)| tail);
    Position::new(line, u32::try_from(tail.encode_utf16().count()).unwrap_or(u32::MAX))
}

#[test]
fn fixing_the_source_clears_the_diagnostics() {
    let mut server = Server::start();
    let (uri, published) = server.open("missing-semicolon", MISSING_SEMICOLON);
    assert_eq!(published.diagnostics.len(), 1);
    let fixed = MISSING_SEMICOLON.replacen("clef treble\n", "clef treble;\n", 1);
    assert_ne!(fixed, MISSING_SEMICOLON);
    let published = server.change(&uri, fixed);
    assert!(
        published.diagnostics.is_empty(),
        "unexpected: {:?}",
        published.diagnostics
    );
    server.stop();
}

#[test]
fn hover_on_a_note_reports_the_music() {
    let mut server = Server::start();
    let (uri, published) = server.open("hover", HOVER_PIECE);
    assert!(published.diagnostics.is_empty(), "the piece should compile");
    let hover = server.client.request::<HoverRequest>(HoverParams {
        text_document_position_params: position_params(&uri, at(HOVER_PIECE, "c4")),
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let hover: Hover = serde_json::from_value(hover).expect("a hover");
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markdown hover");
    };
    assert!(content.value.contains("**C4** — 1/4"), "{}", content.value);
    assert!(content.value.contains("bar 1 · beat 1"), "{}", content.value);
    assert!(content.value.contains("C major"), "{}", content.value);
    server.stop();
}

#[test]
fn hover_on_a_use_site_reports_the_expansion() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let hover = server.client.request::<HoverRequest>(HoverParams {
        text_document_position_params: position_params(&uri, at(GLASS_MOUNTAIN, "use sigh")),
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let hover: Hover = serde_json::from_value(hover).expect("a hover");
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markdown hover");
    };
    assert!(content.value.contains("sigh(e5)"), "{}", content.value);
    assert!(content.value.contains("motif `sigh`"), "{}", content.value);
    server.stop();
}

#[test]
fn definition_on_a_use_lands_on_the_motif() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let definition = server
        .client
        .request::<GotoDefinition>(lsp_types::GotoDefinitionParams {
            text_document_position_params: position_params(&uri, at(GLASS_MOUNTAIN, "use sigh")),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });
    let definition: GotoDefinitionResponse = serde_json::from_value(definition).expect("a definition");
    let GotoDefinitionResponse::Scalar(location) = definition else {
        panic!("expected one location: {definition:?}");
    };
    assert_eq!(location.uri, uri);
    assert_eq!(location.range.start.line, at(GLASS_MOUNTAIN, "motif sigh").line);
    server.stop();
}

#[test]
fn symbols_list_the_outline() {
    let mut server = Server::start();
    let (uri, _) = server.open("annotated", ANNOTATED);
    let symbols = server.client.request::<DocumentSymbolRequest>(DocumentSymbolParams {
        text_document: TextDocumentIdentifier { uri },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    });
    let symbols: DocumentSymbolResponse = serde_json::from_value(symbols).expect("symbols");
    let DocumentSymbolResponse::Flat(symbols) = symbols else {
        panic!("expected flat symbols");
    };
    let names: Vec<(&str, SymbolKind)> = symbols
        .iter()
        .map(|symbol| (symbol.name.as_str(), symbol.kind))
        .collect();
    assert!(names.contains(&("Exposition", SymbolKind::NAMESPACE)), "{names:?}");
    assert!(names.contains(&("antecedent", SymbolKind::FUNCTION)), "{names:?}");
    server.stop();
}

#[test]
fn semantic_tokens_cover_a_broken_document() {
    // A document mid-edit: an unrecognized span, an unterminated string. The
    // lexer is total, so the tokens are too.
    let broken =
        "piece \"x\" {\n    fn choose(x: Option(Nat)) -> Nat = match x { None ->\n    @@ mid-edit \"unterminated\n}\n";
    let mut server = Server::start();
    let (uri, _) = server.open("broken", broken);
    let tokens = server
        .client
        .request::<SemanticTokensFullRequest>(SemanticTokensParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });
    let tokens: SemanticTokensResult = serde_json::from_value(tokens).expect("semantic tokens");
    let SemanticTokensResult::Tokens(tokens) = tokens else {
        panic!("expected tokens");
    };
    assert!(!tokens.data.is_empty());
    // The very first token is `piece`: a keyword of length 5 at the origin.
    let first = tokens.data.first().expect("a first token");
    assert_eq!((first.delta_line, first.delta_start, first.length), (0, 0, 5));
    assert_eq!(first.token_type, 1, "keyword is legend index 1");
    assert!(
        tokens.data.iter().filter(|token| token.token_type == 1).count() >= 5,
        "fn, Option, match, and None stay keywords even in incomplete source"
    );
    for token in &tokens.data {
        assert!(token.token_type < 9, "token type within the legend");
    }
    server.stop();
}

#[test]
fn completion_offers_the_vocabulary_and_the_names() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let completions = server.client.request::<Completion>(CompletionParams {
        text_document_position: position_params(&uri, Position::new(0, 0)),
        work_done_progress_params: WorkDoneProgressParams::default(),
        context: None,
        partial_result_params: PartialResultParams::default(),
    });
    let completions: CompletionResponse = serde_json::from_value(completions).expect("completions");
    let CompletionResponse::Array(items) = completions else {
        panic!("expected a completion list");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    for expected in ["piece", "transpose", "voice"] {
        assert!(labels.contains(&expected), "keyword `{expected}` missing");
    }
    for expected in ["sigh", "violin", "strings"] {
        assert!(labels.contains(&expected), "name `{expected}` missing");
    }
    // The menu teaches from the authoritative vocabulary: language keywords
    // carry their syntax docs and studio units carry their typed catalogue
    // entry rather than a second hard-coded description.
    let tempo = items.iter().find(|item| item.label == "tempo").expect("tempo item");
    assert_eq!(tempo.detail.as_deref(), Some("how fast, written where it changes"));
    let Some(lsp_types::Documentation::MarkupContent(content)) = &tempo.documentation else {
        panic!("tempo should carry markdown documentation");
    };
    assert!(content.value.contains("```musa"), "{}", content.value);
    let hz = items.iter().find(|item| item.label == "Hz").expect("Hz item");
    assert_eq!(hz.detail.as_deref(), Some("Number Hz"));
    let Some(lsp_types::Documentation::MarkupContent(content)) = &hz.documentation else {
        panic!("Hz should carry catalogue documentation");
    };
    assert!(
        content.value.contains("Hertz measure cycles per second"),
        "{}",
        content.value
    );
    assert!(content.value.contains("bundled Musa source"), "{}", content.value);
    server.stop();
}

#[test]
fn formatting_is_one_whole_document_edit_and_idempotent() {
    let messy = "piece  \"Messy\"  {\n     meter 4/4;\n  tempo 1/4 = 96;\n}\n";
    let mut server = Server::start();
    let (uri, _) = server.open("messy", messy);
    let format = |client: &mut Client, uri: &Uri| {
        client.request::<Formatting>(lsp_types::DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            options: lsp_types::FormattingOptions::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
    };
    let edits: Vec<lsp_types::TextEdit> =
        serde_json::from_value(format(&mut server.client, &uri)).expect("formatting edits");
    let [edit] = edits.as_slice() else {
        panic!("expected one whole-document edit: {edits:?}");
    };
    assert_eq!(edit.range.start, Position::new(0, 0));
    assert_ne!(edit.new_text, messy);
    // Applying the edit and formatting again is the identity — the
    // formatter's own law (roadmap §17.3), seen through the protocol.
    server.change(&uri, edit.new_text.clone());
    let again = format(&mut server.client, &uri);
    assert!(again.is_null(), "formatting a canonical source should be null: {again}");
    server.stop();
}

/// Ask the server for `name`'s folds, opened from `source`.
fn folds(server: &mut Server, name: &str, source: &str) -> Vec<FoldingRange> {
    let (uri, _) = server.open(name, source);
    let answer = server.client.request::<FoldingRangeRequest>(FoldingRangeParams {
        text_document: TextDocumentIdentifier { uri },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    });
    serde_json::from_value::<Vec<FoldingRange>>(answer).expect("folding ranges")
}

#[test]
fn every_example_and_every_broken_fixture_folds() {
    // The fixtures are the executable specification; the test reads the
    // directory so a fixture added later joins the law without an edit here.
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut sources: Vec<(String, String)> = Vec::new();
    for directory in [examples.clone(), examples.join("broken")] {
        let mut entries: Vec<_> = std::fs::read_dir(&directory)
            .expect("examples directory")
            .map(|entry| entry.expect("directory entry").path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "musa"))
            .collect();
        entries.sort();
        for path in entries {
            let name = path.file_stem().expect("file name").to_string_lossy().into_owned();
            sources.push((name, std::fs::read_to_string(&path).expect("fixture text")));
        }
    }
    assert!(sources.len() > 20, "the suite of fixtures itself: {}", sources.len());
    let mut server = Server::start();
    for (name, source) in &sources {
        let ranges = folds(&mut server, name, source);
        let mut previous = 0;
        for (at, range) in ranges.iter().enumerate() {
            assert!(range.start_line < range.end_line, "{name}: fold {at} spans lines");
            assert!(range.start_line >= previous || at == 0, "{name}: folds sorted");
            previous = range.start_line;
        }
        // Every fixture closes at least one block except the one whose point
        // is that nothing closes.
        if !name.starts_with("unclosed") {
            assert!(!ranges.is_empty(), "{name}: a piece is at least its own block");
        }
    }
    server.stop();
}

#[test]
fn folds_are_the_brace_blocks_and_the_comment_runs() {
    let mut server = Server::start();
    let ranges = folds(&mut server, "glass-mountain", GLASS_MOUNTAIN);
    let starts: Vec<(u32, Option<FoldingRangeKind>)> = ranges
        .iter()
        .map(|range| (range.start_line, range.kind.clone()))
        .collect();
    // The Check's promise: a fold per part, per motif, and the studio block.
    for needle in ["motif sigh", "part violin", "part strings", "studio {"] {
        let line = at(GLASS_MOUNTAIN, needle).line;
        assert!(
            starts.contains(&(line, Some(FoldingRangeKind::Region))),
            "no region fold at `{needle}` (line {line}): {starts:?}"
        );
    }
    // The two three-line comment runs fold as comments.
    for needle in ["// Under the pedal", "// The sound side"] {
        let line = at(GLASS_MOUNTAIN, needle).line;
        assert!(
            starts.contains(&(line, Some(FoldingRangeKind::Comment))),
            "no comment fold at `{needle}` (line {line}): {starts:?}"
        );
    }
    // Every brace pair in the fixture spans lines: fourteen region folds and
    // two comment runs, no more.
    let regions = ranges
        .iter()
        .filter(|range| range.kind == Some(FoldingRangeKind::Region))
        .count();
    assert_eq!(regions, 14, "{starts:?}");
    server.stop();
}

#[test]
fn single_line_blocks_and_string_braces_do_not_fold() {
    let source = "piece \"a { not a block\" {\n    motif m { c4/4 }\n    score {\n        part p {\n            voice v {\n                c4/4 e4/4 g4/4 c4/4\n            }\n        }\n    }\n}\n// one comment only\n";
    let mut server = Server::start();
    let ranges = folds(&mut server, "single-line", source);
    // The piece, score, part, and voice fold; the single-line motif does
    // not, the `{` inside the title is not a block, and one comment line is
    // not a run.
    let regions: Vec<u32> = ranges
        .iter()
        .filter(|range| range.kind == Some(FoldingRangeKind::Region))
        .map(|range| range.start_line)
        .collect();
    assert_eq!(regions, vec![0, 2, 3, 4], "{ranges:?}");
    assert!(
        !ranges.iter().any(|range| range.kind == Some(FoldingRangeKind::Comment)),
        "a single comment line is not a run: {ranges:?}"
    );
    server.stop();
}

#[test]
fn an_unclosed_block_offers_no_fold() {
    let unclosed = include_str!("../../../../examples/broken/unclosed-block.musa");
    let mut server = Server::start();
    let ranges = folds(&mut server, "unclosed-block", unclosed);
    // Every block in the fixture is left open, and there is no closing line
    // to fold to — so there is nothing to offer.
    assert!(
        !ranges.iter().any(|range| range.kind == Some(FoldingRangeKind::Region)),
        "unclosed blocks fold nothing: {ranges:?}"
    );
    server.stop();
}

/// A piece with one name nobody speaks: the lint warning is a diagnostic
/// like any other, and its certain fix is a quick fix like any other — the
/// server carries it with no new plumbing, which is the point this test pins.
const UNUSED_MOTIF_PIECE: &str = "piece \"Lint\" {
    tempo 1/4 = 96;
    meter 4/4;
    key c major;

    motif answer() {
        g4/4 a4/4 e4/4 f4/4
    }

    score {
        part piano {
            voice right {
                bar { c4/4 d4/4 e4/4 f4/4 }
            }
        }
    }
}
";

#[test]
fn a_lint_warning_publishes_with_its_quick_fix() {
    let mut server = Server::start();
    let (uri, published) = server.open("unused-motif", UNUSED_MOTIF_PIECE);
    let [diagnostic] = published.diagnostics.as_slice() else {
        panic!("expected exactly one diagnostic: {:?}", published.diagnostics);
    };
    assert_eq!(
        diagnostic.code,
        Some(lsp_types::NumberOrString::String("unused-material".to_owned()))
    );
    assert_eq!(diagnostic.severity, Some(lsp_types::DiagnosticSeverity::WARNING));
    let actions = server.client.request::<CodeActionRequest>(CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        range: diagnostic.range,
        context: lsp_types::CodeActionContext {
            diagnostics: vec![diagnostic.clone()],
            only: Some(vec![CodeActionKind::QUICKFIX]),
            trigger_kind: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    });
    let actions: CodeActionResponse = serde_json::from_value(actions).expect("code actions");
    let [CodeActionOrCommand::CodeAction(action)] = actions.as_slice() else {
        panic!("expected exactly one quick fix: {actions:?}");
    };
    assert_eq!(action.title, "delete this motif");
    let edit = action.edit.as_ref().expect("a workspace edit");
    let edits = edit
        .changes
        .as_ref()
        .and_then(|changes| changes.get(&uri))
        .expect("edits for the document");
    let [edit] = edits.as_slice() else {
        panic!("expected one text edit: {edits:?}");
    };
    // The fix deletes the declaration's lines: from the `motif` keyword's
    // line to just past the closing brace's line.
    assert_eq!(edit.range.start, Position::new(5, 0));
    assert_eq!(edit.range.end, Position::new(8, 0));
    assert_eq!(edit.new_text, "");
    server.stop();
}

#[test]
fn hover_on_a_keyword_reports_its_documentation() {
    let mut server = Server::start();
    let (uri, published) = server.open("hover", HOVER_PIECE);
    assert!(published.diagnostics.is_empty(), "the piece should compile");
    let hover = server.client.request::<HoverRequest>(HoverParams {
        text_document_position_params: position_params(&uri, at(HOVER_PIECE, "tempo")),
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let hover: Hover = serde_json::from_value(hover).expect("a hover");
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markdown hover");
    };
    assert!(content.value.contains("**tempo**"), "{}", content.value);
    assert!(content.value.contains("where it changes"), "{}", content.value);
    assert!(content.value.contains("```musa"), "{}", content.value);
    server.stop();
}

#[test]
fn hover_on_a_controlled_music_function_explains_its_boundary() {
    let source = "piece \"hover builtin\" {
        let subject: EventTrack(WrittenTime) = music { c4/1 };
        fn same(p: Pitch) -> Pitch { p }
        let transformed: EventTrack(WrittenTime) = map_note_pitches(same, subject);
        score { part p { voice v { use transformed; } } }
    }";
    let mut server = Server::start();
    let (uri, published) = server.open("hover-builtin", source);
    assert!(published.diagnostics.is_empty(), "the piece should compile");
    let hover = server.client.request::<HoverRequest>(HoverParams {
        text_document_position_params: position_params(&uri, at(source, "map_note_pitches")),
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let hover: Hover = serde_json::from_value(hover).expect("a hover");
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markdown hover");
    };
    assert!(content.value.contains("**map_note_pitches**"), "{}", content.value);
    assert!(content.value.contains("Key signatures"), "{}", content.value);
    server.stop();
}

#[test]
fn hover_on_a_declaration_reports_the_checked_signature_and_its_summary() {
    let mut server = Server::start();
    let (uri, published) = server.open("tooling", TOOLING);
    assert!(published.diagnostics.is_empty(), "{published:?}");
    let content = hover_markdown(&mut server, &uri, at(TOOLING, "lifted(what"));
    // The signature the checker settled on — not the text.
    assert!(
        content.contains("fn lifted(what: EventTrack(WrittenTime), by: Interval) -> EventTrack(WrittenTime)"),
        "{content}"
    );
    // The comment block above the declaration, as prose.
    assert!(content.contains("Raise a passage"), "{content}");
    server.stop();
}

#[test]
fn hover_on_an_sfz_instrument_reports_the_imported_contract_and_support() {
    let source = r#"instrument piano from "assets/piano.sfz" conforms note_instrument;
piece "SFZ hover" { score { part p { voice v { c4/1 } } } }
"#;
    let mut server = Server::start();
    let (uri, _) = server.open("sfz-hover", source);
    let content = hover_markdown(&mut server, &uri, at(source, "piano from"));
    assert!(content.contains("imported note instrument"), "{content}");
    assert!(content.contains("`sfz@1`"), "{content}");
    assert!(content.contains("techniques: ordinary"), "{content}");
    assert!(
        content.contains("Every unsupported sound-changing input is an error naming the input"),
        "{content}"
    );
    assert!(content.contains("`assets/piano.sfz` · undeclared"), "{content}");
    assert!(content.contains("registered SFZ support facts"), "{content}");
    server.stop();
}

#[test]
fn hover_on_a_soundfont_preset_reports_the_strict_adapter_boundary() {
    let source = r#"instrument piano from "assets/piano.sf2#preset=0:1" conforms note_instrument;
piece "SoundFont hover" { score { part p { voice v { c4/1 } } } }
"#;
    let mut server = Server::start();
    let (uri, _) = server.open("sf2-hover", source);
    let content = hover_markdown(&mut server, &uri, at(source, "piano from"));
    assert!(content.contains("`sf2@1`"), "{content}");
    assert!(content.contains("explicit preset selection"), "{content}");
    assert!(content.contains("exact global/local zone combination"), "{content}");
    assert!(content.contains("ambient MIDI controllers"), "{content}");
    assert!(content.contains("`assets/piano.sf2` · undeclared"), "{content}");
    server.stop();
}

#[test]
fn hover_draws_the_distinction_between_a_domain_and_the_one_it_is_confused_with() {
    // `NoteName` and `Pc(12)` are both "a pitch class" in ordinary speech and
    // are different objects here. A hover that named only the type would let
    // a reader carry the confusion; the sentence is the compiler's own.
    let mut server = Server::start();
    let (uri, _) = server.open("tooling", TOOLING);
    let content = hover_markdown(&mut server, &uri, at(TOOLING, "centre: NoteName"));
    assert!(content.contains("let centre: NoteName"), "{content}");
    assert!(
        content.contains("C\u{266f} and D\u{266d} are two"),
        "the spelled/modulo-twelve distinction: {content}"
    );
    server.stop();
}

#[test]
fn hover_marks_a_deprecated_binding_with_what_to_write_instead() {
    let mut server = Server::start();
    let (uri, _) = server.open("tooling", TOOLING);
    let content = hover_markdown(&mut server, &uri, at(TOOLING, "theme: EventTrack(WrittenTime)"));
    assert!(content.contains("**Deprecated**"), "{content}");
    assert!(content.contains("write `subject` instead"), "{content}");
    server.stop();
}

#[test]
fn signature_help_names_the_parameter_the_caret_is_on() {
    let mut server = Server::start();
    let (uri, _) = server.open("tooling", TOOLING);
    let call = at(TOOLING, "lifted(subject");
    // Inside the first argument.
    let help = signature_help(&mut server, &uri, shifted(call, 7));
    let signature = help.signatures.first().expect("one signature");
    assert_eq!(
        signature.label,
        "fn lifted(what: EventTrack(WrittenTime), by: Interval) -> EventTrack(WrittenTime)"
    );
    let parameters = signature.parameters.as_ref().expect("parameters");
    let labels: Vec<&str> = parameters
        .iter()
        .map(|parameter| match &parameter.label {
            lsp_types::ParameterLabel::Simple(label) => label.as_str(),
            lsp_types::ParameterLabel::LabelOffsets(_) => panic!("expected simple labels"),
        })
        .collect();
    assert_eq!(
        labels,
        ["what: EventTrack(WrittenTime)", "by: Interval"],
        "{parameters:?}"
    );
    assert_eq!(help.active_parameter, Some(0));
    // Past the comma, the second.
    let help = signature_help(&mut server, &uri, shifted(call, 16));
    assert_eq!(help.active_parameter, Some(1));
    server.stop();
}

#[test]
fn signature_help_answers_for_a_claim_from_the_compilers_own_registry() {
    let mut server = Server::start();
    let (uri, _) = server.open("tooling", TOOLING);
    let claim = at(TOOLING, "realizes(chord");
    let help = signature_help(&mut server, &uri, shifted(claim, 9));
    let signature = help.signatures.first().expect("one signature");
    assert!(signature.label.starts_with("realizes("), "{}", signature.label);
    let parameters = signature.parameters.as_ref().expect("parameters");
    assert_eq!(parameters.len(), 2, "{parameters:?}");
    // The policy argument, past the comma.
    let help = signature_help(&mut server, &uri, shifted(claim, 24));
    assert_eq!(help.active_parameter, Some(1));
    server.stop();
}

#[test]
fn studio_help_comes_from_the_catalogue_even_when_the_call_is_incomplete() {
    let mut server = Server::start();
    let (uri, _) = server.open("studio-tooling", STUDIO_TOOLING);

    let hover = hover_markdown(&mut server, &uri, at(STUDIO_TOOLING, "lowpass"));
    assert!(hover.contains("Keeps frequencies below a cutoff."), "{hover}");
    assert!(hover.contains("quality factor Q"), "{hover}");
    assert!(hover.contains("Origin: bundled Musa source"), "{hover}");

    let definition = server
        .client
        .request::<GotoDefinition>(lsp_types::GotoDefinitionParams {
            text_document_position_params: position_params(&uri, at(STUDIO_TOOLING, "lowpass")),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });
    let definition: GotoDefinitionResponse = serde_json::from_value(definition).expect("a source definition");
    let GotoDefinitionResponse::Scalar(location) = definition else {
        panic!("expected one location: {definition:?}");
    };
    assert_eq!(location.uri.as_str(), "musa-stdlib:/std/sound/catalogue.musa");
    let catalogue = musa_project::standard_library_source(location.uri.as_str()).expect("catalogue source");
    assert_eq!(
        location.range.start.line,
        at(catalogue, "studio_vocabulary").line,
        "the row belongs to the source artifact"
    );

    let call = at(STUDIO_TOOLING, "lowpass(cutoff");
    let help = signature_help(&mut server, &uri, shifted(call, 20));
    assert!(
        help.signatures
            .first()
            .expect("one studio signature")
            .label
            .contains("resonance: Ratio")
    );

    let items = completions(&mut server, &uri, shifted(call, 18));
    let resonance = items
        .iter()
        .find(|item| item.label == "resonance:")
        .expect("canonical parameter completion");
    assert!(resonance.sort_text.as_deref().is_some_and(|sort| sort.starts_with('0')));
    assert!(!items.iter().any(|item| item.label == "q:"));
    server.stop();
}

#[test]
fn sound_completion_comes_from_checked_instrument_and_project_facts() {
    let mut server = Server::start();
    let (uri, published) = server.open("sound-completion", GLASS_MOUNTAIN);
    assert!(
        published
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Some(lsp_types::DiagnosticSeverity::ERROR))
    );
    let items = completions(&mut server, &uri, at(GLASS_MOUNTAIN, "sound side"));

    let expression = items
        .iter()
        .find(|item| item.label == "expression")
        .expect("source-declared standard control");
    assert_eq!(expression.kind, Some(lsp_types::CompletionItemKind::PROPERTY));
    assert_eq!(expression.detail.as_deref(), Some("Normalized control · Continuous"));
    let documentation = expression.documentation.as_ref().expect("control origin");
    assert!(format!("{documentation:?}").contains("checked `std::sound::instrument` declaration"));

    assert!(items.iter().any(|item| item.label == "glass_pad"));
    let neutral = items
        .iter()
        .find(|item| item.label == "std.performance.neutral")
        .expect("effective source profile");
    assert_eq!(neutral.detail.as_deref(), Some("performance profile"));
    assert!(!items.iter().any(|item| item.label == "voice.gain"));
    server.stop();
}

#[test]
fn exposed_control_help_retains_the_source_index_and_navigates_to_its_declaration() {
    let source = "piece \"control help\" {
        score { part p { voice v { c4/1 } } }
        instrument tone conforms note_instrument {
            implementation graph { expression }
        }
    }";
    let mut server = Server::start();
    let (uri, _) = server.open("control-help", source);
    let position = at(source, "expression");
    let content = hover_markdown(&mut server, &uri, position);
    assert!(content.contains("**std.performance::expression**"), "{content}");
    assert!(content.contains("Kind: `Normalized` · rate: `Continuous`"), "{content}");
    assert!(content.contains("ordinary pattern unifier"), "{content}");

    let definition = server
        .client
        .request::<GotoDefinition>(lsp_types::GotoDefinitionParams {
            text_document_position_params: position_params(&uri, position),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });
    let definition: GotoDefinitionResponse = serde_json::from_value(definition).expect("a control definition");
    let GotoDefinitionResponse::Scalar(location) = definition else {
        panic!("expected one location: {definition:?}");
    };
    assert_eq!(location.uri.as_str(), "musa-stdlib:/std/performance.musa");
    let declaration = musa_project::standard_library_source(location.uri.as_str()).expect("performance source");
    assert_eq!(
        location.range.start.line,
        at(declaration, "expression: ControlKey").line,
    );
    server.stop();
}

#[test]
fn completion_offers_a_calls_parameter_names() {
    let mut server = Server::start();
    let (uri, _) = server.open("tooling", TOOLING);
    let items = completions(&mut server, &uri, shifted(at(TOOLING, "lifted(subject"), 7));
    let named = items.iter().find(|item| item.label == "by:").expect("`by:` offered");
    assert_eq!(named.detail.as_deref(), Some("Interval"));
    assert_eq!(named.kind, Some(lsp_types::CompletionItemKind::FIELD));
    assert!(
        named.sort_text.as_deref().is_some_and(|sort| sort.starts_with('0')),
        "the site sorts above the vocabulary: {named:?}"
    );
    // Nothing is taken away: the language's own words are still on offer.
    assert!(items.iter().any(|item| item.label == "meter"), "the vocabulary stays");
    server.stop();
}

#[test]
fn completion_in_a_policy_position_offers_the_three_policies() {
    let mut server = Server::start();
    let (uri, _) = server.open("tooling", TOOLING);
    let items = completions(&mut server, &uri, shifted(at(TOOLING, "realizes(chord"), 24));
    for policy in ["exactly", "may_omit", "may_add"] {
        let offered = items
            .iter()
            .find(|item| item.label == policy)
            .unwrap_or_else(|| panic!("`{policy}` missing: {items:?}"));
        assert_eq!(offered.kind, Some(lsp_types::CompletionItemKind::ENUM_MEMBER));
        assert!(
            offered.sort_text.as_deref().is_some_and(|sort| sort.starts_with('0')),
            "{offered:?}"
        );
    }
    server.stop();
}

#[test]
fn completion_in_a_events_hole_offers_only_what_a_hole_may_splice() {
    let mut server = Server::start();
    let (uri, published) = server.open("events-splice", EVENTS_SPLICE);
    assert!(published.diagnostics.is_empty(), "{published:?}");
    let items = completions(&mut server, &uri, at(EVENTS_SPLICE, "subject} in"));
    let spliced = items
        .iter()
        .find(|item| item.label == "subject")
        .expect("the music-typed binding");
    assert!(
        spliced.sort_text.as_deref().is_some_and(|sort| sort.starts_with('0')),
        "{spliced:?}"
    );
    // A hole splices music and nothing else, so every name the site itself
    // offered is music. The rest of the vocabulary is still there, unsorted.
    for item in items.iter().filter(|item| item.sort_text.is_some()) {
        assert!(
            item.detail
                .as_deref()
                .is_some_and(|detail| detail.contains("EventTrack(WrittenTime)")),
            "a hole may splice only music: {item:?}"
        );
    }
    server.stop();
}

#[test]
fn symbols_list_the_declarations_written_here_and_no_others() {
    let mut server = Server::start();
    let (uri, _) = server.open("tooling", TOOLING);
    let symbols = flat_symbols(&mut server, &uri);
    let names: Vec<&str> = symbols.iter().map(|symbol| symbol.name.as_str()).collect();
    for expected in [
        "record Centred: Type",
        "let home: Centred",
        "fn lifted(what: EventTrack(WrittenTime), by: Interval) -> EventTrack(WrittenTime)",
        "let subject: EventTrack(WrittenTime)",
    ] {
        assert!(names.contains(&expected), "`{expected}` missing: {names:?}");
    }
    let deprecated = symbols
        .iter()
        .find(|symbol| symbol.name.starts_with("let theme"))
        .expect("the deprecated binding");
    assert_eq!(deprecated.tags.as_deref(), Some([SymbolTag::DEPRECATED].as_slice()));
    // Sorted by where they are written, because that is how an outline reads.
    let mut positions: Vec<_> = symbols.iter().map(|symbol| symbol.location.range.start).collect();
    let ordered = positions.clone();
    positions.sort_by_key(|position| (position.line, position.character));
    assert_eq!(positions, ordered);

    // An imported declaration has a span in another document; an outline
    // entry for it would send every jump to an offset in the wrong file.
    let (uri, _) = server.open("stdlib", STDLIB_PIECE);
    let names: Vec<String> = flat_symbols(&mut server, &uri)
        .into_iter()
        .map(|symbol| symbol.name)
        .collect();
    assert!(names.iter().any(|name| name == "let answer: Nat"), "{names:?}");
    assert!(
        !names.iter().any(|name| name.contains("identity_nat")),
        "the bundled declaration is not written here: {names:?}"
    );
    server.stop();
}

#[test]
fn navigation_crosses_a_function_and_the_values_it_builds() {
    // A function that answers a record is reached from every call site, and a
    // jump from a call lands on the function rather than on the binding the
    // call fills.
    let study = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/module-functor-study.musa"),
    )
    .expect("the functor study");
    let mut server = Server::start();
    let (uri, published) = server.open("module-functor-study", &study);
    assert!(published.diagnostics.is_empty(), "{published:?}");

    let definition = server
        .client
        .request::<GotoDefinition>(lsp_types::GotoDefinitionParams {
            text_document_position_params: position_params(&uri, at(&study, "canon(c_major")),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });
    let definition: GotoDefinitionResponse = serde_json::from_value(definition).expect("a definition");
    let GotoDefinitionResponse::Scalar(location) = definition else {
        panic!("expected one location: {definition:?}");
    };
    assert_eq!(location.range.start.line, at(&study, "fn canon(context").line);

    // Both bindings call the one function.
    let references = server
        .client
        .request::<References>(reference_params(&uri, at(&study, "canon(c_major"), true));
    let references: Vec<Location> = serde_json::from_value(references).expect("references");
    assert_eq!(references.len(), 3, "declaration and two calls: {references:?}");

    let symbols: Vec<String> = flat_symbols(&mut server, &uri)
        .into_iter()
        .map(|symbol| symbol.name)
        .collect();
    assert!(symbols.iter().any(|name| name.starts_with("fn canon(")), "{symbols:?}");
    assert!(
        symbols.iter().any(|name| name.starts_with("record CanonMaterial")),
        "{symbols:?}"
    );
    server.stop();
}

#[test]
fn a_missing_register_offers_no_quick_fix_because_the_octave_is_a_choice() {
    // The rule the prompt fixes: a code action appears only where the
    // diagnostic carries a certain fix. A stacked chord with no register is
    // an error the compiler will not guess at — which octave the composer
    // meant is composition — so it arrives with no fix and the editor offers
    // nothing rather than something plausible.
    let source = "piece \"register\" {
    meter 4/4;
    key c major;
    score { part piano { voice melody { stack c major /1 } } }
}
";
    let mut server = Server::start();
    let (uri, published) = server.open("register", source);
    let diagnostic = published.diagnostics.first().expect("a diagnostic");
    assert!(diagnostic.message.contains("register"), "{}", diagnostic.message);
    let actions = server.client.request::<CodeActionRequest>(CodeActionParams {
        text_document: TextDocumentIdentifier { uri },
        range: diagnostic.range,
        context: lsp_types::CodeActionContext {
            diagnostics: vec![diagnostic.clone()],
            only: Some(vec![CodeActionKind::QUICKFIX]),
            trigger_kind: None,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    });
    let actions: Option<CodeActionResponse> = serde_json::from_value(actions).expect("code actions");
    assert!(
        actions.is_none_or(|actions| actions.is_empty()),
        "an uncertain fix must not be offered"
    );
    server.stop();
}

#[test]
fn analysis_answers_a_command_and_publishes_no_diagnostics() {
    let mut server = Server::start();
    let (uri, published) = server.open("glass-mountain", GLASS_MOUNTAIN);
    assert!(published.diagnostics.is_empty(), "{published:?}");
    let lenses = server.client.request::<CodeLensRequest>(CodeLensParams {
        text_document: TextDocumentIdentifier { uri },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    });
    let lenses: Vec<CodeLens> = serde_json::from_value(lenses).expect("code lenses");
    assert!(!lenses.is_empty(), "a compiled piece can be analyzed");
    let lens = lenses
        .iter()
        .find(|lens| {
            lens.command
                .as_ref()
                .is_some_and(|command| command.title == "Analyze: chords")
        })
        .expect("a lens for the chord reading");
    let command = lens.command.as_ref().expect("a command");
    assert_eq!(command.command, "musa.analyze");
    let arguments = command.arguments.clone().expect("the lens's own arguments");

    let answer = server.client.request::<ExecuteCommand>(ExecuteCommandParams {
        command: command.command.clone(),
        arguments,
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    assert_eq!(answer.get("kind").and_then(serde_json::Value::as_str), Some("chords"));
    assert!(answer.get("findings").is_some(), "typed findings, not prose: {answer}");
    // The assumptions travel with the findings: an analysis that stated none
    // would be claiming rather than observing.
    assert!(
        answer
            .get("assumptions")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|assumptions| !assumptions.is_empty()),
        "{answer}"
    );

    // The whole point: an observation is not a mistake, so nothing about it
    // reaches the problems pane.
    assert!(
        !server
            .client
            .notifications
            .iter()
            .any(|notification| notification.method == "textDocument/publishDiagnostics"),
        "analysis must not publish diagnostics"
    );
    server.stop();
}

#[test]
fn a_bundled_module_can_be_read_but_not_written() {
    // The other half of the read-only identity: the definition link points at
    // a document that exists nowhere on disk, so the client has to be able to
    // ask for its text — otherwise the link the server produced goes nowhere.
    let mut server = Server::start();
    let (_, _) = server.open("stdlib", STDLIB_PIECE);
    let answer = server.client.request::<ExecuteCommand>(ExecuteCommandParams {
        command: "musa.bundledSource".to_owned(),
        arguments: vec![serde_json::Value::String("musa-stdlib:/std/core.musa".to_owned())],
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let source = answer.as_str().expect("the module's own text");
    assert!(source.contains("identity_nat"), "{source}");

    let response = server.client.response::<ExecuteCommand>(ExecuteCommandParams {
        command: "musa.bundledSource".to_owned(),
        arguments: vec![serde_json::Value::String("musa-stdlib:/std/invented.musa".to_owned())],
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let error = response.response_result.expect_err("no such bundled module");
    assert!(error.message.contains("not a bundled Musa module"), "{}", error.message);
    server.stop();
}

#[test]
fn an_adapter_command_answers_with_a_workspace_edit_or_the_adapters_own_refusal() {
    // The route that makes `26-language-design-decision.md` §4's edit operation
    // reach a musician: the client sends the command it means, and the server
    // answers with the adapter's edit against this document. It applies
    // nothing itself, so a client that declines has changed nothing.
    let piece = "piece \"Doubled\" {\n    import syntax std::adapters::doubled as doubled;\n\n    let pair = syntax doubled { c4 };\n\n    score { part piano { voice one { c4/1 } } }\n}\n";
    let mut server = Server::start();
    let (uri, _) = server.open("doubled", piece);
    let at = piece.find("syntax doubled").expect("the piece writes a region");

    let answer = server.client.request::<ExecuteCommand>(ExecuteCommandParams {
        command: "musa.adapterEdit".to_owned(),
        arguments: vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::from(at),
            serde_json::Value::String("replace".to_owned()),
            serde_json::Value::from(2u64),
            serde_json::Value::String("d4".to_owned()),
        ],
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let edit: lsp_types::WorkspaceEdit = serde_json::from_value(answer).expect("a workspace edit");
    let changes = edit.changes.expect("edits for this document");
    let [edit] = changes.get(&uri).expect("this document's edits").as_slice() else {
        panic!("one command, one edit: {changes:?}");
    };
    assert_eq!(edit.new_text, "d4");

    // A command the adapter does not know is the adapter's sentence, and it
    // arrives as a refusal rather than as an empty edit — an editor cannot tell
    // "nothing to do" from "I would not do that" any other way.
    let response = server.client.response::<ExecuteCommand>(ExecuteCommandParams {
        command: "musa.adapterEdit".to_owned(),
        arguments: vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::from(at),
            serde_json::Value::String("transpose".to_owned()),
            serde_json::Value::from(2u64),
            serde_json::Value::String("up".to_owned()),
        ],
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let error = response.response_result.expect_err("the adapter refused");
    assert!(error.message.contains("one command"), "{}", error.message);
    server.stop();
}

#[test]
fn an_unknown_analysis_is_refused_rather_than_reported_as_clean() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let response = server.client.response::<ExecuteCommand>(ExecuteCommandParams {
        command: "musa.analyze".to_owned(),
        arguments: vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::String("counterpoint-in-reverse".to_owned()),
        ],
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let error = response.response_result.expect_err("an unknown analysis is refused");
    assert!(error.message.contains("not an analysis"), "{}", error.message);
    server.stop();
}

/// The markdown of a hover, or a panic naming the position that had none.
fn hover_markdown(server: &mut Server, uri: &Uri, position: Position) -> String {
    let hover = server.client.request::<HoverRequest>(HoverParams {
        text_document_position_params: position_params(uri, position),
        work_done_progress_params: WorkDoneProgressParams::default(),
    });
    let hover: Hover = serde_json::from_value(hover).unwrap_or_else(|_| panic!("no hover at {position:?}"));
    let HoverContents::Markup(content) = hover.contents else {
        panic!("expected markdown hover");
    };
    content.value
}

/// The signature help at one position, or a panic.
fn signature_help(server: &mut Server, uri: &Uri, position: Position) -> SignatureHelp {
    let answer = server
        .client
        .request::<SignatureHelpRequest>(lsp_types::SignatureHelpParams {
            text_document_position_params: position_params(uri, position),
            work_done_progress_params: WorkDoneProgressParams::default(),
            context: None,
        });
    serde_json::from_value(answer).unwrap_or_else(|_| panic!("no signature help at {position:?}"))
}

/// The completion menu at one position, as a list.
fn completions(server: &mut Server, uri: &Uri, position: Position) -> Vec<lsp_types::CompletionItem> {
    let answer = server.client.request::<Completion>(CompletionParams {
        text_document_position: position_params(uri, position),
        work_done_progress_params: WorkDoneProgressParams::default(),
        context: None,
        partial_result_params: PartialResultParams::default(),
    });
    let completions: CompletionResponse = serde_json::from_value(answer).expect("completions");
    let CompletionResponse::Array(items) = completions else {
        panic!("expected a completion list");
    };
    items
}

/// The document's symbols, flat.
fn flat_symbols(server: &mut Server, uri: &Uri) -> Vec<lsp_types::SymbolInformation> {
    let symbols = server.client.request::<DocumentSymbolRequest>(DocumentSymbolParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    });
    let symbols: DocumentSymbolResponse = serde_json::from_value(symbols).expect("symbols");
    let DocumentSymbolResponse::Flat(symbols) = symbols else {
        panic!("expected flat symbols");
    };
    symbols
}

#[test]
fn opening_a_piece_beside_its_libraries_resolves_the_imports() {
    // A directory project from the corpus, opened the way an editor opens
    // it: the URI names the file, and `use "../library/…"` must resolve
    // against the file's directory. The bug this pins named the document
    // `file:///…` and resolved the imports into a directory that does not
    // exist, so the editor reported what the command line never did.
    let piece = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/album/pieces/01-opening.musa");
    let text = std::fs::read_to_string(&piece).expect("the corpus piece");
    let mut server = Server::start();
    let uri = Uri::from_str(&format!("file://{}", piece.display())).expect("uri");
    server.client.notify::<DidOpenTextDocument>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri,
            language_id: "musa".to_owned(),
            version: 1,
            text,
        },
    });
    let published = server.client.notification::<PublishDiagnostics>();
    assert!(
        published.diagnostics.is_empty(),
        "the album piece imports fine from the command line; unexpected: {:?}",
        published.diagnostics
    );
    server.stop();
}
