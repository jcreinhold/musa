use musa_project::{ProjectCommand, ProjectSession, Validity};

const GOOD: &str = "piece \"Budget\" { let values: List<Nat> = range(8); score { part p { voice v { c4/1 } } } }";
const TOO_LARGE: &str =
    "piece \"Budget\" { let values: List<Nat> = range(100001); score { part p { voice v { c4/1 } } } }";

#[test]
fn resource_rejection_keeps_the_last_valid_artifacts() {
    let mut session = ProjectSession::from_text(GOOD, "budget.musa");
    let good_revision = session.snapshot().revision();
    let good_mei = session.snapshot().mei().unwrap_or_default().to_owned();
    assert!(!good_mei.is_empty());

    let update = session.apply(ProjectCommand::SetSource(TOO_LARGE.to_owned()));
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
