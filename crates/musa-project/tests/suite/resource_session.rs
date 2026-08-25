use musa_project::{ProjectCommand, ProjectSession, Validity};

const GOOD: &str = "piece \"Budget\" { score { part p { voice v { c4/1 } } } }";

/// A piece over the construction budget, refused rather than aborted.
///
/// Written as a long plain voice rather than the `range(100001)` this fixture
/// used before the cutover, because deep *recursion* past the nesting limit is
/// the case prompt 165's own measurement says still aborts rather than refuses
/// — "756 levels are refused and 1,256 abort" — and a test that aborts takes
/// the whole suite with it. A voice's fold is δ rules all the way down, so
/// eight thousand notes charge work and construction but no depth, and the
/// refusal is the one §4 promises.
fn too_large() -> String {
    format!(
        "piece \"Budget\" {{ score {{ part p {{ voice v {{ {} }} }} }} }}",
        "c4/1 ".repeat(8000)
    )
}

#[test]
fn resource_rejection_keeps_the_last_valid_artifacts() {
    let mut session = ProjectSession::from_text(GOOD, "budget.musa");
    let good_revision = session.snapshot().revision();
    let good_mei = session.snapshot().mei().unwrap_or_default().to_owned();
    assert!(!good_mei.is_empty());

    let update = session.apply(ProjectCommand::SetSource(too_large()));
    assert!(update.is_ok(), "editing source is not an exceptional operation");
    assert!(matches!(update, Ok(found) if found.validity == Validity::Stale));
    let snapshot = session.snapshot();
    assert!(!snapshot.compiles());
    assert!(
        snapshot
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == "resource-limit")
    );
    assert_eq!(snapshot.mei(), Some(good_mei.as_str()));
    assert_eq!(snapshot.score_revision(), Some(good_revision));
}
