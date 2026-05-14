use vb_ui_makepad::shell::{Screen, ShellNav};
use vb_ui_snapshot::REQUIRED_FIXTURES;

#[test]
fn shell_reachability_maps_every_nav_to_release_screen() {
    assert_eq!(ShellNav::Overview.screen(), Screen::ExecutionOverview);
    assert_eq!(
        ShellNav::WorkflowGraph.screen(),
        Screen::WorkflowGraphAuthoring
    );
    assert_eq!(ShellNav::Executions.screen(), Screen::ExecutionDetailsGraph);
    assert_eq!(
        ShellNav::Verification.screen(),
        Screen::VerificationCertificate
    );
    assert_eq!(ShellNav::Replay.screen(), Screen::ReplayTheater);
    assert_eq!(ShellNav::Incidents.screen(), Screen::IncidentFailureConsole);
    assert_eq!(ShellNav::Actions.screen(), Screen::ActionRegistry);
    assert_eq!(ShellNav::Storage.screen(), Screen::StorageDoctorAiContext);
}

#[test]
fn shell_reachability_is_bijective_across_nav_screen_and_fixture_ids() {
    let rows = [
        (
            ShellNav::Overview,
            Screen::ExecutionOverview,
            "execution_overview",
        ),
        (
            ShellNav::WorkflowGraph,
            Screen::WorkflowGraphAuthoring,
            "workflow_graph_authoring",
        ),
        (
            ShellNav::Executions,
            Screen::ExecutionDetailsGraph,
            "execution_details",
        ),
        (
            ShellNav::Verification,
            Screen::VerificationCertificate,
            "verification_certificate",
        ),
        (ShellNav::Replay, Screen::ReplayTheater, "replay_theater"),
        (
            ShellNav::Incidents,
            Screen::IncidentFailureConsole,
            "incident_failure",
        ),
        (ShellNav::Actions, Screen::ActionRegistry, "action_registry"),
        (
            ShellNav::Storage,
            Screen::StorageDoctorAiContext,
            "storage_doctor_ai_context",
        ),
    ];

    assert_eq!(rows.len(), 8);
    assert_eq!(rows.map(|row| row.0.screen()), rows.map(|row| row.1));
    assert_eq!(rows.map(|row| row.2), REQUIRED_FIXTURES);
}
