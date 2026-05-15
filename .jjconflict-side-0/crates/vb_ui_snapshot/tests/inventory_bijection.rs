use proptest::prelude::*;
use vb_ui_snapshot::{REQUIRED_FIXTURES, demo_fixture_names, fixtures::load_demo_fixture};

const CANONICAL: [&str; 8] = [
    "execution_overview",
    "workflow_graph_authoring",
    "execution_details",
    "verification_certificate",
    "replay_theater",
    "incident_failure",
    "action_registry",
    "storage_doctor_ai_context",
];

#[test]
fn inventory_returns_canonical_eight_when_all_required_screens_present() {
    let names = demo_fixture_names();

    assert_eq!(names, CANONICAL);
    assert_eq!(REQUIRED_FIXTURES, CANONICAL);
}

#[test]
fn fixture_loader_returns_execution_overview_when_screen_id_is_canonical() {
    let result = load_demo_fixture("execution_overview")
        .map(|fixture| fixture.screen_kind)
        .map_err(|error| format!("{error:?}"));

    assert_eq!(result, Ok(String::from("ExecutionOverview")));
}

#[test]
fn fixture_loader_returns_storage_doctor_when_screen_id_is_canonical() {
    let result = load_demo_fixture("storage_doctor_ai_context")
        .map(|fixture| fixture.screen_kind)
        .map_err(|error| format!("{error:?}"));

    assert_eq!(result, Ok(String::from("StorageDoctorAiContext")));
}

#[test]
fn fixture_loader_rejects_unknown_screen_with_exact_fixture_error() {
    let result = load_demo_fixture("unknown_vb_nf2u_screen")
        .map(|fixture| fixture.name)
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "FixtureNotFound { fixture_id: \"unknown_vb_nf2u_screen\" }"
        ))
    );
}

#[test]
fn fixture_loader_rejects_partial_screen_id_with_exact_fixture_error() {
    let result = load_demo_fixture("execution")
        .map(|fixture| fixture.name)
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "FixtureNotFound { fixture_id: \"execution\" }"
        ))
    );
}

proptest! {
    #[test]
    fn inventory_validation_rejects_any_non_canonical_fixture_id(candidate in "[a-z_]{0,40}") {
        prop_assume!(!CANONICAL.contains(&candidate.as_str()));

        let result = load_demo_fixture(&candidate)
            .map(|fixture| fixture.name)
            .map_err(|error| format!("{error:?}"));

        prop_assert_eq!(
            result,
            Err(format!("FixtureNotFound {{ fixture_id: {candidate:?} }}"))
        );
    }
}
