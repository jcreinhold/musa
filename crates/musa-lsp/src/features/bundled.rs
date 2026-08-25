//! The standard library's own text, for clients that must show it.
//!
//! A bundled module is compiled into the binary. Go-to-definition on
//! `identity_nat` lands in `musa-stdlib:/std/core.musa`, and there is no such
//! file on disk for an editor to open — so the client asks the server for the
//! text and shows it in a document of its own making, read-only, because an
//! edit would have nowhere to land.
//!
//! It is a command rather than a new request method on purpose. The protocol
//! already carries `workspace/executeCommand`, both clients already speak it
//! for analysis, and a bespoke method would be one more thing every future
//! client has to implement before it can follow a link the server itself
//! produced.
//!
//! The server does not render the text, decide the document's name, or say
//! how to display it: those are the client's, and a client that wants a
//! different presentation needs nothing from here to build it.

use serde_json::Value;

/// The command a client sends to read a bundled module.
pub(crate) const BUNDLED_SOURCE: &str = "musa.bundledSource";

/// The Musa source of the bundled module `uri` names.
///
/// # Errors
/// A URI no bundled module answers to. Refused rather than answered with an
/// empty document: a reader shown a blank standard library would conclude the
/// module is empty.
pub(crate) fn execute(uri: &str, project_source: Option<&str>) -> Result<Value, String> {
    let source = project_source
        .or_else(|| musa_project::standard_library_source(uri))
        .ok_or_else(|| {
            if uri.starts_with("musa-stdlib:") {
                format!("`{uri}` is not a bundled Musa module")
            } else {
                format!("`{uri}` is not a readable immutable Musa module")
            }
        })?;
    Ok(Value::String(source.to_owned()))
}
