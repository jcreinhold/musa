//! A language server for `.musa` (design roadmap §15.11).
//!
//! Owns: the LSP protocol over stdio, the document lifecycle (one
//! [`ProjectSession`](musa_project::ProjectSession) per open URI), and the
//! coordinate translation between the session's byte spans and LSP's UTF-16
//! positions — and nothing else. It is a thin shell beside `musa` and the
//! desktop: every diagnostic, hover, definition, symbol, and fix is a
//! restatement of the session's facts, and every rule of
//! `docs/rules/desktop/03-interaction.md` §7 — the interface computes no musical
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
use std::str::FromStr as _;

use lsp_server::{Connection, Message, ProtocolError, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Exit, Notification as _, PublishDiagnostics,
};
use lsp_types::request::{
    CodeActionRequest, CodeLensRequest, Completion, DocumentSymbolRequest, ExecuteCommand, FoldingRangeRequest,
    Formatting, GotoDefinition, HoverRequest, PrepareRenameRequest, References, Rename, Request as _,
    SemanticTokensFullRequest, SignatureHelpRequest,
};
use lsp_types::{
    CodeActionProviderCapability, CodeLensOptions, CompletionOptions, ExecuteCommandOptions, HoverProviderCapability,
    OneOf, PositionEncodingKind, PublishDiagnosticsParams, RenameOptions, SemanticTokensFullOptions,
    SemanticTokensLegend, SemanticTokensOptions, SemanticTokensServerCapabilities, ServerCapabilities,
    SignatureHelpOptions, TextDocumentSyncCapability, TextDocumentSyncKind, Uri, WorkDoneProgressOptions,
};
use workspace::{Document, Workspace};

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
    // An editor launches this binary; nobody attaches a debugger to it, and
    // its stdout is the transport. A span per request on stderr is the only
    // account of what the server did, and `MUSA_LOG` is how a person asks for
    // one (roadmap §15.11).
    let _installed = musa_project::Logging::new().install();
    let (connection, io_threads) = Connection::stdio();
    let outcome = run(&connection);
    // Drop the connection — and with it every sender — before joining: the
    // writer thread ends when its channel closes, and the channel closes
    // only once this handle is gone. Joining first deadlocks the shutdown
    // the protocol just agreed to.
    drop(connection);
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
        folding_range_provider: Some(lsp_types::FoldingRangeProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            // A dot reaches into a module and an open paren begins an
            // argument, and both are positions where the menu knows something
            // the prefix does not.
            trigger_characters: Some(vec![".".to_owned(), "(".to_owned()]),
            ..CompletionOptions::default()
        }),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_owned()]),
            retrigger_characters: Some(vec![",".to_owned()]),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        // Analysis is asked for, never volunteered: the lens is the asking,
        // and the command is the answer (`features::analysis`).
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(false),
        }),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: commands(),
            work_done_progress_options: WorkDoneProgressOptions::default(),
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
    // The method and the request id, which is what a client's own log records
    // too — so a slow completion in an editor can be matched to the request
    // that served it. The document is not a field here: it lives inside the
    // params, differently shaped per method, and digging it out would be work
    // done for the log.
    let span = tracing::debug_span!("request", method = %method, id = %id);
    let _entered = span.enter();
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
                .and_then(surface_only)
                .and_then(|document| features::definition::definition(document, &at.text_document.uri, at.position))
        }),
        DocumentSymbolRequest::METHOD => answer::<DocumentSymbolRequest>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .and_then(|document| features::symbols::symbols(document, &params.text_document.uri))
        }),
        References::METHOD => answer::<References>(request, |params| {
            workspace
                .document(&params.text_document_position.text_document.uri)
                .and_then(surface_only)
                .and_then(|document| {
                    features::names::references(document, &params.text_document_position.text_document.uri, &params)
                })
        }),
        PrepareRenameRequest::METHOD => answer::<PrepareRenameRequest>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .and_then(surface_only)
                .and_then(|document| features::names::prepare_rename(document, params.position))
        }),
        Rename::METHOD => answer_fallible::<Rename>(request, |params| {
            workspace
                .document(&params.text_document_position.text_document.uri)
                .and_then(surface_only)
                .map_or_else(
                    || Err("this document has no names to rename".to_owned()),
                    |document| features::names::rename(document, &params),
                )
        }),
        CodeActionRequest::METHOD => answer::<CodeActionRequest>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .and_then(surface_only)
                .and_then(|document| features::code_action::code_actions(document, &params.text_document.uri, &params))
        }),
        Formatting::METHOD => answer::<Formatting>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .and_then(features::formatting::format)
        }),
        FoldingRangeRequest::METHOD => answer::<FoldingRangeRequest>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .and_then(surface_only)
                .and_then(features::folding::folding_ranges)
        }),
        SemanticTokensFullRequest::METHOD => answer::<SemanticTokensFullRequest>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .map(features::semantic_tokens::full)
        }),
        Completion::METHOD => answer::<Completion>(request, |params| {
            let at = params.text_document_position;
            workspace
                .document(&at.text_document.uri)
                .and_then(surface_only)
                .map(|document| features::completion::completions(document, at.position))
        }),
        SignatureHelpRequest::METHOD => answer::<SignatureHelpRequest>(request, |params| {
            let at = params.text_document_position_params;
            workspace
                .document(&at.text_document.uri)
                .and_then(surface_only)
                .and_then(|document| features::signature_help::signature_help(document, at.position))
        }),
        CodeLensRequest::METHOD => answer::<CodeLensRequest>(request, |params| {
            workspace
                .document(&params.text_document.uri)
                .and_then(surface_only)
                .and_then(|document| features::analysis::code_lenses(document, &params.text_document.uri))
        }),
        ExecuteCommand::METHOD => {
            answer_fallible::<ExecuteCommand>(request, |params| execute_command(workspace, &params).map(Some))
        }
        _ => Response::new_err(id, METHOD_NOT_FOUND, format!("musa-lsp does not answer `{method}`")),
    }
}

/// Every command this server executes, for the handshake.
///
/// Two, and both are questions a client cannot answer for itself: what an
/// analysis saw, and what a bundled module says. Neither is an edit — a
/// command that changed a document would be a command the client could not
/// undo.
fn commands() -> Vec<String> {
    vec![
        features::analysis::ANALYZE.to_owned(),
        features::bundled::BUNDLED_SOURCE.to_owned(),
    ]
}

/// Run one workspace command.
///
/// The arguments are the command's own, positionally, and a client sending a
/// command sends what the lens or the link that offered it carried. A missing
/// or malformed argument is the client's protocol bug and is answered as one,
/// not as an empty result.
fn execute_command(
    workspace: &Workspace,
    params: &lsp_types::ExecuteCommandParams,
) -> Result<serde_json::Value, String> {
    if params.command == features::bundled::BUNDLED_SOURCE {
        let [uri] = params.arguments.as_slice() else {
            return Err(format!("`{}` takes one bundled module URI", params.command));
        };
        let uri = uri.as_str().ok_or_else(|| "the argument is a URI string".to_owned())?;
        return features::bundled::execute(uri);
    }
    if params.command != features::analysis::ANALYZE {
        return Err(format!("musa-lsp does not run `{}`", params.command));
    }
    let [uri, kind] = params.arguments.as_slice() else {
        return Err(format!(
            "`{}` takes a document URI and an analysis kind",
            params.command
        ));
    };
    let (Some(uri), Some(kind)) = (uri.as_str(), kind.as_str()) else {
        return Err("both arguments are strings: a document URI and an analysis kind".to_owned());
    };
    let uri = Uri::from_str(uri).map_err(|_| format!("`{uri}` is not a URI"))?;
    let document = workspace
        .document(&uri)
        .ok_or_else(|| "that document is not open here".to_owned())?;
    features::analysis::execute(document, kind)
}

/// The document, if it is written in the surface language.
///
/// The gate on every feature that reads a surface syntax tree or resolves a
/// name a `.musa` file declares: completion, go-to-definition, references,
/// rename, code actions, and folding all answer from structure a kernel
/// document does not have. Answering them anyway would not be empty — it
/// would be *wrong*, because a kernel document's compiled facts carry the
/// source spans of the piece that produced them, and every one of those
/// offsets is a position in a different file.
///
/// The features that do answer for both alternatives — diagnostics,
/// formatting, semantic tokens, symbols, hover — dispatch inside themselves,
/// because for them the kernel case is a different answer rather than no
/// answer.
fn surface_only(document: &Document) -> Option<&Document> {
    (document.alternative() == musa_language::DocumentAlternative::Surface).then_some(document)
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

/// [`answer`], for the one method — rename — whose refusal is part of its
/// semantics: "that name is taken" is an error, not an empty edit.
fn answer_fallible<R>(request: Request, compute: impl FnOnce(R::Params) -> Result<R::Result, String>) -> Response
where
    R: lsp_types::request::Request,
{
    let id = request.id.clone();
    match request.extract::<R::Params>(R::METHOD) {
        Ok((id, params)) => match compute(params) {
            Ok(result) => Response::new_ok(id, result),
            Err(message) => Response::new_err(id, INVALID_PARAMS, message),
        },
        Err(_) => Response::new_err(id, INVALID_PARAMS, format!("malformed `{}` parameters", R::METHOD)),
    }
}

/// The document lifecycle, and the diagnostics every change publishes.
fn on_notification(
    connection: &Connection,
    workspace: &mut Workspace,
    notification: lsp_server::Notification,
) -> Result<Flow, ServerError> {
    let span = tracing::debug_span!("notification", method = %notification.method);
    let _entered = span.enter();
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
