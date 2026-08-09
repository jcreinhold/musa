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
    CodeActionRequest, Completion, DocumentSymbolRequest, FoldingRangeRequest, Formatting, GotoDefinition,
    HoverRequest, Initialize, PrepareRenameRequest, References, Rename, SemanticTokensFullRequest, Shutdown,
};
use lsp_types::{
    CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse, CompletionParams, CompletionResponse,
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse, FoldingRange,
    FoldingRangeKind, FoldingRangeParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams, InitializedParams,
    Location, PartialResultParams, Position, PrepareRenameResponse, PublishDiagnosticsParams, ReferenceContext,
    ReferenceParams, RenameParams, SemanticTokensParams, SemanticTokensResult, SymbolKind,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams, Uri,
    VersionedTextDocumentIdentifier, WorkDoneProgressParams,
};

const GLASS_MOUNTAIN: &str = include_str!("../../../examples/glass-mountain.musa");
const ANNOTATED: &str = include_str!("../../../examples/annotated.musa");
const MISSING_SEMICOLON: &str = include_str!("../../../examples/broken/missing-semicolon.musa");

/// A small valid piece whose every position the tests can count by hand —
/// one full bar of 4/4, because a bar is written down and checked (prompt 57).
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
        at(GLASS_MOUNTAIN, "sigh();"),
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
            at(GLASS_MOUNTAIN, "sigh();").line,
            at_last(GLASS_MOUNTAIN, "sigh();").line,
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
        .request::<References>(reference_params(&uri, at(GLASS_MOUNTAIN, "sigh();"), false));
    let locations: Vec<Location> = serde_json::from_value(answer).expect("locations");
    assert_eq!(locations.len(), 2, "the two uses only: {locations:?}");
    server.stop();
}

#[test]
fn references_on_a_patch_find_the_declaration_and_the_assigns() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let answer = server.client.request::<References>(reference_params(
        &uri,
        shifted(at(GLASS_MOUNTAIN, "patch glass_pad"), 6),
        true,
    ));
    let locations: Vec<Location> = serde_json::from_value(answer).expect("locations");
    let mut lines: Vec<u32> = locations.iter().map(|location| location.range.start.line).collect();
    lines.sort_unstable();
    // Both parts are assigned to the pad: one declaration, two uses. The
    // `modulate lfo -> glass_pad.lowpass.cutoff` property path is not one.
    let mut expected: Vec<u32> = [
        at(GLASS_MOUNTAIN, "patch glass_pad").line,
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
        .request::<PrepareRenameRequest>(position_params(&uri, at(GLASS_MOUNTAIN, "sigh();")));
    let answer: Option<PrepareRenameResponse> = serde_json::from_value(answer).expect("prepare rename");
    let Some(PrepareRenameResponse::RangeWithPlaceholder { range, placeholder }) = answer else {
        panic!("expected a range with the name: {answer:?}");
    };
    assert_eq!(placeholder, "sigh");
    assert_eq!(range.start, at(GLASS_MOUNTAIN, "sigh();"));
    assert_eq!(range.end, shifted(at(GLASS_MOUNTAIN, "sigh();"), 4));
    // On a keyword there is nothing to prepare: null, the lawful answer.
    let answer = server
        .client
        .request::<PrepareRenameRequest>(position_params(&uri, at(GLASS_MOUNTAIN, "tempo")));
    assert!(answer.is_null(), "expected null on `tempo`: {answer}");
    server.stop();
}

#[test]
fn rename_rewrites_exactly_the_recorded_spans() {
    let mut server = Server::start();
    let (uri, _) = server.open("glass-mountain", GLASS_MOUNTAIN);
    let answer = server
        .client
        .request::<Rename>(rename_params(&uri, at(GLASS_MOUNTAIN, "sigh();"), "lament"));
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
        .response::<Rename>(rename_params(&uri, at(GLASS_MOUNTAIN, "sigh();"), "4x"));
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
        .request::<References>(reference_params(&uri, at(GLASS_MOUNTAIN, "sigh();"), true));
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
    assert!(content.value.contains("sigh()"), "{}", content.value);
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
        "piece \"x\" {\n    fn choose(x: option[nat]) -> nat = match x { none ->\n    @@ mid-edit \"unterminated\n}\n";
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
        "fn, option, match, and none stay keywords even in incomplete source"
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
    // A keyword teaches from the menu: its own documentation rides the item
    // (prompt 84), while a unit has only its class.
    let tempo = items.iter().find(|item| item.label == "tempo").expect("tempo item");
    assert_eq!(tempo.detail.as_deref(), Some("how fast, written where it changes"));
    let Some(lsp_types::Documentation::MarkupContent(content)) = &tempo.documentation else {
        panic!("tempo should carry markdown documentation");
    };
    assert!(content.value.contains("```musa"), "{}", content.value);
    let hz = items.iter().find(|item| item.label == "Hz").expect("Hz item");
    assert_eq!(hz.detail.as_deref(), Some("unit"));
    assert!(hz.documentation.is_none());
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
    // Every brace pair in the fixture spans lines: thirteen region folds and
    // two comment runs, no more.
    let regions = ranges
        .iter()
        .filter(|range| range.kind == Some(FoldingRangeKind::Region))
        .count();
    assert_eq!(regions, 13, "{starts:?}");
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
    let unclosed = include_str!("../../../examples/broken/unclosed-block.musa");
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
/// like any other, and its certain fix is a quick fix like any other
/// (prompt 83 — the server carries it with no new plumbing, which is the
/// point this test pins).
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
