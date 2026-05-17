#[kani::proof]
fn inventory_exactly_matches_canonical_screens() {
    assert!(crate::REQUIRED_FIXTURES.len() == 8);
    assert!(crate::REQUIRED_FIXTURES.iter().any(|screen| *screen == "execution_overview"));
    assert!(crate::REQUIRED_FIXTURES.iter().any(|screen| *screen == "workflow_graph_authoring"));
    assert!(crate::REQUIRED_FIXTURES.iter().any(|screen| *screen == "execution_details"));
    assert!(crate::REQUIRED_FIXTURES.iter().any(|screen| *screen == "verification_certificate"));
    assert!(crate::REQUIRED_FIXTURES.iter().any(|screen| *screen == "replay_theater"));
    assert!(crate::REQUIRED_FIXTURES.iter().any(|screen| *screen == "incident_failure"));
    assert!(crate::REQUIRED_FIXTURES.iter().any(|screen| *screen == "action_registry"));
    assert!(crate::REQUIRED_FIXTURES.iter().any(|screen| *screen == "storage_doctor_ai_context"));
}
