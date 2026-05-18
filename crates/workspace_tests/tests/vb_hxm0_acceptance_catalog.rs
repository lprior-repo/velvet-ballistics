#![forbid(unsafe_code)]

use velvet_ballastics_workspace_tests::acceptance_catalog::{
    CatalogValidationError, Scenario, catalog, validate_catalog,
};

const REQUIRED_BEHAVIOR_ROWS: &[(&str, &str)] = &[
    (
        "canonical naming and workspace spelling gates",
        "VB-BDD-CATALOG-001",
    ),
    (
        "validation gates reject malformed workflow parts",
        "VB-BDD-CATALOG-002",
    ),
    (
        "YAML strict admission emits accepted artifacts before execution",
        "VB-BDD-CATALOG-003",
    ),
    (
        "runtime direct API exposes submit, inspect, cancel, trace, and shutdown",
        "VB-BDD-CATALOG-004",
    ),
    (
        "binary IPC rejects malformed frames before payload allocation",
        "VB-BDD-CATALOG-005",
    ),
    (
        "storage recovery preserves journal evidence and rejects corrupt records",
        "VB-BDD-CATALOG-006",
    ),
    (
        "generated Rust remains semantically equivalent to IR mode",
        "VB-BDD-CATALOG-007",
    ),
    (
        "capability admission fails closed for missing action grants",
        "VB-BDD-CATALOG-008",
    ),
    (
        "resource bounds and hot-path budgets fail before unbounded allocation",
        "VB-BDD-CATALOG-009",
    ),
    (
        "test evidence must be executable and assertion-strong",
        "VB-BDD-CATALOG-010",
    ),
];

fn valid_bad_scenario(id: &'static str) -> Scenario {
    Scenario {
        id,
        master_behavior: "bad catalog fixture",
        given: "a catalog row",
        when: "the catalog gate validates it",
        then: "the exact validation error is returned",
        public_surface: "workspace test boundary",
        fixture: "isolated bad catalog fixture",
        expected_outcome: Some("rejected"),
        expected_error: None,
        durability_profile: "none",
        related_bead: "vb-hxm0",
        executable_evidence_target: Some(
            "crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs",
        ),
        deferred_follow_up_bead: None,
    }
}

#[test]
fn test_catalog_lists_every_master_doc_behavior_by_scenario_id() {
    // Given: master acceptance behavior areas from velvet-ballistics-MASTER.md.
    let scenarios = catalog();

    // When: the acceptance catalog is validated.
    let validation = validate_catalog(scenarios);

    // Then: every behavior area has an executable scenario id and exact assertions.
    assert_eq!(validation, Ok(()));
    let actual_behavior_rows: Vec<(&str, &str)> = scenarios
        .iter()
        .map(|scenario| (scenario.master_behavior, scenario.id))
        .collect();
    assert_eq!(actual_behavior_rows, REQUIRED_BEHAVIOR_ROWS);
}

#[test]
fn test_catalog_maps_existing_tests_to_covered_scenarios() {
    // Given: existing tests and follow-up beads from the BDD release epic.
    let scenarios = catalog();

    // When: catalog rows are inspected for test-target evidence.
    let executable_test_targets: Vec<&str> = scenarios
        .iter()
        .filter_map(|scenario| scenario.executable_evidence_target)
        .collect();
    let follow_up_beads: Vec<&str> = scenarios
        .iter()
        .filter_map(|scenario| scenario.deferred_follow_up_bead)
        .collect();

    // Then: covered scenarios point at real test files and deferred gaps point only at beads.
    assert_eq!(executable_test_targets.len(), 6);
    assert_eq!(follow_up_beads.len(), 4);
    assert_eq!(
        executable_test_targets,
        vec![
            "crates/workspace_tests/tests/vb_37lc_canonical_spelling_red.rs",
            "crates/workspace_tests/tests/bdd_validation_tests.rs",
            "crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs",
            "crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs",
            "crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs",
            "crates/workspace_tests/tests/vb_5xs4_test_loop_inventory_red.rs",
        ]
    );
    assert_eq!(
        follow_up_beads,
        vec!["vb-te1i", "vb-rpch", "vb-0sps", "vb-ssei"]
    );
    let related_beads: Vec<(&str, &str)> = scenarios
        .iter()
        .map(|scenario| (scenario.id, scenario.related_bead))
        .collect();
    assert_eq!(
        related_beads,
        vec![
            ("VB-BDD-CATALOG-001", "vb-37lc"),
            ("VB-BDD-CATALOG-002", "vb-qi37.8"),
            ("VB-BDD-CATALOG-003", "vb-core-yaml-e2e-chain"),
            ("VB-BDD-CATALOG-004", "vb-vt2f"),
            ("VB-BDD-CATALOG-005", "vb-te1i"),
            ("VB-BDD-CATALOG-006", "vb-rpch"),
            ("VB-BDD-CATALOG-007", "vb-0sps"),
            ("VB-BDD-CATALOG-008", "vb-ssei"),
            ("VB-BDD-CATALOG-009", "vb-e4mt"),
            ("VB-BDD-CATALOG-010", "vb-5xs4"),
        ]
    );
}

#[test]
fn test_catalog_direct_runtime_api_row_points_to_executable_evidence_when_vt2f_is_done() {
    // Given: catalog row VB-BDD-CATALOG-004 for vb-vt2f direct runtime API evidence.
    let scenarios = catalog();

    // When: the row is selected by exact scenario id.
    let direct_api_row: Vec<&Scenario> = scenarios
        .iter()
        .filter(|scenario| scenario.id == "VB-BDD-CATALOG-004")
        .collect();

    // Then: vt2f is executable evidence, not a deferred follow-up.
    assert_eq!(direct_api_row.len(), 1);
    let row = direct_api_row.first().copied();
    assert_eq!(
        row.map(|scenario| scenario.executable_evidence_target),
        Some(Some(
            "crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs"
        ))
    );
    assert_eq!(
        row.map(|scenario| scenario.deferred_follow_up_bead),
        Some(None)
    );
    assert_eq!(row.map(|scenario| scenario.related_bead), Some("vb-vt2f"));
}

#[test]
fn test_catalog_gate_fails_when_behavior_has_no_scenario() {
    // Given: an empty catalog.
    let scenarios = [];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: release evidence is rejected instead of counted.
    assert_eq!(validation, Err(CatalogValidationError::EmptyCatalog));
}

#[test]
fn test_catalog_gate_fails_when_scenario_has_no_test_target() {
    // Given: a scenario with Given/When/Then but no executable test target.
    let scenarios = [Scenario {
        id: "VB-BDD-CATALOG-BAD-TARGET",
        master_behavior: "bad target fixture",
        given: "a catalog row",
        when: "the target is blank",
        then: "the row is rejected",
        public_surface: "workspace test boundary",
        fixture: "isolated bad catalog fixture",
        expected_outcome: Some("rejected"),
        expected_error: None,
        durability_profile: "none",
        related_bead: "vb-hxm0",
        executable_evidence_target: None,
        deferred_follow_up_bead: None,
    }];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: prose-only scenarios cannot count as release evidence.
    assert_eq!(
        validation,
        Err(CatalogValidationError::MissingEvidenceDisposition {
            scenario_id: "VB-BDD-CATALOG-BAD-TARGET".to_owned(),
        })
    );
}

#[test]
fn test_catalog_gate_fails_when_follow_up_is_disguised_as_executable_evidence() {
    // Given: a deferred follow-up bead placed in the executable evidence slot.
    let scenarios = [Scenario {
        id: "VB-BDD-CATALOG-BAD-EVIDENCE",
        master_behavior: "bad evidence fixture",
        given: "a catalog row with a follow-up bead",
        when: "the follow-up is labeled executable evidence",
        then: "the row is rejected",
        public_surface: "workspace test boundary",
        fixture: "isolated bad catalog fixture",
        expected_outcome: Some("rejected"),
        expected_error: None,
        durability_profile: "none",
        related_bead: "vb-hxm0",
        executable_evidence_target: Some("follow-up bead vb-hxm0"),
        deferred_follow_up_bead: None,
    }];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: a follow-up bead cannot be counted as executable test evidence.
    assert_eq!(
        validation,
        Err(CatalogValidationError::InvalidExecutableEvidenceTarget {
            scenario_id: "VB-BDD-CATALOG-BAD-EVIDENCE".to_owned(),
        })
    );
}

#[test]
fn test_catalog_gate_fails_when_deferred_gap_does_not_match_related_bead() {
    // Given: a deferred gap whose follow-up bead does not match the owning row.
    let scenarios = [Scenario {
        id: "VB-BDD-CATALOG-BAD-FOLLOW-UP",
        master_behavior: "bad follow-up fixture",
        given: "a catalog row with a mismatched bead",
        when: "the row is deferred",
        then: "the mismatch is rejected",
        public_surface: "workspace test boundary",
        fixture: "isolated bad catalog fixture",
        expected_outcome: Some("rejected"),
        expected_error: None,
        durability_profile: "none",
        related_bead: "vb-hxm0",
        executable_evidence_target: None,
        deferred_follow_up_bead: Some("vb-other"),
    }];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: deferred evidence remains traceable to the exact owning bead.
    assert_eq!(
        validation,
        Err(CatalogValidationError::InvalidDeferredFollowUpBead {
            scenario_id: "VB-BDD-CATALOG-BAD-FOLLOW-UP".to_owned(),
        })
    );
}

#[test]
fn test_catalog_gate_fails_when_given_when_then_is_missing() {
    // Given: a scenario with an empty Given clause and otherwise valid evidence.
    let mut scenario = valid_bad_scenario("VB-BDD-CATALOG-MISSING-GWT");
    scenario.given = "";
    let scenarios = [scenario];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: the exact MissingGivenWhenThen variant identifies the row.
    assert_eq!(
        validation,
        Err(CatalogValidationError::MissingGivenWhenThen {
            scenario_id: "VB-BDD-CATALOG-MISSING-GWT".to_owned(),
        })
    );
}

#[test]
fn test_catalog_gate_fails_when_exact_assertion_is_missing() {
    // Given: a scenario with neither expected outcome nor expected error.
    let mut scenario = valid_bad_scenario("VB-BDD-CATALOG-MISSING-ASSERTION");
    scenario.expected_outcome = None;
    scenario.expected_error = None;
    let scenarios = [scenario];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: the exact MissingExactAssertion variant identifies the row.
    assert_eq!(
        validation,
        Err(CatalogValidationError::MissingExactAssertion {
            scenario_id: "VB-BDD-CATALOG-MISSING-ASSERTION".to_owned(),
        })
    );
}

#[test]
fn test_catalog_gate_fails_when_evidence_dispositions_conflict() {
    // Given: a scenario with both executable target and deferred follow-up bead set.
    let mut scenario = valid_bad_scenario("VB-BDD-CATALOG-CONFLICTING-EVIDENCE");
    scenario.deferred_follow_up_bead = Some("vb-hxm0");
    let scenarios = [scenario];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: the exact ConflictingEvidenceDisposition variant identifies the row.
    assert_eq!(
        validation,
        Err(CatalogValidationError::ConflictingEvidenceDisposition {
            scenario_id: "VB-BDD-CATALOG-CONFLICTING-EVIDENCE".to_owned(),
        })
    );
}

#[test]
fn test_catalog_gate_fails_when_public_surface_names_private_or_helper_api() {
    // Given: a scenario whose public surface is actually private/helper-coupled.
    let mut scenario = valid_bad_scenario("VB-BDD-CATALOG-PRIVATE-SURFACE");
    scenario.public_surface = "private helper module";
    let scenarios = [scenario];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: the exact PrivateSurface variant identifies the row.
    assert_eq!(
        validation,
        Err(CatalogValidationError::PrivateSurface {
            scenario_id: "VB-BDD-CATALOG-PRIVATE-SURFACE".to_owned(),
        })
    );
}

#[test]
fn test_catalog_gate_fails_when_fixture_is_not_isolated() {
    // Given: a scenario that declares a shared fixture.
    let mut scenario = valid_bad_scenario("VB-BDD-CATALOG-SHARED-FIXTURE");
    scenario.fixture = "shared catalog fixture";
    let scenarios = [scenario];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: the exact SharedFixture variant identifies the row.
    assert_eq!(
        validation,
        Err(CatalogValidationError::SharedFixture {
            scenario_id: "VB-BDD-CATALOG-SHARED-FIXTURE".to_owned(),
        })
    );
}

#[test]
fn test_catalog_gate_fails_when_scenario_id_is_duplicate() {
    // Given: two otherwise valid scenarios with the same id.
    let first = valid_bad_scenario("VB-BDD-CATALOG-DUPLICATE");
    let second = valid_bad_scenario("VB-BDD-CATALOG-DUPLICATE");
    let scenarios = [first, second];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: the exact DuplicateScenarioId variant identifies the repeated id.
    assert_eq!(
        validation,
        Err(CatalogValidationError::DuplicateScenarioId {
            scenario_id: "VB-BDD-CATALOG-DUPLICATE".to_owned(),
        })
    );
}
