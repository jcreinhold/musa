//! The `musa-lsp` executable: a language server for `.musa` over stdio.
//!
//! An editor launches this binary as the server command and speaks JSON-RPC
//! on its standard streams; all of the protocol lives in the library, so the
//! binary is exactly one call.

fn main() -> std::process::ExitCode {
    musa_lsp::serve()
}
