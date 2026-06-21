//! RED tests for bead vb-5xs4 weak Rust test-loop inventory.
//!
//! Given the approved contract and test plan, these tests target only the
//! contracted public API. They are intentionally compile-red until State 3 adds
//! `quality::test_loop_inventory` and its public domain types.

use proptest::prelude::*;
use vb_workspace_tests::quality::test_loop_inventory::{
    AssignmentEvidence, BehaviorEvidence, CaseEvidence, CaseLabel, Disposition, DispositionKind,
    DispositionSelection, ExceptionMetadata, ExceptionReason, ExceptionScope, Finding, FindingId,
    FindingSummary, Inventory, InventoryError, InventoryReport, InventoryScope, LabelEvidence,
    LabelingPolicy, Location, LoopPattern, LoopPatternKind, LoopRisk, MutationEvidence, OwnerName,
    RepairMetadata, ReportAction, ReportFinding, RiskReason, SafeLabelInput, SourceText, TestFile,
    ValidatedFinding, ValidatedInventory, WorkspaceRoot, assign_disposition, classify_loop_pattern,
    discover_rust_test_files, render_inventory_report, scan_test_file, validate_inventory,
};

fn workspace_root(path: &'static str) -> WorkspaceRoot {
    WorkspaceRoot::new(path)
}

fn candidate_file(path: &'static str) -> TestFile {
    TestFile::new(path)
}

fn location(line: u32, column: u32) -> Location {
    Location::new(line, column)
}

fn fid(value: &'static str) -> FindingId {
    FindingId::new(value)
}

fn behavior(value: &'static str) -> BehaviorEvidence {
    BehaviorEvidence::new(value)
}

fn case_label(value: &'static str) -> CaseLabel {
    CaseLabel::new(value)
}

fn cases(values: &[&'static str]) -> CaseEvidence {
    require_valid_fixture(
        CaseEvidence::new(values.iter().map(|value| (*value).to_owned()).collect()),
        "valid case evidence fixture",
    )
}

fn exception(
    reason: &'static str,
    scope: &'static str,
    owner: &'static str,
    review_trigger: &'static str,
) -> ExceptionMetadata {
    require_valid_fixture(
        ExceptionMetadata::new(reason, scope, owner, review_trigger),
        "valid exception metadata fixture",
    )
}

fn safe_labeling_finding(
    path: &'static str,
    location: &'static str,
    behavior: &'static str,
    cases: Vec<String>,
) -> ValidatedFinding {
    require_valid_fixture(
        ValidatedFinding::safe_labeling(path, location, behavior, cases),
        "valid safe-label finding fixture",
    )
}

fn inventory_with_findings(findings: Vec<ValidatedFinding>) -> ValidatedInventory {
    require_valid_fixture(
        ValidatedInventory::with_findings(findings, MutationEvidence::NotProvided, None),
        "valid inventory fixture",
    )
}

fn invalid_exception_inventory_result(
    exception: ExceptionMetadata,
) -> Result<ValidatedInventory, InventoryError> {
    ValidatedInventory::with_findings(
        vec![invalid_exception_finding(exception)],
        MutationEvidence::NotProvided,
        None,
    )
}

fn invalid_exception_finding(exception: ExceptionMetadata) -> ValidatedFinding {
    ValidatedFinding::accepted_exception("tests/weak.rs", "7:5", exception)
}

fn invalid_exception_with_empty_reason() -> ExceptionMetadata {
    exception_parts("", "one file", "qa-owner", "re-review on edit")
}

fn invalid_exception_with_empty_scope() -> ExceptionMetadata {
    exception_parts("legacy harness", "", "qa-owner", "re-review on edit")
}

fn invalid_exception_with_empty_owner() -> ExceptionMetadata {
    exception_parts("legacy harness", "one file", "", "re-review on edit")
}

fn invalid_exception_with_empty_trigger() -> ExceptionMetadata {
    exception_parts("legacy harness", "one file", "qa-owner", "")
}

fn exception_parts(reason: &str, scope: &str, owner: &str, trigger: &str) -> ExceptionMetadata {
    ExceptionMetadata {
        reason: ExceptionReason::new(reason),
        scope: ExceptionScope::new(scope),
        owner: OwnerName::new(owner),
        review_trigger: ReportAction::new(trigger),
    }
}

fn exception_policy_error(field: &str) -> InventoryError {
    InventoryError::PolicyViolation {
        rule: "accepted_exception_metadata_complete".to_owned(),
        field: field.to_owned(),
    }
}

fn empty_case_error(behavior: Option<&str>) -> InventoryError {
    let label = behavior.map_or_else(String::new, str::to_owned);
    InventoryError::AmbiguousCaseLabel {
        label,
        behavior: behavior.map(str::to_owned),
        case_count: 0,
    }
}

fn require_valid_fixture<T>(result: Result<T, InventoryError>, context: &str) -> T {
    assert_eq!(
        result.as_ref().map(|_value| ()).map_err(Clone::clone),
        Ok(()),
        "{context} must remain constructible"
    );
    match result {
        Ok(value) => value,
        Err(_error) => std::process::abort(),
    }
}

fn weak_table_pattern() -> LoopPattern {
    LoopPattern::new(
        "tests/weak.rs",
        location(7, 5),
        LoopPatternKind::TableLoop,
        1,
        LabelEvidence::Absent,
    )
}

fn weak_inventory_without_disposition() -> Inventory {
    Inventory::from_findings(vec![Finding::risky(
        "tests/weak.rs:7:5:TableLoop",
        RiskReason::MissingCaseIdentity,
        None,
    )])
}

#[path = "vb_5xs4_test_loop_inventory_red/classification_cases.rs"]
mod classification_cases;
#[path = "vb_5xs4_test_loop_inventory_red/discovery_cases.rs"]
mod discovery_cases;
#[path = "vb_5xs4_test_loop_inventory_red/disposition_cases.rs"]
mod disposition_cases;
#[path = "vb_5xs4_test_loop_inventory_red/property_cases.rs"]
mod property_cases;
#[path = "vb_5xs4_test_loop_inventory_red/report_cases.rs"]
mod report_cases;
#[path = "vb_5xs4_test_loop_inventory_red/report_error_cases.rs"]
mod report_error_cases;
#[path = "vb_5xs4_test_loop_inventory_red/scan_cases.rs"]
mod scan_cases;
#[path = "vb_5xs4_test_loop_inventory_red/validation_cases.rs"]
mod validation_cases;
