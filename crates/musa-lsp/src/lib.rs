//! A language server for `.musa` (design roadmap §15.11).
//!
//! Owns: the LSP protocol over stdio, the document lifecycle (one
//! [`ProjectSession`](musa_project::ProjectSession) per open URI), and the
//! coordinate translation between the session's byte spans and LSP's UTF-16
//! positions — and nothing else. It is a thin shell beside `musa-cli` and the
//! desktop: every diagnostic, hover, definition, symbol, and fix is a
//! restatement of the session's facts, and every rule of
//! `docs/interface/03-interaction.md` §7 — the interface computes no musical
//! facts — holds here as it does there.
//!
//! Must never expose: parser, compiler, renderer, DSP, or CPAL types;
//! `lsp-server` protocol plumbing beyond [`run`]'s connection (the crate *is*
//! a protocol adapter — the connection is its interface, not an
//! implementation detail); a second opinion about the source. The server
//! never opens the audio device, never writes to disk, and never edits text
//! itself: every change it proposes travels as a workspace edit for the
//! client to apply and return as an ordinary `didChange`.
//!
//! Invariants: the server is single-threaded — one main loop owns the
//! sessions, matching "the session is single-threaded by construction";
//! [`convert`] is the only module that knows LSP's coordinate system;
//! semantic tokens and completion are answered from the lexer rather than
//! the facts, because they must work on half-typed source that has no valid
//! compile — the one reason this shell, alone among the shells, also depends
//! on `musa-language`.
//!
//! ```no_run
//! fn main() -> std::process::ExitCode {
//!     musa_lsp::serve()
//! }
//! ```

mod convert;
mod features;
mod workspace;

use std::process::ExitCode;

use lsp_server::{Connection, Message, ProtocolError, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Exit, Notification as _, PublishDiagnostics,
};
use lsp_types::request::{
    CodeActionRequest, Completion, DocumentSymbolRequest, Formatting, GotoDefinition, HoverRequest, Request as _,
    SemanticTokensFullRequest,
};
use lsp_types::{
    CodeActionProviderCapability, CompletionOptions, HoverProviderCapability, OneOf, PositionEncodingKind,
    PublishDiagnosticsParams, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};
use workspace::Workspace;

/// The JSON-RPC error codes this server can raise. `lsp-types` defines only
/// the LSP-specific ones; these two are the specification's.
const METHOD_NOT_FOUND: i32 = -32601;
/// The params would not parse — the client's protocol bug, not the music's.
const INVALID_PARAMS: i32 = -32602;

/// Serve the protocol over stdio: the whole public surface `main` needs.
///
/// Returns the process's exit code rather than exiting, so the decision to
/// end the process stays with the process.
pub fn serve() -> ExitCode {
    let (connection, io_threads) = Connection::stdio();
    let outcome = run(&connection);
    let joined = io_threads.join();
    match (outcome, joined) {
        (Ok(()), Ok(())) => ExitCode::SUCCESS,
        (outcome, joined) => {
            if let Err(error) = outcome {
                eprintln!("musa-lsp: {error}");
            }
            if let Err(error) = joined {
                eprintln!("musa-lsp: {error}");
            }
            ExitCode::FAILURE
        }
    }
}

/// Serve the protocol over `connection` until `shutdown` or `exit`.
///
/// Public — rather than reached only through [`serve`] — so the whole server
/// can be driven in tests over [`Connection::memory`], end to end, without a
/// subprocess. There is no other caller; there is also no smaller honest way
/// to test a protocol adapter than through the protocol.
///
/// # Errors
/// [`ServerError::Handshake`] if the initialize exchange fails, and
/// [`ServerError::Channel`] if the message channel breaks before `exit`.
pub fn run(connection: &Connection) -> Result<(), ServerError> {
    let capabilities =
        serde_json::to_value(server_capabilities()).map_err(|error| ServerError::Capabilities(error.to_string()))?;
    connection.initialize(capabilities)?;
    main_loop(connection)
}

/// The ways serving can end badly. The protocol's own errors travel as
/// protocol responses; these are the ones that end the loop.
#[derive(Debug)]
pub enum ServerError {
    /// The initialize handshake failed.
    Handshake(ProtocolError),
    /// The capabilities could not be serialized — a bug at build time, not a
    /// runtime condition.
    Capabilities(String),
    /// The message channel broke before the client said `exit`.
    Channel,
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Handshake(error) => write!(out, "initialize handshake failed: {error}"),
            Self::Capabilities(error) => write!(out, "capabilities failed to serialize: {error}"),
            Self::Channel => write!(out, "the message channel broke before `exit`"),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Handshake(error) => Some(error),
            Self::Capabilities(_) | Self::Channel => None,
        }
    }
}

impl From<ProtocolError> for ServerError {
    fn from(error: ProtocolError) -> Self {
        Self::Handshake(error)
    }
}

/// What the server answers, declared once at handshake.
fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF16),
        // Pieces are small and the desktop's editor syncs the same way; a
        // partial-sync code path would be a second document model to keep
        // true, bought with nothing.
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: None,
            ..CompletionOptions::default()
        }),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: features::semantic_tokens::legend(),
                    token_modifiers: Vec::new(),
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: Some(false),
                ..SemanticTokensOptions::default()
            },
        )),
        ..ServerCapabilities::default()
    }
}

/// Whether the loop keeps serving.
enum Flow {
    Continue,
    Exit,
}

/// One loop owns the sessions; there is no interleaving to reason about.
fn main_loop(connection: &Connection) -> Result<(), ServerError> {
    let mut workspace = Workspace::new();
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(());
                }
                let response = dispatch(&workspace, request);
                send(connection, Message::Response(response))?;
            }
            Message::Notification(notification) => {
                if matches!(on_notification(connection, &mut workspace, notification)?, Flow::Exit) {
                    return Ok(());
                }
            }
            // The server asks the client nothing, so a response is never for
            // us; dropping it is correct.
            Message::Response(_) => {}
        }
    }
    // The channel closed with the client gone; that is an ending, not an
    // error.
    Ok(())
}

/// Send one message, or report the channel broken.
fn send(connection: &Connection, message: Message) -> Result<(), ServerError> {
    connection.sender.send(message).map_err(|_| ServerError::Channel)
}

/// Answer a request. Every arm is the same shape: find the document, ask the
/// feature module, translate the answer — `None` is a lawful answer to most.
fn dispatch(workspace: &Workspace, request: Request) -> Response {
    let id = request.id.clone();
    let method = request.method.clone();
    match method.as_str() {
        HoverRequest::METHOD => answer::<HoverRequest>(request, |params| {
            let at = params.text_document_position_params;
            workspace
                .document(&at.text_document.uri)
                .and_then(|document| features::hover::hover(document, at.position))
        }),
        GotoDefinition::METHOD => answer::<GotoDefinition>(request, |params| {
            let at = params.text_document_position_params;
            workspace
                .document(&at.text_document.uri)
                .and_then(|document| features::definition::definition(document, &at.text_document.uri, at.position))
        }),
        DocumentSymbolRequest::METHOD => answer::<DocumentSymbolRequest>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .and_then(|document| features::symbols::symbols(document, &params.text_document.uri))
        }),
        CodeActionRequest::METHOD => answer::<CodeActionRequest>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .and_then(|document| features::code_action::code_actions(document, &params.text_document.uri, &params))
        }),
        Formatting::METHOD => answer::<Formatting>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .and_then(features::formatting::format)
        }),
        SemanticTokensFullRequest::METHOD => answer::<SemanticTokensFullRequest>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .map(features::semantic_tokens::full)
        }),
        Completion::METHOD => answer::<Completion>(request, |params| {
            workspace
                .document(&params.text_document_position.text_document.uri)
                .map(features::completion::completions)
        }),
        _ => Response::new_err(id, METHOD_NOT_FOUND, format!("musa-lsp does not answer `{method}`")),
    }
}

/// Extract the params, compute the answer, serialize it. A method that
/// matched but would not parse is the client's protocol bug, answered as
/// such.
fn answer<R>(request: Request, compute: impl FnOnce(R::Params) -> R::Result) -> Response
where
    R: lsp_types::request::Request,
{
    let id = request.id.clone();
    match request.extract::<R::Params>(R::METHOD) {
        Ok((id, params)) => Response::new_ok(id, compute(params)),
        Err(_) => Response::new_err(id, INVALID_PARAMS, format!("malformed `{}` parameters", R::METHOD)),
    }
}

/// The document lifecycle, and the diagnostics every change publishes.
fn on_notification(
    connection: &Connection,
    workspace: &mut Workspace,
    notification: lsp_server::Notification,
) -> Result<Flow, ServerError> {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let Some(params) = params::<DidOpenTextDocument>(notification) else {
                return Ok(Flow::Continue);
            };
            let document = params.text_document;
            let uri = document.uri;
            workspace.open(&uri, document.text, document.version);
            publish(connection, workspace, &uri)?;
        }
        DidChangeTextDocument::METHOD => {
            let Some(params) = params::<DidChangeTextDocument>(notification) else {
                return Ok(Flow::Continue);
            };
            let uri = params.text_document.uri;
            // Full sync: the last change carries the whole text.
            if let Some(change) = params.content_changes.into_iter().last() {
                workspace.change(&uri, change.text, params.text_document.version);
                publish(connection, workspace, &uri)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let Some(params) = params::<DidCloseTextDocument>(notification) else {
                return Ok(Flow::Continue);
            };
            let uri = params.text_document.uri;
            workspace.close(&uri);
            // Closing clears the file from the client's problems pane.
            send(
                connection,
                Message::Notification(lsp_server::Notification::new(
                    PublishDiagnostics::METHOD.to_owned(),
                    PublishDiagnosticsParams {
                        uri,
                        diagnostics: Vec::new(),
                        version: None,
                    },
                )),
            )?;
        }
        Exit::METHOD => return Ok(Flow::Exit),
        _ => {}
    }
    Ok(Flow::Continue)
}

/// Parse a notification's params, or ignore the notification — a malformed
/// notification has no return address, so silence is the only honest answer.
fn params<N>(notification: lsp_server::Notification) -> Option<N::Params>
where
    N: lsp_types::notification::Notification,
{
    notification.extract::<N::Params>(N::METHOD).ok()
}

/// Publish the document's current diagnostics after every change.
fn publish(connection: &Connection, workspace: &Workspace, uri: &Uri) -> Result<(), ServerError> {
    let Some(document) = workspace.document(uri) else {
        return Ok(());
    };
    send(
        connection,
        Message::Notification(lsp_server::Notification::new(
            PublishDiagnostics::METHOD.to_owned(),
            PublishDiagnosticsParams {
                uri: uri.clone(),
                diagnostics: features::diagnostics::all(document, uri),
                version: Some(document.version()),
            },
        )),
    )
}
