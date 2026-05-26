# Martin Fowler Test Plan: vb-37lc Canonical Spelling Scan

## Happy Path Tests
- `given_only_canonical_names_when_repository_is_scanned_then_report_has_zero_findings`
- `given_documented_repository_path_exception_when_scanned_then_scan_passes`
- `given_documented_master_filename_exception_when_scanned_then_scan_passes`
- `given_explicit_migration_reference_when_scanned_then_scan_passes`
- `given_complete_canonical_table_when_config_validates_then_config_is_accepted`

## Error Path Tests
- `given_missing_or_external_root_when_scan_starts_then_invalid_root_error`
- `given_incomplete_canonical_table_when_config_validates_then_invalid_configuration_error`
- `given_broad_wildcard_allowlist_when_config_validates_then_invalid_configuration_error`
- `given_invalid_legacy_spelling_outside_allowlist_when_repository_is_scanned_then_gate_fails_closed`
- `given_wrong_crate_or_module_spelling_when_repository_is_scanned_then_gate_fails_closed`
- `given_unreadable_selected_file_when_scanned_then_input_read_failed_is_returned`
- `given_file_discovery_failure_when_inputs_are_discovered_then_file_discovery_failed_is_returned`
- `given_invalid_scan_pattern_when_config_validates_then_pattern_compilation_failed_is_returned`
- `given_unwritable_report_destination_when_report_is_rendered_then_report_write_failed_is_returned`

## Edge Case Tests
- `given_empty_repository_surface_when_scanned_then_zero_findings_report_is_returned`
- `given_file_with_invalid_spelling_at_first_column_when_scanned_then_column_is_one`
- `given_file_with_multiple_invalid_occurrences_on_one_line_when_scanned_then_each_occurrence_is_reported`
- `given_path_containing_exception_substring_when_classified_then_it_is_not_automatically_allowed`
- `given_same_inputs_in_different_orders_when_scanned_then_reports_are_identical`
- `given_documented_excluded_binary_path_when_discovering_inputs_then_path_is_not_scanned`
- `given_source_doc_manifest_script_when_discovering_inputs_then_path_is_scanned`

## Contract Verification Tests
- `given_canonical_table_when_loaded_then_product_binary_package_and_rig_are_ballistics`
- `given_canonical_table_when_loaded_then_crate_module_and_database_are_underscore_ballistics`
- `given_canonical_table_when_loaded_then_language_version_is_v1`
- `given_legacy_occurrence_without_exception_when_classified_then_invalid_legacy_is_returned`
- `given_invalid_occurrence_when_reported_then_path_line_column_class_and_remediation_are_exact`
- `given_repository_when_scan_runs_then_git_diff_remains_empty`
- `given_scan_source_when_forbidden_construct_scan_runs_then_no_forbidden_constructs_are_present`
- `given_scan_change_when_static_dependency_check_runs_then_runtime_core_has_no_yaml_json_or_http`
- `given_scan_change_when_quality_gate_interface_is_reviewed_then_no_public_api_stability_claim_is_made`
- `given_state_one_artifacts_when_handoff_occurs_then_no_release_artifact_is_produced`

## Error Variant Coverage
- ERR-001: `given_missing_or_external_root_when_scan_starts_then_invalid_root_error`
- ERR-002: `given_incomplete_canonical_table_when_config_validates_then_invalid_configuration_error`; `given_broad_wildcard_allowlist_when_config_validates_then_invalid_configuration_error`
- ERR-003: `given_file_discovery_failure_when_inputs_are_discovered_then_file_discovery_failed_is_returned`
- ERR-004: `given_unreadable_selected_file_when_scanned_then_input_read_failed_is_returned`
- ERR-005: `given_invalid_scan_pattern_when_config_validates_then_pattern_compilation_failed_is_returned`
- ERR-006: `given_invalid_legacy_spelling_outside_allowlist_when_repository_is_scanned_then_gate_fails_closed`
- ERR-007: `given_unwritable_report_destination_when_report_is_rendered_then_report_write_failed_is_returned`

## Given/When/Then Scenarios

### Scenario 1: Canonical names pass
Given: a repository surface contains only `velvet-ballistics`, `velvet_ballistics`, and `velvet-ballistics/v1` in their valid naming contexts.
When: the canonical spelling scan runs with the complete scan configuration.
Then:
- The scan returns `Ok(ScanReport)`.
- The report contains zero findings.
- The quality gate exits successfully.

### Scenario 2: Documented legacy exception passes
Given: a scanned document contains a legacy spelling only as the current external repository path, the current master filename, or an explicitly labeled migration reference.
When: the occurrence classifier evaluates the text.
Then:
- The occurrence is classified as `allowed_legacy`.
- No invalid finding is emitted for that occurrence.
- The exception does not allow neighboring paths or unrelated text.

### Scenario 3: Invalid legacy spelling fails closed
Given: a source, doc, manifest, script, or configured bead reference contains legacy project spelling without explicit migration context.
When: the repository scan runs.
Then:
- The scan returns `NamingScanError::InvalidCanonicalSpelling`.
- Findings are nonempty.
- The first finding includes exact path, line, column, spelling class, and remediation.
- The quality gate fails.

### Scenario 4: Wrong crate/module spelling fails closed
Given: a Rust source or manifest context uses a crate or module spelling other than `velvet_ballistics`.
When: the repository scan runs.
Then:
- The occurrence is reported as invalid canonical naming.
- The remediation states the required crate/module spelling.
- No file is modified by the scan.

### Scenario 5: Deterministic report ordering
Given: the same set of invalid findings is discovered in different traversal orders.
When: the scan renders the report.
Then:
- Findings are sorted by path, line, column, and spelling class.
- The rendered report is byte-for-byte identical for identical inputs and configuration.

### Scenario 6: Failures remain typed
Given: file discovery, file reading, text decoding, pattern compilation, or report writing fails.
When: the scan reaches the failing operation.
Then:
- The operation returns the matching `NamingScanError` variant.
- No unsafe, unwrap, expect, panic, todo, unimplemented, dbg, unchecked indexing/slicing/casts/arithmetic, or ignored `Result` is required to handle the failure.

## End-to-End Scenario

### Scenario 7: Quality gate blocks invalid naming
Given: the scan is wired into the canonical quality lane and a fixture introduces invalid legacy spelling outside the allowlist.
When: the quality lane runs.
Then:
- The lane fails before completion.
- The output points to the exact invalid occurrence.
- Replacing the invalid occurrence with the canonical spelling makes the same lane pass.
