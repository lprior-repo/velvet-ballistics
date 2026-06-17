#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::derivable_impls,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::io_other_error,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::new_without_default,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

#![forbid(unsafe_code)]

use velvet_ballistics_workspace_tests::acceptance_catalog::{
    CatalogValidationError, Scenario, catalog, validate_catalog,
};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const VB_KYYF_TRACEABILITY_PATH: &str = ".evidence/vb-kyyf/acceptance-catalog-traceability.md";
const VB_KYYF_PROOF_OBLIGATIONS_PATH: &str = ".beads/vb-kyyf/proof-obligations.planned.jsonl";
const VB_NJJU_SCENARIO_ROWS: &[&str] = &[
    "BDD-NJJU-001",
    "BDD-NJJU-002",
    "BDD-NJJU-003",
    "BDD-NJJU-004",
];

const REQUIRED_BEHAVIOR_ROWS: &[(&str, &str)] = &[
    // VB-BDD-CATALOG-001 (canonical naming) was removed with vb_37lc spelling tests
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
        "codegen residue stays quarantined outside active workspace scope",
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
fn test_catalog_lists_every_master_doc_behavior_by_scenario_id() -> io::Result<()> {
    // Given: master acceptance behavior areas from velvet-ballistics-MASTER.md.
    let scenarios = catalog();

    // When: the acceptance catalog is validated.
    let validation = validate_catalog(scenarios);
    write_vb_kyyf_traceability(scenarios)?;

    // Then: every behavior area has an executable scenario id and exact assertions.
    assert_eq!(validation, Ok(()));
    let actual_behavior_rows: Vec<(&str, &str)> = scenarios
        .iter()
        .filter(|scenario| scenario.id.starts_with("VB-BDD-CATALOG-"))
        .map(|scenario| (scenario.master_behavior, scenario.id))
        .collect();
    assert_eq!(actual_behavior_rows, REQUIRED_BEHAVIOR_ROWS);
    assert_bdd_kyyf_007_catalog_target_matches_planned_po_007(scenarios)?;
    assert_vb_njju_catalog_rows_are_public_and_executable(scenarios);
    assert!(fs::metadata(workspace_root()?.join(VB_KYYF_TRACEABILITY_PATH))?.len() > 0);

    Ok(())
}

fn assert_vb_njju_catalog_rows_are_public_and_executable(scenarios: &[Scenario]) {
    let actual_rows: Vec<&Scenario> = scenarios
        .iter()
        .filter(|scenario| scenario.related_bead == "vb-njju")
        .collect();
    let actual_ids: Vec<&str> = actual_rows.iter().map(|scenario| scenario.id).collect();

    assert_eq!(actual_ids, VB_NJJU_SCENARIO_ROWS);
    for scenario in actual_rows {
        assert!(!scenario.given.is_empty());
        assert!(!scenario.when.is_empty());
        assert!(!scenario.then.is_empty());
        assert!(
            scenario.public_surface.contains("public") || scenario.public_surface.contains("Moon")
        );
        assert!(scenario.fixture.contains("isolated vb-njju"));
        assert_eq!(scenario.deferred_follow_up_bead, None);
        assert!(
            scenario
                .executable_evidence_target
                .is_some_and(|target| target.starts_with("crates/"))
        );
    }
}

fn assert_bdd_kyyf_007_catalog_target_matches_planned_po_007(
    scenarios: &[Scenario],
) -> io::Result<()> {
    let actual_target = scenarios
        .iter()
        .find(|scenario| scenario.id == "BDD-KYYF-007" && scenario.related_bead == "vb-kyyf")
        .and_then(|scenario| scenario.executable_evidence_target);
    let planned = fs::read_to_string(workspace_root()?.join(VB_KYYF_PROOF_OBLIGATIONS_PATH))?;
    let po_007_declares_traceability_path = planned
        .lines()
        .find(|line| line.contains("\"id\": \"PO-007\""))
        .map(|line| line.contains(VB_KYYF_TRACEABILITY_PATH));

    assert_eq!(actual_target, Some(VB_KYYF_TRACEABILITY_PATH));
    assert_eq!(po_007_declares_traceability_path, Some(true));

    Ok(())
}

fn write_vb_kyyf_traceability(scenarios: &[Scenario]) -> io::Result<()> {
    let root = workspace_root()?;
    let target = root.join(VB_KYYF_TRACEABILITY_PATH);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut report = String::from("# vb-kyyf acceptance catalog traceability\n\n");
    report.push_str("Catalog evidence for PO-007 / BDD-KYYF-007.\n\n");
    report.push_str(
        "| Scenario id | Given | When | Then | Public surface | Evidence path | Traceability |\n",
    );
    report.push_str("|---|---|---|---|---|---|---|\n");

    for scenario in scenarios
        .iter()
        .copied()
        .filter(|scenario| scenario.related_bead == "vb-kyyf")
    {
        if let Some(evidence_path) = scenario.executable_evidence_target {
            report.push_str("| ");
            report.push_str(scenario.id);
            report.push_str(" | ");
            report.push_str(scenario.given);
            report.push_str(" | ");
            report.push_str(scenario.when);
            report.push_str(" | ");
            report.push_str(scenario.then);
            report.push_str(" | ");
            report.push_str(scenario.public_surface);
            report.push_str(" | ");
            report.push_str(evidence_path);
            report.push_str(" | bead vb-kyyf / ");
            report.push_str(obligation_for_scenario(scenario.id));
            report.push_str(" / ");
            report.push_str(scenario.master_behavior);
            report.push_str(" |\n");
        }
    }

    fs::write(target, report)
}

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "workspace_tests parent missing"))?;
    crates_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "workspace root parent missing"))
}

fn obligation_for_scenario(scenario_id: &str) -> &'static str {
    match scenario_id {
        "BDD-KYYF-001" => "PO-001",
        "BDD-KYYF-002" => "PO-002",
        "BDD-KYYF-003" => "PO-003",
        "BDD-KYYF-004" => "PO-004",
        "BDD-KYYF-007" => "PO-007",
        _ => "not-planned-for-vb-kyyf",
    }
}

#[test]
fn test_catalog_maps_existing_tests_to_covered_scenarios() {
    // Given: existing tests and follow-up beads from the BDD release epic.
    let scenarios = catalog();

    // When: catalog rows are inspected for test-target evidence.
    let executable_targets: Vec<&str> = scenarios
        .iter()
        .filter_map(|scenario| scenario.executable_evidence_target)
        .collect();
    let executable_test_targets: Vec<&str> = executable_targets
        .iter()
        .copied()
        .filter(|target| target.starts_with("crates/workspace_tests/tests/"))
        .collect();
    let executable_evidence_targets: Vec<&str> = executable_targets
        .iter()
        .copied()
        .filter(|target| target.starts_with(".evidence/"))
        .collect();
    let follow_up_beads: Vec<&str> = scenarios
        .iter()
        .filter_map(|scenario| scenario.deferred_follow_up_bead)
        .collect();

    // Then: covered scenarios point at real test/evidence files and deferred gaps point only at beads.
    // Note: VB-BDD-CATALOG-001 was removed with vb_37lc spelling tests
    assert_eq!(executable_test_targets.len(), 10);
    assert_eq!(executable_evidence_targets.len(), 5);
    assert_eq!(executable_targets.len(), 15);
    assert_eq!(follow_up_beads.len(), 3);
    assert_eq!(
        executable_test_targets,
        vec![
            "crates/workspace_tests/tests/vb_njju_mutation_fuzz_property_closure.rs",
            "crates/workspace_tests/tests/vb_njju_mutation_fuzz_property_closure.rs",
            "crates/workspace_tests/tests/vb_njju_mutation_fuzz_property_closure.rs",
            "crates/workspace_tests/tests/vb_njju_mutation_fuzz_property_closure.rs",
            "crates/workspace_tests/tests/bdd_validation_tests.rs",
            "crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs",
            "crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs",
            "crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs",
            "crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs",
            "crates/workspace_tests/tests/vb_5xs4_test_loop_inventory_red.rs",
        ]
    );
    assert_eq!(follow_up_beads, vec!["vb-te1i", "vb-rpch", "vb-0sps"]);
    let related_beads: Vec<(&str, &str)> = scenarios
        .iter()
        .filter(|scenario| scenario.id.starts_with("VB-BDD-CATALOG-"))
        .map(|scenario| (scenario.id, scenario.related_bead))
        .collect();
    assert_eq!(
        related_beads,
        vec![
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
