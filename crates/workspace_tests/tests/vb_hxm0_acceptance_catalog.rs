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
    let existing_test_targets = scenarios
        .iter()
        .filter(|scenario| scenario.covered_by_existing_test)
        .filter(|scenario| {
            scenario
                .test_target
                .starts_with("crates/workspace_tests/tests/")
        })
        .count();
    let follow_up_targets = scenarios
        .iter()
        .filter(|scenario| !scenario.covered_by_existing_test)
        .filter(|scenario| scenario.test_target.starts_with("follow-up bead vb-"))
        .count();

    // Then: covered scenarios point at real test files and gaps point at beads.
    assert!(existing_test_targets >= 5);
    assert!(follow_up_targets >= 4);
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
        test_target: "",
        covered_by_existing_test: false,
    }];

    // When: the catalog gate runs.
    let validation = validate_catalog(&scenarios);

    // Then: prose-only scenarios cannot count as release evidence.
    assert_eq!(
        validation,
        Err(CatalogValidationError::MissingTestTarget {
            scenario_id: "VB-BDD-CATALOG-BAD-TARGET".to_owned(),
        })
    );
}
