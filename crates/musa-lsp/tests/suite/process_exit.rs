//! The one law `Connection::memory()` cannot state: the *process* ends.
//!
//! The library's own tests drive `run` over an in-memory pair, where there
//! are no IO threads to strand. Over stdio there are, and joining them
//! while the connection — and its senders — are still alive deadlocked the
//! shutdown the protocol had just agreed to. This test launches the binary
//! itself and requires it to exit cleanly after a proper shutdown.

// A subprocess test waits and kills on statically-valid input: a failure is
// a bug in the server, and panicking is the correct behavior there.
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use std::io::Write as _;
use std::process::{Command, Stdio};

/// One JSON-RPC frame, as the protocol spells it.
fn frame(message: &serde_json::Value) -> Vec<u8> {
    let body = serde_json::to_vec(message).expect("serialize");
    let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    out.extend_from_slice(&body);
    out
}

#[test]
fn the_process_exits_after_a_clean_shutdown() {
    let mut server = Command::new(env!("CARGO_BIN_EXE_musa-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn musa-lsp");
    let mut stdin = server.stdin.take().expect("stdin pipe");
    for message in [
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}}),
        serde_json::json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"}),
        serde_json::json!({"jsonrpc": "2.0", "method": "exit"}),
    ] {
        stdin.write_all(&frame(&message)).expect("write frame");
    }
    // The transport's half of `exit`: the client's end of the pipe closes.
    drop(stdin);

    // A healthy server exits at once; ten seconds is an eternity, not a
    // margin, and a timeout kills the child rather than hanging CI.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match server.try_wait().expect("wait on musa-lsp") {
            Some(status) => {
                assert!(status.success(), "musa-lsp exited with {status}");
                return;
            }
            None if std::time::Instant::now() < deadline => std::thread::sleep(std::time::Duration::from_millis(20)),
            None => {
                server.kill().expect("kill musa-lsp");
                panic!("musa-lsp did not exit after shutdown + exit + closed stdin");
            }
        }
    }
}
