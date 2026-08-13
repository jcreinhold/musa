//! Adapter commands, so a region can be edited from an editor.
//!
//! A syntax region is written in a package's language rather than in Musa, and
//! only that package knows what editing one means. What the server can do —
//! and what `26-language-design-decision.md` §4 says it must do — is carry the
//! command to the adapter and the adapter's answer back as ordinary text
//! edits, so a musician changes a region through the same door they change
//! everything else through.
//!
//! A command rather than a code action. A code action is offered from what the
//! server can see, and the server cannot see which of a package's commands
//! apply to the node under the cursor without knowing the package's own value
//! type — which is exactly what the phase boundary refuses. So the client
//! sends the command it means, with the anchor the value it is showing
//! carries, and the server answers with the edit or with the adapter's own
//! sentence about why there is none.

use lsp_types::{TextEdit, Uri, WorkspaceEdit};
use serde_json::Value;

use crate::workspace::Document;

/// The command a client sends to edit through an adapter.
pub(crate) const ADAPTER_EDIT: &str = "musa.adapterEdit";

/// Ask the adapter that reads the region at `offset` to serve one command.
///
/// The answer is a workspace edit against this document, which the client
/// applies the way it applies a rename's: the server changes nothing itself,
/// so a client that decides not to apply it has changed nothing either.
///
/// # Errors
/// The adapter's own refusal, a region that is read-only because its adapter
/// declares no `edit`, or an offset no region stands at. All three are
/// sentences a musician should read rather than empty results, because an
/// empty edit and a refused command look identical in an editor and mean
/// entirely different things.
pub(crate) fn execute(
    document: &Document,
    uri: &Uri,
    offset: u32,
    command: &str,
    anchor: u64,
    argument: &str,
) -> Result<Value, String> {
    let impact = document
        .session()
        .adapter_command(offset, command, anchor, argument)
        .map_err(|error| error.to_string())?;
    let edits: Vec<TextEdit> = impact
        .iter()
        .map(|edit| TextEdit {
            range: document.lines().range(musa_project::Span {
                start: edit.start,
                end: edit.end,
            }),
            new_text: edit.text.clone(),
        })
        .collect();
    let edit = WorkspaceEdit {
        changes: Some(std::iter::once((uri.clone(), edits)).collect()),
        ..WorkspaceEdit::default()
    };
    serde_json::to_value(edit).map_err(|error| error.to_string())
}
