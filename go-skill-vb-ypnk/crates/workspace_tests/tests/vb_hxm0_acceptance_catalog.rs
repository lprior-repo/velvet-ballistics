#![forbid(unsafe_code)]

use velvet_ballastics_workspace_tests::acceptance_catalog::{
    CatalogValidationError, Scenario, catalog, validate_catalog,
};

const REQUIRED_BEHAVIORS: &[&str] = &[
    "canonical naming and workspace spelling gates",
    "validation gates reject malformed workflow parts",
    "YAML strict admission emits accepted artifacts before execution",
    "runtime direct API exposes submit, inspect, cancel, trace, and shutdown",
    "binary IPC rejects malformed frames before payload allocation",
    "storage recovery preserves journal evidence and rejects corrupt records",
    "generated Rust remains semantically equivalent to IR mode",
    "capability admission fails closed for missing action grants",
    "resource bounds and hot-path budgets fail before unbounded allocation",
    "test evidence must be executable and assertion-strong",
];

#[test]
fn test_catalog_lists_every_master_doc_behavior_by_scenario_id() {
    // Given: master acceptance behavior areas from velvet-ballistics-MASTER.md.
    let scenarios = catalog();

    // When: the acceptance catalog is validated.
    let validation = validate_catalog(scenarios);

    // Then: every behavior area has an executable scenario id and exact assertions.
    assert_eq!(validation, Ok(()));
    for behavior in REQUIRED_BEHAVIORS {
        assert!(
            scenarios
                .iter()
                .any(|scenario| scenario.master_behavior == *behavior && !scenario.id.is_empty()),
            "missing behavior catalog entry for {behavior}"
        );
    }
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
    assert_eq!(executable_test_targets.len(), 5);
    assert_eq!(follow_up_beads.len(), 5);
    assert_eq!(
        executable_test_targets,
        vec![
            "crates/workspace_tests/tests/vb_37lc_canonical_spelling_red.rs",
            "crates/workspace_tests/tests/bdd_validation_tests.rs",
            "crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs",
            "crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs",
            "crates/workspace_tests/tests/vb_5xs4_test_loop_inventory_red.rs",
        ]
    );
    assert_eq!(
        follow_up_beads,
        vec!["vb-vt2f", "vb-te1i", "vb-rpch", "vb-0sps", "vb-ssei"]
    );
    assert!(
        scenarios
            .iter()
            .all(|scenario| !scenario.related_bead.is_empty())
    );
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
