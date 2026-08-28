//! The committed header and the crate agree, byte for byte.

use std::path::PathBuf;

fn committed() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("include/musa_au.h")
}

/// The generated header is current.
///
/// The header is the only place the ABI is written twice, so this is the test
/// that keeps the second copy honest. It is a currency test in the shape the
/// UI fixtures already use: regenerate, compare, and say how to fix it.
#[test]
fn the_committed_header_is_current() {
    let generated = musa_au::header();
    let path = committed();
    if std::env::var_os("UPDATE_AU_HEADER").is_some() {
        std::fs::write(&path, &generated).expect("the header directory exists");
        return;
    }
    let found = std::fs::read_to_string(&path).expect("include/musa_au.h exists");
    assert_eq!(
        found, generated,
        "crates/musa-au/include/musa_au.h is stale — rerun with UPDATE_AU_HEADER=1 and commit the result"
    );
}

/// The header states the same event layout the Rust side has.
///
/// A static assertion the C compiler will check is only as good as the number
/// written into it, so the number is checked here too.
#[test]
fn the_header_states_this_crates_event_layout() {
    let generated = musa_au::header();
    let size = size_of::<musa_au::MusaAuEvent>();
    assert!(
        generated.contains(&format!("sizeof(MusaAuEvent) == {size},")),
        "the header does not assert this crate's MusaAuEvent size"
    );
    assert!(generated.contains(&format!(
        "#define MUSA_AU_ABI_VERSION {}u",
        musa_au::MUSA_AU_ABI_VERSION
    )));
}
