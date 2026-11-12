# Martin Fowler Test Plan: vb-5xs4

These are executable-specification plans only. State 1 does not write tests.

## Happy Path Tests
- `given_known_weak_loop_fixture_when_inventory_runs_then_report_includes_location_kind_reason_and_action`
  - Given: a Rust test fixture contains a table loop with assertions that lack case identity.
  - When: the inventory operation scans the fixture.
  - Then: the report includes file path, stable location, loop pattern kind, risk reason, and a repair-required action.
- `given_case_labeled_loop_fixture_when_inventory_runs_then_safe_labeling_proven_is_recorded`
  - Given: a Rust test loop includes behavior name and case label in every assertion failure path.
  - When: the classifier evaluates the loop.
  - Then: the finding is assigned `SafeLabelingProven` with behavior and case evidence.
- `given_accepted_exception_fixture_when_inventory_runs_then_exception_metadata_is_complete`
  - Given: a loop is intentionally retained with documented exception metadata.
  - When: the inventory validates assignments.
  - Then: reason, scope, owner, and expiry/review trigger are present.
- `given_same_source_and_policy_when_inventory_runs_twice_then_reports_are_identical`
  - Given: the same normalized source tree and same labeling policy.
  - When: inventory runs twice.
  - Then: output order and classifications are identical.

## Error Path Tests
- `given_unreadable_workspace_when_inventory_runs_then_workspace_unreadable_error`
  - Given: the workspace root cannot be read.
  - When: discovery starts.
  - Then: `InventoryError::WorkspaceUnreadable` is returned.
- `given_out_of_scope_root_when_discovery_runs_then_input_root_out_of_scope`
  - Given: a caller requests a path outside `tests/**` or `crates/**`.
  - When: discovery validates scope.
  - Then: `InventoryError::InputRootOutOfScope` is returned before traversal.
- `given_invalid_text_candidate_when_scanned_then_typed_error_without_panic`
  - Given: a candidate file contains invalid text for the scanner.
  - When: scan starts.
  - Then: `InventoryError::InvalidUtf8` or another exact typed scanner error is returned without panic.
- `given_unreadable_candidate_file_when_scanned_then_file_read_failed_error`
  - Given: a candidate Rust test file exists in scope but cannot be read.
  - When: scan attempts to read it.
  - Then: `InventoryError::FileReadFailed` is returned with file context.
- `given_invalid_text_candidate_when_scanned_then_invalid_utf8_error_without_panic`
  - Given: a candidate file contains invalid UTF-8 bytes.
  - When: scan starts.
  - Then: `InventoryError::InvalidUtf8` is returned and no panic-style escape occurs.
- `given_unparseable_rust_when_scanned_then_parse_failed_error`
  - Given: a candidate source cannot be parsed enough to classify loops.
  - When: scan runs.
  - Then: `InventoryError::ParseFailed` is returned with file context.
- `given_ambiguous_case_label_when_classified_then_ambiguous_case_label_or_repair_required`
  - Given: a loop has a label that is not unique to behavior and case.
  - When: classification runs.
  - Then: the classifier returns `AmbiguousCaseLabel` or marks the loop `RepairRequired`; it must not mark it safe.
- `given_unassigned_risky_loop_when_quality_gate_runs_then_gate_fails`
  - Given: a risky loop finding has no repair, exception, or safe proof.
  - When: inventory validation runs.
  - Then: `InventoryError::UnassignedRiskyPattern` is returned.
- `given_unassigned_risky_loop_when_quality_gate_runs_then_unassigned_risky_pattern_error`
  - Given: a risky loop finding has no disposition.
  - When: the quality gate validates the inventory.
  - Then: the exact error variant is `InventoryError::UnassignedRiskyPattern`.
- `given_conflicting_dispositions_when_validated_then_conflicting_disposition_error`
  - Given: a risky finding has both repair and accepted exception assignments.
  - When: validation runs.
  - Then: `InventoryError::ConflictingDisposition` is returned.
- `given_deleted_test_when_inventory_compares_changes_then_destructive_change_detected`
  - Given: a test disappears and is presented as loop-quality improvement.
  - When: validation checks repair evidence.
  - Then: `InventoryError::DestructiveChangeDetected` is returned.
- `given_untraceable_generated_source_when_scanned_then_unsupported_generated_source_error`
  - Given: macro/generated code cannot be traced to a stable first-party source location.
  - When: scan attempts classification.
  - Then: `InventoryError::UnsupportedGeneratedSource` is returned or the source is excluded with evidence.
- `given_repository_rule_violation_when_static_gate_runs_then_policy_violation_error`
  - Given: implementation uses a forbidden construct such as panic-style escape, unsafe, unchecked indexing, or runtime-core YAML/JSON/HTTP.
  - When: static policy gates run.
  - Then: `InventoryError::PolicyViolation` or equivalent fail-closed policy evidence is produced.

## Edge Case Tests
- `given_empty_workspace_scope_when_inventory_runs_then_empty_valid_inventory_is_returned`
  - Empty bounded scope is valid only if no risky findings exist and the report says none discovered.
- `given_large_table_loop_when_classified_then_each_case_label_is_checked`
  - Large tables must not short-circuit label sufficiency checks.
- `given_nested_loops_in_test_when_scanned_then_each_loop_pattern_has_stable_location`
  - Nested loops produce distinct findings with stable locations.
- `given_helper_function_runs_table_cases_when_scanned_then_helper_driven_pattern_is_inventory_candidate`
  - Helper-driven repetition is not missed just because there is no local `for` loop.
- `given_macro_invocation_generates_test_cases_when_scanned_then_traceable_macro_source_is_classified`
  - Traceable macro use is inventoried or explicitly recorded as unsupported.
- `given_malformed_source_when_scanned_then_no_retained_loop_with_insufficient_context`
  - Malformed or macro-shaped input must not produce retained-loop findings unless behavior identity and case identity are proven.
- `given_non_risky_loop_and_risky_loop_when_validated_then_non_risky_loop_does_not_hide_risk`
  - Safe entries do not suppress unsafe entries.

## Contract Verification Tests
- `test_precondition_inventory_scope_is_bounded_to_first_party_test_surfaces`
- `test_precondition_case_label_policy_required_before_classification`
- `test_postcondition_every_risky_pattern_has_location_kind_reason_and_action`
- `test_postcondition_every_risky_pattern_has_exactly_one_disposition`
- `test_invariant_no_risky_loop_pattern_remains_unassigned`
- `test_invariant_retained_loop_failure_diagnostics_identify_behavior_and_case`
- `test_invariant_deletion_is_not_repair`
- `test_invariant_runtime_core_avoids_yaml_json_http_dependencies`
- `test_error_taxonomy_each_error_condition_returns_exact_inventory_error_variant`

## Individual Error Variant Traceability Tests
- ERR-001: `given_unreadable_workspace_when_inventory_runs_then_workspace_unreadable_error`
- ERR-002: `given_out_of_scope_root_when_discovery_runs_then_input_root_out_of_scope`
- ERR-003: `given_unreadable_candidate_file_when_scanned_then_file_read_failed_error`
- ERR-004: `given_invalid_text_candidate_when_scanned_then_invalid_utf8_error_without_panic`
- ERR-005: `given_unparseable_rust_when_scanned_then_parse_failed_error`
- ERR-006: `given_ambiguous_case_label_when_classified_then_ambiguous_case_label_or_repair_required`
- ERR-007: `given_unassigned_risky_loop_when_quality_gate_runs_then_unassigned_risky_pattern_error`
- ERR-008: `given_conflicting_dispositions_when_validated_then_conflicting_disposition_error`
- ERR-009: `given_deleted_test_when_inventory_compares_changes_then_destructive_change_detected`
- ERR-010: `given_untraceable_generated_source_when_scanned_then_unsupported_generated_source_error`
- ERR-011: `given_repository_rule_violation_when_static_gate_runs_then_policy_violation_error`

## Waiver Review Tests
- `review_waiver_001_has_clause_owner_expiry_and_compensating_evidence`
- `review_waiver_002_has_clause_owner_expiry_and_compensating_evidence`
- `review_waiver_003_has_clause_owner_expiry_and_compensating_evidence`
- `review_waiver_004_has_clause_owner_expiry_and_compensating_evidence`

## Given/When/Then Scenarios

### Scenario 1: Weak table loop becomes repair-required
Given: a Rust test loops over input cases and asserts without including a case label in failure context.
When: the inventory classifies the loop.
Then:
- the pattern is marked risky;
- the risk reason says failure context can hide the broken case;
- the disposition is `RepairRequired`;
- the quality gate can pass only if a repair owner/bead is recorded.

### Scenario 2: Case-labeled loop is retained safely
Given: a looped test includes stable behavior identity and per-case identity in each failure message or assertion context.
When: the inventory validates the finding.
Then:
- the disposition is `SafeLabelingProven`;
- the proof cites the exact behavior and case evidence;
- no repair bead is required for that finding.

### Scenario 3: Accepted exception requires complete metadata
Given: a loop is intentionally retained for a narrow reason.
When: the exception is assigned.
Then:
- reason, scope, owner, and expiry/review trigger are mandatory;
- missing metadata fails validation.

### Scenario 4: Unassigned risk fails closed
Given: at least one risky loop has no repair, exception, or safe-labeling proof.
When: the quality gate runs.
Then:
- validation returns `InventoryError::UnassignedRiskyPattern`;
- no passable inventory report is produced.

### Scenario 5: Test deletion is destructive
Given: a weak loop disappears from source without repair proof or accepted removal rationale.
When: inventory validation compares evidence.
Then:
- the change is classified as destructive;
- `InventoryError::DestructiveChangeDetected` is returned;
- deletion is not counted as quality improvement.

### Scenario 6: End-to-end inventory on real project surfaces
Given: `tests/**` and `crates/**` are discoverable in the workspace.
When: the inventory workflow runs with the repository labeling policy.
Then:
- all risky loop/table-loop/helper-loop findings are listed;
- every risky finding has exactly one disposition;
- retained loops include sufficient case identity or accepted exception metadata;
- the report makes no unsupported mutation-improvement claim.
