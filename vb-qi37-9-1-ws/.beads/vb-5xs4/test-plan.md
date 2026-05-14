# Test Plan: vb-5xs4 — Weak Rust Test Loop Inventory

## Summary
- Behaviors identified: 28 public contract behaviors across discovery, scanning, classification, disposition assignment, validation, reporting, deletion resistance, and policy gates.
- Trophy allocation: 31 named unit tests / 15 integration scenarios / 2 e2e scenarios / 2 static policy scenarios. The unit minimum is 5 named deletion-resistant tests per public function; integration remains the widest behavior layer because this feature scans real Rust test fixtures and validates end-to-end inventory state.
- Proptest invariants: 10.
- Fuzz targets: 4.
- Kani harnesses: 7.
- Error variants covered: `InventoryError::WorkspaceUnreadable`, `InputRootOutOfScope`, `FileReadFailed`, `InvalidUtf8`, `ParseFailed`, `AmbiguousCaseLabel`, `UnassignedRiskyPattern`, `ConflictingDisposition`, `DestructiveChangeDetected`, `UnsupportedGeneratedSource`, `PolicyViolation`.
- Mutation target: `cargo-mutants` kill rate must be >=90%, with 100% kill required for mutations touching disposition completeness, case-label sufficiency, scope filtering, deletion resistance, and typed error mapping.
- Red phase expectation: all executable tests described here must fail before implementation because `crate::quality::test_loop_inventory` and its public API are not yet present or not yet contract-compliant.

## Public API Under Test

Planned tests must use only these public contract signatures:

- `discover_rust_test_files(root: WorkspaceRoot, scope: InventoryScope) -> Result<Vec<TestFile>, InventoryError>`
- `scan_test_file(file: TestFile, text: SourceText) -> Result<Vec<LoopPattern>, InventoryError>`
- `classify_loop_pattern(pattern: LoopPattern, policy: LabelingPolicy) -> Result<LoopRisk, InventoryError>`
- `assign_disposition(risk: LoopRisk, evidence: AssignmentEvidence) -> Result<Disposition, InventoryError>`
- `validate_inventory(findings: Inventory) -> Result<ValidatedInventory, InventoryError>`
- `render_inventory_report(inventory: ValidatedInventory) -> Result<InventoryReport, InventoryError>`

Do not test private helpers, AST internals, parser implementation details, filesystem traversal internals, or exact terminal formatting. Assert exact public values, exact error variants, and observable report fields.

### File/Encoding Error Boundary Reconciliation

`InventoryError::FileReadFailed` is contract-required by `contract.md` ERR-003 and `proof-obligations.jsonl` target `crate::quality::test_loop_inventory::scan_test_file`. Therefore it remains a required public-boundary scenario for the contracted function `scan_test_file(file: TestFile, text: SourceText) -> Result<Vec<LoopPattern>, InventoryError>`.

To make this executable without inventing a second scanner API, `SourceText` must be a public domain input that can encode source acquisition outcomes before parsing:

- `SourceText::Text("...")` -> scanner parses text.
- `SourceText::ReadFailed { operation: "read_to_string" }` with `TestFile { path: "tests/unreadable_candidate.rs" }` -> `scan_test_file` returns exact `Err(InventoryError::FileReadFailed { path: "tests/unreadable_candidate.rs", operation: "read_to_string" })`.
- `SourceText::InvalidUtf8 { byte_offset: 3 }` with `TestFile { path: "tests/invalid_utf8.rs" }` -> `scan_test_file` returns exact `Err(InventoryError::InvalidUtf8 { path: "tests/invalid_utf8.rs", byte_offset: 3 })`.

If downstream implementation makes `SourceText` an infallible string-only type, it violates ERR-003/PRE-004 and must update/re-review the contract before removing `FileReadFailed` from public-boundary tests.

## Required Fixtures

Create fixture files under a test fixture root such as `tests/fixtures/vb_5xs4/` and use real temporary workspaces for integration tests.

1. `weak_table_loop_missing_case_label.rs`
   - Contains a `#[test]` with `for case in cases { assert_eq!(actual, expected); }` and no case name in assertion context.
   - Expected: risky table loop, `RepairRequired`, risk reason `MissingCaseIdentity`.
2. `weak_iterator_for_each_missing_behavior.rs`
   - Contains `.iter().for_each(|case| assert!(predicate(case)))` without behavior identity.
   - Expected: risky iterator loop, `RepairRequired`.
3. `safe_case_labeled_loop.rs`
   - Contains every assertion with stable behavior identity and per-case identity, e.g. assertion context includes `behavior=parser rejects invalid ids` and `case=empty-id`.
    - Expected: exact `Ok(LoopRisk::SafeLabelingProven { behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty", "whitespace"] })`.
4. `ambiguous_label_loop.rs`
   - Contains repeated label `case=invalid` under behavior `parser rejects invalid ids` for two different inputs.
   - Expected: exact `Err(InventoryError::AmbiguousCaseLabel { label: "invalid", behavior: Some("parser rejects invalid ids"), case_count: 2 })` from `classify_loop_pattern`.
5. `accepted_exception_loop.rs`
   - Contains intentionally retained loop plus complete exception metadata fixture supplied through `AssignmentEvidence`.
    - Expected: exact `Ok(Disposition::AcceptedException { reason: "bounded smoke loop", scope: "single deterministic fixture", owner: "Lewis", review_trigger: "mutation refresh" })`.
6. `accepted_exception_missing_owner.rs`
   - Same as above but missing owner.
   - Expected: exact `Err(InventoryError::PolicyViolation { rule: "accepted_exception_metadata_complete", field: "owner" })`.
7. `nested_loops.rs`
   - Contains outer and inner repeated assertions.
    - Expected: exact `[LoopPattern { path: "tests/nested_loops.rs", location: line 6 column 5, kind: NestedOuterLoop, assertion_count: 0 }, LoopPattern { path: "tests/nested_loops.rs", location: line 8 column 9, kind: NestedInnerLoop, assertion_count: 1 }]`.
8. `helper_driven_table_cases.rs`
   - A `#[test]` calls a helper that loops over cases and asserts.
   - Expected: helper-driven pattern is inventoried with pattern kind `HelperDrivenTableLoop`, stable location, and disposition requirement `RepairRequired`.
9. `traceable_macro_loop.rs`
   - First-party macro invocation expands or represents table-like test repetition with stable source span.
   - Expected: inventoried with stable first-party location and pattern kind `TraceableMacroLoop`.
10. `untraceable_generated_loop.rs`
    - Generated/macro-shaped source without stable first-party span.
    - Expected: exact `Err(InventoryError::UnsupportedGeneratedSource { path_or_macro: "untraceable_generated_loop", reason: "no_stable_first_party_location" })`.
11. `malformed_rust_unrecoverable.rs`
    - Invalid Rust syntax with unclosed delimiter at line 3 column 1.
    - Expected: exact `Err(InventoryError::ParseFailed { path: "tests/malformed_rust_unrecoverable.rs", location: line 3 column 1 })`.
12. `invalid_utf8.rs.bytes`
    - Non-UTF-8 bytes loaded through scanner file boundary.
    - Expected: exact `Err(InventoryError::InvalidUtf8 { path: "tests/invalid_utf8.rs", byte_offset: 3 })`.
13. `mixed_risky_and_non_risky.rs`
    - One safe loop and one risky unlabeled loop.
    - Expected: exact `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/mixed_risky_and_non_risky.rs:12:5:TableLoop" })`.
14. `deletion_baseline.rs` and `deletion_after.rs`
    - Baseline contains weak loop; after state deletes it without repair/exception/proof evidence.
    - Expected: exact `Err(InventoryError::DestructiveChangeDetected { path: "tests/deletion_baseline.rs", previous_finding: "tests/deletion_baseline.rs:7:5:TableLoop" })`.
15. `empty_tests_scope/`
    - Workspace with empty `tests/**` and `crates/**`.
    - Expected: `InventoryReport { risky_count: 0, findings: [], mutation_evidence: MutationEvidence::NotProvided, mutation_improvement_claim: None }`.
16. `vendor_generated_out_of_scope/`
    - Paths under `vendor/`, `target/`, external symlink, generated directory without whitelist.
    - Expected: exact discovery result excludes these paths when `InventoryScope::FirstPartyRustTests` has no generated-source whitelist.

Fixtures are deletion-resistant test assets: test code must assert each fixture path exists before scanning and must fail with exact `InventoryError::FileReadFailed { path: "tests/fixtures/vb_5xs4/weak_table_loop_missing_case_label.rs", operation: "fixture_preflight" }` for a missing `weak_table_loop_missing_case_label.rs` fixture. For `safe_case_labeled_loop.rs`, the exact missing-fixture error is `InventoryError::FileReadFailed { path: "tests/fixtures/vb_5xs4/safe_case_labeled_loop.rs", operation: "fixture_preflight" }`. For `ambiguous_label_loop.rs`, the exact missing-fixture error is `InventoryError::FileReadFailed { path: "tests/fixtures/vb_5xs4/ambiguous_label_loop.rs", operation: "fixture_preflight" }`. Do not treat a missing fixture as an empty passing scan.

## 1. Behavior Inventory

1. Workspace discovery returns bounded Rust test files when root is readable and scope is `tests/**` plus `crates/**`.
2. Workspace discovery rejects unreadable workspace roots with `InventoryError::WorkspaceUnreadable` when root cannot be read.
3. Workspace discovery rejects out-of-scope roots with `InventoryError::InputRootOutOfScope` before traversal when caller requests paths outside `tests/**` or `crates/**`.
4. Workspace discovery excludes vendored, external, target, and generated paths when they are not explicitly whitelisted.
5. File scanning returns loop patterns with stable file path, location, pattern kind, and source context when Rust test text contains repeated assertions.
6. `scan_test_file` returns exact `InventoryError::FileReadFailed` when called with `TestFile { path: "tests/unreadable_candidate.rs" }` and `SourceText::ReadFailed { operation: "read_to_string" }`.
7. File scanning returns `InventoryError::InvalidUtf8` when candidate bytes are not valid UTF-8.
8. File scanning returns exact `InventoryError::ParseFailed { path: "tests/malformed_rust_unrecoverable.rs", location: line 3 column 1 }` when Rust syntax has an unclosed delimiter at line 3 column 1.
9. File scanning returns exact `InventoryError::UnsupportedGeneratedSource` when generated or macro-expanded source cannot be traced to first-party test location.
10. Weak loop classifier marks unlabeled table loops `RepairRequired` when failure output cannot identify failing case.
11. Weak loop classifier marks iterator/closure table execution `RepairRequired` when failure output lacks behavior or case identity.
12. Weak loop classifier inventories helper-driven repeated assertions when helper execution hides repeated cases.
13. Weak loop classifier gives nested loops distinct findings when multiple loop patterns exist in one test.
14. Case-label classifier returns exact `SafeLabelingProven { behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty", "whitespace"] }` when failure paths expose behavior identity `parser rejects invalid ids` and case identities `empty` plus `whitespace`.
15. Case-label classifier rejects ambiguous labels with exact `InventoryError::AmbiguousCaseLabel` when labels are duplicated, unstable, or behavior-free.
16. Classifier is deterministic when normalized input and labeling policy are identical.
17. Classifier risk follows the exact evidence order `Absent -> RepairRequired`, `Duplicate -> AmbiguousCaseLabel`, `BehaviorOnly -> AmbiguousCaseLabel`, `CaseOnly -> AmbiguousCaseLabel`, `BehaviorAndCase -> SafeLabelingProven`.
18. Disposition assignment returns exactly one legal disposition when evidence supplies one complete repair, exception, or safe-labeling proof.
19. Accepted exception assignment requires reason, scope, owner, and expiry/review trigger when retaining a risky loop by exception.
20. Safe-labeling proof assignment requires behavior evidence and case evidence when retaining a loop as safe.
21. Inventory validation rejects risky findings without a disposition using `InventoryError::UnassignedRiskyPattern`.
22. Inventory validation rejects multiple dispositions for one finding using `InventoryError::ConflictingDisposition`.
23. Inventory validation rejects deleted tests presented as repair using `InventoryError::DestructiveChangeDetected`.
24. Inventory validation preserves risky findings when non-risky findings are present.
25. Inventory report returns exact `ReportFinding { path: "tests/weak.rs", location: "7:5", kind: TableLoop, risk_reason: MissingCaseIdentity, disposition: RepairRequired, owner: "Lewis", action: "vb-repair-1" }` for the canonical risky pattern fixture.
26. Inventory report never claims mutation-quality improvement without mutation evidence.
27. Runtime core remains free of YAML, JSON, and HTTP dependencies when implementing inventory/classification logic.
28. Static policy gate rejects forbidden constructs and fallible panic-style paths with exact `InventoryError::PolicyViolation` at the contracted unit/API boundary.

## 2. Trophy Allocation

| # | Behavior | Primary Layer | Supporting Layers | Rationale |
|---|----------|---------------|-------------------|-----------|
| 1 | Bounded discovery succeeds | Integration | proptest | Requires real fixture tree and path filtering. |
| 2 | Unreadable workspace error | Integration | manual QA, coverage | Filesystem permission behavior is runtime shell. |
| 3 | Out-of-scope roots rejected | Unit | proptest, Kani | Pure path-scope predicate should reject before traversal. |
| 4 | Vendor/generated excluded | Integration | proptest, static scan | Needs realistic paths and whitelist config. |
| 5 | Scan emits stable patterns | Integration | coverage | Parser/scanner boundary with real source fixtures. |
| 6 | File read failure | Unit | coverage | Contract maps file acquisition failure into `scan_test_file` through public `SourceText::ReadFailed`; no private file-reading boundary is tested. |
| 7 | Invalid UTF-8 | Integration | fuzz, coverage | Untrusted bytes boundary. |
| 8 | Parse failure | Integration | fuzz, coverage | Parser boundary. |
| 9 | Unsupported generated source | Integration | fuzz, manual QA | Source mapping boundary. |
| 10 | Unlabeled loop repair | Unit | proptest, Kani, mutation | Pure classification rule. |
| 11 | Iterator weak loop repair | Integration | mutation | Scanner plus classifier behavior. |
| 12 | Helper-driven loop inventoried | Integration | manual QA | Requires source-shape recognition beyond local `for`. |
| 13 | Nested loops distinct | Integration | proptest | Stable location and scanner behavior. |
| 14 | Safe labeling proven | Unit | Kani, proptest, mutation | Pure sufficiency predicate and constructor discipline. |
| 15 | Ambiguous labels rejected | Unit | Lean, Kani, proptest, mutation | Critical label-sufficiency law. |
| 16 | Deterministic classification | Unit | Lean, proptest | Pure determinism over normalized inputs. |
| 17 | Monotonic evidence refinement | Unit | Lean, proptest | Pure lattice property. |
| 18 | Exactly one disposition | Unit | Lean, Kani, proptest | Algebraic validation of disposition sum. |
| 19 | Exception metadata complete | Unit | integration, coverage, mutation | Public disposition/report value. |
| 20 | Safe proof fields complete | Unit | Lean, Kani, mutation | Constructor/validation invariant. |
| 21 | Unassigned risk fails closed | Integration | Lean, Kani, mutation | Whole inventory validation behavior. |
| 22 | Conflicting disposition rejected | Unit | Lean, Kani, mutation | Pure validation branch. |
| 23 | Deletion not repair | Integration | manual QA, mutation | Requires comparing before/after evidence. |
| 24 | Non-risky cannot suppress risk | Integration | proptest, mutation | Inventory aggregation behavior. |
| 25 | Report returns canonical finding fields | Integration | coverage | Renderer returns exact `ReportFinding { path: "tests/weak.rs", location: "7:5", kind: TableLoop, risk_reason: MissingCaseIdentity, disposition: RepairRequired, owner: "Lewis", action: "vb-repair-1" }`. |
| 26 | No mutation claim without evidence | Static | manual QA | Text/static report policy. |
| 27 | Runtime core no YAML/JSON/HTTP | Static | moon fast | Dependency/source policy. |
| 28 | Forbidden constructs policy | E2E | static scan, manual QA | Repository-wide policy gate observable via command. |

Layer ratio by named planned tests: Unit 31, integration scenarios 15, E2E scenarios 2, static policy scenarios 2. The behavior table still assigns each behavior to its strongest primary layer, and the executable plan requires at least 30 deletion-resistant unit tests across the six public functions before implementation starts.

## 2.1 Required Named Unit Test Inventory — 31 Named Unit Tests

These 31 unit tests are mandatory. Each is deletion-resistant: it must construct typed public inputs in test code, assert the target public function exists, assert exact returned value or exact `InventoryError` variant, and fail to compile or fail assertions if the target behavior is deleted. No unit test may assert only `is_ok()` or `is_err()`.

### `discover_rust_test_files` — 5 unit tests

1. `fn discover_returns_tests_and_crates_paths_when_scope_is_first_party_tests()`
   - Assertion: returned paths exactly equal `["crates/core/tests/loop_cases.rs", "tests/weak_loop.rs"]` after deterministic normalization.
2. `fn discover_returns_input_root_out_of_scope_when_scope_contains_parent_escape()`
   - Assertion: exact `Err(InventoryError::InputRootOutOfScope { path: "../outside" })`.
3. `fn discover_returns_input_root_out_of_scope_when_scope_contains_absolute_tmp()`
   - Assertion: exact `Err(InventoryError::InputRootOutOfScope { path: "/tmp" })`.
4. `fn discover_excludes_vendor_and_target_paths_when_not_whitelisted()`
   - Assertion: returned paths exactly exclude `vendor/crate/tests/vendor.rs` and `target/generated/test.rs`; included paths exactly equal first-party paths.
5. `fn discover_returns_workspace_unreadable_when_root_marker_is_unreadable()`
   - Assertion: exact `Err(InventoryError::WorkspaceUnreadable { root: "/tmp/vb-5xs4-missing-root" })` from the public discovery error mapping.

### `scan_test_file` — 6 unit tests

6. `fn scan_returns_table_loop_pattern_when_for_loop_contains_unlabeled_assertion()`
   - Assertion: exact single `LoopPattern { path: "tests/weak_table_loop_missing_case_label.rs", location: line 7 column 5, kind: TableLoop, assertion_count: 1 }`.
7. `fn scan_returns_iterator_loop_pattern_when_for_each_contains_assertion()`
   - Assertion: exact single `LoopPattern { path: "tests/weak_iterator_for_each_missing_behavior.rs", location: line 3 column 31, kind: IteratorTableLoop, assertion_count: 1 }`.
8. `fn scan_returns_two_patterns_when_source_contains_nested_loops()`
   - Assertion: exact patterns `[LoopPattern { path: "tests/nested_loops.rs", location: line 6 column 5, kind: NestedOuterLoop, assertion_count: 0 }, LoopPattern { path: "tests/nested_loops.rs", location: line 8 column 9, kind: NestedInnerLoop, assertion_count: 1 }]`.
9. `fn scan_returns_invalid_utf8_when_source_text_is_invalid()`
   - Assertion: exact `Err(InventoryError::InvalidUtf8 { path: "tests/invalid_utf8.rs", byte_offset: 3 })`.
10. `fn scan_returns_parse_failed_when_source_is_unrecoverable_rust()`
   - Assertion: exact `Err(InventoryError::ParseFailed { path: "tests/malformed_rust_unrecoverable.rs", location: line 3 column 1 })`.
11. `fn scan_returns_file_read_failed_when_source_text_records_read_failure()`
   - Assertion: for `TestFile { path: "tests/unreadable_candidate.rs" }` and `SourceText::ReadFailed { operation: "read_to_string" }`, exact `Err(InventoryError::FileReadFailed { path: "tests/unreadable_candidate.rs", operation: "read_to_string" })`.

### `classify_loop_pattern` — 5 unit tests

12. `fn classify_returns_repair_required_when_table_loop_has_no_case_label()`
   - Assertion: exact `Ok(LoopRisk::Risky { reason: MissingCaseIdentity, required_action: RepairRequired })`.
13. `fn classify_returns_ambiguous_case_label_when_duplicate_case_labels_are_present()`
   - Assertion: exact `Err(InventoryError::AmbiguousCaseLabel { label: "invalid", behavior: Some("parser rejects invalid ids"), case_count: 2 })`.
14. `fn classify_returns_ambiguous_case_label_when_behavior_identity_is_missing()`
   - Assertion: exact `Err(InventoryError::AmbiguousCaseLabel { label: "case=empty", behavior: None, case_count: 1 })`.
15. `fn classify_returns_safe_labeling_proven_when_every_assertion_has_behavior_and_case()`
   - Assertion: exact `Ok(LoopRisk::SafeLabelingProven { behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty", "whitespace"] })`.
16. `fn classify_returns_same_loop_risk_when_same_pattern_and_policy_are_reused()`
   - Assertion: both calls return exact `Ok(LoopRisk::Risky { reason: MissingCaseIdentity, required_action: RepairRequired })`.

### `assign_disposition` — 5 unit tests

17. `fn assign_returns_repair_required_when_repair_bead_evidence_is_complete()`
   - Assertion: exact `Ok(Disposition::RepairRequired { bead: "vb-repair-1", owner: "Lewis" })`.
18. `fn assign_returns_accepted_exception_when_exception_metadata_is_complete()`
   - Assertion: exact `Ok(Disposition::AcceptedException { reason: "bounded smoke loop", scope: "single deterministic fixture", owner: "Lewis", review_trigger: "mutation refresh" })`.
19. `fn assign_returns_policy_violation_when_exception_owner_is_missing()`
   - Assertion: exact `Err(InventoryError::PolicyViolation { rule: "accepted_exception_metadata_complete", field: "owner" })`.
20. `fn assign_returns_safe_labeling_proven_when_behavior_and_case_evidence_are_complete()`
   - Assertion: exact `Ok(Disposition::SafeLabelingProven { behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty"] })`.
21. `fn assign_returns_ambiguous_case_label_when_safe_proof_case_evidence_is_missing()`
   - Assertion: exact `Err(InventoryError::AmbiguousCaseLabel { label: "parser rejects invalid ids", behavior: Some("parser rejects invalid ids"), case_count: 0 })`.

### `validate_inventory` — 5 unit tests

22. `fn validate_returns_validated_inventory_when_every_risky_finding_has_one_disposition()`
   - Assertion: exact `Ok(ValidatedInventory { risky_count: 1, repair_required_count: 1, accepted_exception_count: 0, safe_labeling_count: 0, finding_ids: ["tests/weak.rs:7:5:TableLoop"] })`.
23. `fn validate_returns_unassigned_risky_pattern_when_risky_finding_has_no_disposition()`
   - Assertion: exact `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/weak.rs:7:5:TableLoop" })`.
24. `fn validate_returns_conflicting_disposition_when_repair_and_exception_are_both_present()`
   - Assertion: exact `Err(InventoryError::ConflictingDisposition { finding_id: "tests/weak.rs:7:5:TableLoop", dispositions: [RepairRequired, AcceptedException] })`.
25. `fn validate_returns_destructive_change_detected_when_baseline_finding_disappears()`
   - Assertion: exact `Err(InventoryError::DestructiveChangeDetected { path: "tests/deletion_baseline.rs", previous_finding: "tests/deletion_baseline.rs:7:5:TableLoop" })`.
26. `fn validate_returns_unassigned_risky_pattern_when_non_risky_finding_is_also_present()`
   - Assertion: exact `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/mixed_risky_and_non_risky.rs:12:5:TableLoop" })`.

### `render_inventory_report` — 5 unit tests

27. `fn report_contains_exact_risky_finding_fields_when_repair_required_is_present()`
   - Assertion: report finding fields exactly equal path `tests/weak.rs`, location `7:5`, kind `TableLoop`, reason `MissingCaseIdentity`, action `RepairRequired`, owner `Lewis`, bead `vb-repair-1`.
28. `fn report_contains_exact_exception_metadata_when_accepted_exception_is_present()`
   - Assertion: report exception fields exactly equal `reason: "bounded smoke loop"`, `scope: "single deterministic fixture"`, `owner: "Lewis"`, `review_trigger: "mutation refresh"`.
29. `fn report_contains_exact_safe_label_evidence_when_safe_labeling_is_present()`
   - Assertion: report safe-label fields exactly equal `behavior_evidence: "parser rejects invalid ids"` and `case_evidence: ["empty", "whitespace"]`.
30. `fn report_contains_zero_findings_and_no_mutation_claim_when_inventory_is_empty()`
   - Assertion: report exact risky count `0`, exact findings list `[]`, exact mutation evidence field `NotProvided`, and no mutation-improvement claim field.
31. `fn report_returns_policy_violation_when_runtime_policy_violation_record_is_rendered_as_success()`
   - Assertion: exact `Err(InventoryError::PolicyViolation { rule: "policy_violations_cannot_render_success", field: "report.status" })`.

## 3. BDD Scenarios

### Behavior: Workspace discovery returns bounded Rust test files
Test function: `fn discovery_returns_bounded_test_files_when_workspace_is_readable()`

Given: a temporary workspace containing `tests/weak_table_loop_missing_case_label.rs`, `crates/foo/tests/integration.rs`, `src/lib.rs`, and `target/generated.rs`.
When: `discover_rust_test_files(root, InventoryScope::FirstPartyRustTests)` is invoked.
Then: the returned `Vec<TestFile>` equals `[TestFile { path: "crates/foo/tests/integration.rs" }, TestFile { path: "tests/weak_table_loop_missing_case_label.rs" }]` in that order.
And: the returned list excludes `src/lib.rs` and `target/generated.rs`.

### Behavior: Workspace discovery rejects unreadable workspace roots
Test function: `fn discovery_returns_workspace_unreadable_when_root_cannot_be_read()`

Given: workspace root path `/tmp/vb-5xs4-missing-root` does not exist.
When: `discover_rust_test_files(root, scope)` is invoked.
Then: the exact result is `Err(InventoryError::WorkspaceUnreadable { root: "/tmp/vb-5xs4-missing-root" })`.

### Behavior: Workspace discovery rejects out-of-scope roots before traversal
Test function: `fn discovery_returns_input_root_out_of_scope_when_scope_escapes_tests_or_crates()`

Given: `root` is `WorkspaceRoot("/tmp/vb-5xs4-scope-root")` and `scope` is `InventoryScope::Roots(["../outside/sentinel_unreadable.rs"])`.
When: `discover_rust_test_files(root, scope)` is invoked.
Then: the exact result is `Err(InventoryError::InputRootOutOfScope { path: "../outside/sentinel_unreadable.rs" })`.

### Behavior: Workspace discovery excludes vendor and generated paths
Test function: `fn discovery_excludes_vendor_generated_and_external_paths_when_not_whitelisted()`

Given: `root` is `WorkspaceRoot("/tmp/vb-5xs4-vendor-scope")`, `scope` is `InventoryScope::FirstPartyRustTests`, and the workspace contains exactly `crates/a/tests/real.rs`, `vendor/crate/tests/vendor.rs`, `target/generated/test.rs`, and symlink label `external/tests/outside.rs`.
When: `discover_rust_test_files(root, scope)` is invoked.
Then: the exact result is `Ok([TestFile { path: "crates/a/tests/real.rs" }])`.

### Behavior: File scanning returns stable loop patterns
Test function: `fn scanner_returns_location_kind_and_context_when_source_contains_test_loop_patterns()`

Given: `weak_table_loop_missing_case_label.rs` fixture text.
When: `scan_test_file(file, text)` is invoked.
Then: the returned loop pattern list equals `[LoopPattern { path: "tests/weak_table_loop_missing_case_label.rs", location: line 7 column 5, kind: TableLoop, assertion_count: 1 }]`.

### Behavior: File scanning reports read failures through contracted `scan_test_file`
Test function: `fn scanner_returns_file_read_failed_when_source_text_records_read_failure()`

Given: `file` is `TestFile { path: "tests/unreadable_candidate.rs" }` and `text` is `SourceText::ReadFailed { operation: "read_to_string" }`.
When: `scan_test_file(file, text)` is invoked.
Then: the exact result is `Err(InventoryError::FileReadFailed { path: "tests/unreadable_candidate.rs", operation: "read_to_string" })`.

### Behavior: File scanning reports invalid UTF-8
Test function: `fn scanner_returns_invalid_utf8_when_candidate_bytes_are_not_text()`

Given: `file` is `TestFile { path: "tests/invalid_utf8.rs" }` and `text` is `SourceText::InvalidUtf8 { byte_offset: 3 }`.
When: `scan_test_file(file, text)` is invoked.
Then: the exact result is `Err(InventoryError::InvalidUtf8 { path: "tests/invalid_utf8.rs", byte_offset: 3 })`.

### Behavior: File scanning reports parse failure
Test function: `fn scanner_returns_parse_failed_when_rust_syntax_is_unrecoverable()`

Given: `malformed_rust_unrecoverable.rs` fixture text.
When: `scan_test_file(file, text)` is invoked.
Then: the exact result is `Err(InventoryError::ParseFailed { path: "tests/malformed_rust_unrecoverable.rs", location: line 3 column 1 })`.

### Behavior: File scanning reports unsupported untraceable generated source
Test function: `fn scanner_returns_unsupported_generated_source_when_source_location_is_untraceable()`

Given: `file` is `TestFile { path: "tests/untraceable_generated_loop.rs" }` and `text` is `SourceText::Text("// vb-5xs4-fixture:untraceable_generated_loop\ninclude!(concat!(env!(\"OUT_DIR\"), \"/generated_loop.rs\"));")`.
When: `scan_test_file(file, text)` is invoked.
Then: the exact result is `Err(InventoryError::UnsupportedGeneratedSource { path_or_macro: "untraceable_generated_loop", reason: "no_stable_first_party_location" })`.

### Behavior: Unlabeled table loop becomes repair-required
Test function: `fn classifier_returns_repair_required_when_table_loop_lacks_case_identity()`

Given: a `LoopPattern` representing repeated assertions over cases with no per-case label in failure context.
When: `classify_loop_pattern(pattern, policy)` is invoked.
Then: the returned `LoopRisk` exact value is `Ok(LoopRisk::Risky { reason: MissingCaseIdentity, required_action: RepairRequired })`.
And: the returned risk carries finding ID `tests/weak_table_loop_missing_case_label.rs:7:5:TableLoop` and owner action requirement `RepairRequired`.

### Behavior: Iterator table execution becomes repair-required
Test function: `fn inventory_returns_repair_required_when_iterator_loop_lacks_behavior_or_case_identity()`

Given: `file` is `TestFile { path: "tests/weak_iterator_for_each_missing_behavior.rs" }`, `text` is `SourceText::Text("// vb-5xs4-fixture:weak_iterator_for_each_missing_behavior\n#[test]\nfn iterator_cases() { [1, 2].iter().for_each(|case| assert!(*case > 0)); }")`, `pattern` is `LoopPattern { path: "tests/weak_iterator_for_each_missing_behavior.rs", location: line 3 column 31, kind: IteratorTableLoop, assertion_count: 1 }`, and `policy` is `LabelingPolicy::RequireBehaviorAndCaseIdentity`.
When: `scan_test_file(file, text)` is invoked.
Then: scanner output equals `[LoopPattern { path: "tests/weak_iterator_for_each_missing_behavior.rs", location: line 3 column 31, kind: IteratorTableLoop, assertion_count: 1 }]`.
When: `classify_loop_pattern(pattern, policy)` is invoked.
Then: classification returns exact `Ok(LoopRisk::Risky { finding_id: "tests/weak_iterator_for_each_missing_behavior.rs:3:31:IteratorTableLoop", reason: MissingBehaviorIdentity, required_action: RepairRequired })`.

### Behavior: Helper-driven repeated assertions are inventoried
Test function: `fn inventory_reports_helper_driven_pattern_when_test_helper_executes_table_cases()`

Given: `file` is `TestFile { path: "tests/helper_driven_table_cases.rs" }`, `text` is `SourceText::Text("// vb-5xs4-fixture:helper_driven_table_cases\n#[test]\nfn helper_driven() { run_cases(&[1, 2]); }\nfn run_cases(cases: &[i32]) { for case in cases { assert!(*case > 0); } }")`, `pattern` is `LoopPattern { path: "tests/helper_driven_table_cases.rs", location: line 4 column 30, kind: HelperDrivenTableLoop, assertion_count: 1 }`, and `policy` is `LabelingPolicy::RequireBehaviorAndCaseIdentity`.
When: `scan_test_file(file, text)` is invoked.
Then: scanner output equals `[LoopPattern { path: "tests/helper_driven_table_cases.rs", location: line 4 column 30, kind: HelperDrivenTableLoop, assertion_count: 1 }]`.
When: `classify_loop_pattern(pattern, policy)` is invoked.
Then: classification returns exact `Ok(LoopRisk::Risky { finding_id: "tests/helper_driven_table_cases.rs:4:30:HelperDrivenTableLoop", reason: MissingCaseIdentity, required_action: RepairRequired })`.

### Behavior: Nested loops produce distinct findings
Test function: `fn scanner_returns_distinct_findings_when_test_contains_nested_loops()`

Given: `file` is `TestFile { path: "tests/nested_loops.rs" }` and `text` is `SourceText::Text("// vb-5xs4-fixture:nested_loops\n#[test]\nfn nested() { for outer in [1] { for inner in [2] { assert_eq!(outer + inner, 3); } } }")`.
When: `scan_test_file(file, text)` is invoked.
Then: returned findings equal `[LoopPattern { path: "tests/nested_loops.rs", location: line 6 column 5, kind: NestedOuterLoop, assertion_count: 0 }, LoopPattern { path: "tests/nested_loops.rs", location: line 8 column 9, kind: NestedInnerLoop, assertion_count: 1 }]`.
When: `validate_inventory(Inventory { findings: [Finding { id: "tests/nested_loops.rs:6:5:NestedOuterLoop", disposition: Some(RepairRequired { bead: "vb-repair-nested-outer", owner: "Lewis" }) }, Finding { id: "tests/nested_loops.rs:8:9:NestedInnerLoop", disposition: None }] })` is invoked.
Then: the exact result is `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/nested_loops.rs:8:9:NestedInnerLoop" })`.

### Behavior: Sufficient labels produce safe labeling proof
Test function: `fn classifier_returns_safe_labeling_proven_when_behavior_and_case_evidence_are_present()`

Given: `safe_case_labeled_loop.rs` pattern has behavior identity `parser rejects invalid ids` and case identities `empty` and `whitespace` on its assertion failure paths.
When: `classify_loop_pattern(pattern, LabelingPolicy::RequireBehaviorAndCaseIdentity)` is invoked and its `LoopRisk::SafeLabelingProven` result is passed to `assign_disposition(risk, AssignmentEvidence::SafeLabelEvidence { behavior: "parser rejects invalid ids", cases: ["empty", "whitespace"] })`.
Then: the exact disposition is `Disposition::SafeLabelingProven { behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty", "whitespace"] }`.

### Behavior: Ambiguous labels are rejected
Test function: `fn classifier_rejects_ambiguous_case_label_when_label_does_not_identify_unique_behavior_and_case()`

Given: `ambiguous_label_loop.rs` pattern with duplicated or behavior-free labels.
When: `classify_loop_pattern(pattern, policy)` is invoked.
Then: the exact result is `Err(InventoryError::AmbiguousCaseLabel { label: "invalid", behavior: Some("parser rejects invalid ids"), case_count: 2 })`.

### Behavior: Classification is deterministic
Test function: `fn classifier_returns_identical_result_when_source_and_policy_are_identical()`

Given: `pattern` is `LoopPattern { path: "tests/weak.rs", location: line 7 column 5, kind: TableLoop, assertion_count: 1 }` and `policy` is `LabelingPolicy::RequireBehaviorAndCaseIdentity`.
When: `classify_loop_pattern(pattern.clone(), policy.clone())` is invoked twice.
Then: both invocations return exact `[LoopRisk::Risky { finding_id: "tests/weak.rs:7:5:TableLoop", reason: MissingCaseIdentity, required_action: RepairRequired }]`.

### Behavior: Evidence refinement follows exact label-state lattice
Test function: `fn classifier_returns_exact_result_for_each_label_evidence_state()`

Given: `pattern` is `LoopPattern { path: "tests/weak.rs", location: line 7 column 5, kind: TableLoop, assertion_count: 1 }` and policies encode label states `Absent`, `Duplicate`, `BehaviorOnly`, `CaseOnly`, and `BehaviorAndCase`.
When: `classify_loop_pattern(pattern.clone(), policy_for_state)` is invoked once for each label state in order `[Absent, Duplicate, BehaviorOnly, CaseOnly, BehaviorAndCase]`.
Then: absent evidence returns `Ok(LoopRisk::Risky { reason: MissingCaseIdentity, required_action: RepairRequired })`; ambiguous evidence returns exact `Err(InventoryError::AmbiguousCaseLabel { label: "invalid", behavior: Some("parser rejects invalid ids"), case_count: 2 })`; behavior-only evidence returns exact `Err(InventoryError::AmbiguousCaseLabel { label: "parser rejects invalid ids", behavior: Some("parser rejects invalid ids"), case_count: 0 })`; case-only evidence returns exact `Err(InventoryError::AmbiguousCaseLabel { label: "empty", behavior: None, case_count: 1 })`; behavior-plus-case evidence returns exact `Ok(LoopRisk::SafeLabelingProven { behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty"] })`.

### Behavior: Exactly one disposition is accepted
Test function: `fn assign_disposition_returns_one_disposition_when_evidence_has_one_legal_action()`

Given: a risky finding and evidence containing exactly one repair bead assignment.
When: `assign_disposition(risk, evidence)` is invoked.
Then: the exact result is `Ok(Disposition::RepairRequired { bead: "vb-repair-1", owner: "Lewis" })`.

### Behavior: Accepted exception metadata is mandatory
Test function: `fn assign_disposition_returns_accepted_exception_when_exception_metadata_is_complete()`

Given: `risk` is `LoopRisk::Risky { finding_id: "tests/accepted_exception_loop.rs:7:5:TableLoop", reason: AcceptedExceptionRequired, required_action: AcceptedException }` and `evidence` is `AssignmentEvidence::ExceptionEvidence { reason: "bounded smoke loop", scope: "single deterministic fixture", owner: "Lewis", review_trigger: "mutation refresh" }`.
When: `assign_disposition(risk, evidence)` is invoked.
Then: the exact disposition is `Disposition::AcceptedException { reason: "bounded smoke loop", scope: "single deterministic fixture", owner: "Lewis", review_trigger: "mutation refresh" }`.

Error variant:
Given: `inventory` is `Inventory { findings: [Finding { id: "tests/accepted_exception_loop.rs:7:5:TableLoop", risk: Risky, disposition: Some(AcceptedException { reason: "bounded smoke loop", scope: "single deterministic fixture", owner: "", review_trigger: "mutation refresh" }) }] }`.
When: `validate_inventory(inventory)` is invoked.
Then: exact result is `Err(InventoryError::PolicyViolation { rule: "accepted_exception_metadata_complete", field: "owner" })`.

### Behavior: Safe proof metadata is mandatory
Test function: `fn assign_disposition_returns_safe_labeling_proven_when_behavior_and_case_evidence_are_complete()`

Given: `risk` is `LoopRisk::SafeLabelingProven { finding_id: "tests/safe_case_labeled_loop.rs:5:5:TableLoop", behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty"] }` and `evidence` is `AssignmentEvidence::SafeLabelEvidence { behavior: "parser rejects invalid ids", cases: ["empty"] }`.
When: `assign_disposition(risk, evidence)` is invoked.
Then: exact disposition is `Disposition::SafeLabelingProven { behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty"] }`.

Error variant:
Given: `risk` is `LoopRisk::Risky { finding_id: "tests/safe_case_labeled_loop.rs:5:5:TableLoop", reason: MissingCaseIdentity, required_action: RepairRequired }` and `evidence` is `AssignmentEvidence::SafeLabelEvidence { behavior: "parser rejects invalid ids", cases: [] }`.
When: `assign_disposition(risk, evidence)` is invoked.
Then: exact result is `Err(InventoryError::AmbiguousCaseLabel { label: "parser rejects invalid ids", behavior: Some("parser rejects invalid ids"), case_count: 0 })`.

### Behavior: Unassigned risky findings fail closed
Test function: `fn validate_inventory_returns_unassigned_risky_pattern_when_risky_finding_has_no_disposition()`

Given: `inventory` is `Inventory { findings: [Finding { id: "tests/weak.rs:7:5:TableLoop", risk: Risky, disposition: None }] }`.
When: `validate_inventory(inventory)` is invoked.
Then: the exact result is `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/weak.rs:7:5:TableLoop" })`.

### Behavior: Conflicting dispositions fail closed
Test function: `fn validate_inventory_returns_conflicting_disposition_when_finding_has_multiple_actions()`

Given: `inventory` is `Inventory { findings: [Finding { id: "tests/weak.rs:7:5:TableLoop", risk: Risky, disposition: Some(RepairRequired { bead: "vb-repair-1", owner: "Lewis" }), second_disposition: Some(AcceptedException { reason: "bounded smoke loop", scope: "single deterministic fixture", owner: "Lewis", review_trigger: "mutation refresh" }) }] }`.
When: `validate_inventory(inventory)` is invoked.
Then: the exact result is `Err(InventoryError::ConflictingDisposition { finding_id: "tests/weak.rs:7:5:TableLoop", dispositions: [RepairRequired, AcceptedException] })`.

### Behavior: Deleted tests are destructive, not repair
Test function: `fn validate_inventory_returns_destructive_change_detected_when_test_disappears_without_repair_evidence()`

Given: `inventory_with_baseline_and_current_findings` is `Inventory { baseline_finding_ids: ["tests/deletion_baseline.rs:7:5:TableLoop"], current_finding_ids: [], dispositions: [] }`.
When: `validate_inventory(inventory_with_baseline_and_current_findings)` is invoked.
Then: exact result is `Err(InventoryError::DestructiveChangeDetected { path: "tests/deletion_baseline.rs", previous_finding: "tests/deletion_baseline.rs:7:5:TableLoop" })`.

### Behavior: Non-risky findings do not suppress risky findings
Test function: `fn validate_inventory_preserves_risky_finding_when_non_risky_finding_is_present()`

Given: `inventory` is `Inventory { findings: [Finding { id: "tests/mixed_risky_and_non_risky.rs:4:5:SafeLabeledLoop", risk: NonRisky, disposition: Some(SafeLabelingProven { behavior_evidence: "parser accepts valid ids", case_evidence: ["alpha"] }) }, Finding { id: "tests/mixed_risky_and_non_risky.rs:12:5:TableLoop", risk: Risky, disposition: None }] }`.
When: `validate_inventory(inventory)` is invoked.
Then: exact result is `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/mixed_risky_and_non_risky.rs:12:5:TableLoop" })`.

### Behavior: Report includes risky finding fields
Test function: `fn report_includes_path_location_kind_reason_and_owner_action_when_risky_pattern_exists()`

Given: `inventory` is `ValidatedInventory { findings: [ValidatedFinding { path: "tests/weak.rs", location: "7:5", kind: TableLoop, risk_reason: MissingCaseIdentity, disposition: RepairRequired { bead: "vb-repair-1", owner: "Lewis" } }], mutation_evidence: MutationEvidence::NotProvided, mutation_improvement_claim: None }`.
When: `render_inventory_report(inventory)` is invoked.
Then: report findings equal `[ReportFinding { path: "tests/weak.rs", location: "7:5", kind: TableLoop, risk_reason: MissingCaseIdentity, disposition: RepairRequired, owner: "Lewis", action: "vb-repair-1" }]`.

### Behavior: Report makes no unsupported mutation improvement claim
Test function: `fn report_contains_no_mutation_improvement_claim_when_inventory_has_no_mutation_evidence()`

Given: `validated_inventory` is `ValidatedInventory { findings: [], risky_count: 0, mutation_evidence: MutationEvidence::NotProvided, mutation_improvement_claim: None }`.
When: `render_inventory_report(validated_inventory)` is invoked.
Then: report exposes exact field `mutation_evidence: MutationEvidence::NotProvided`.
And: report exposes exact field `mutation_improvement_claim: None`.

### Behavior: Runtime core avoids YAML JSON HTTP dependencies
Test function: `fn static_scan_rejects_yaml_json_or_http_dependencies_in_runtime_core()`

Given: `inventory_with_clean_policy_evidence` is `Inventory { findings: [], policy_evidence: PolicyEvidence { forbidden_dependencies: [], forbidden_constructs: [] } }`.
When: `validate_inventory(inventory_with_clean_policy_evidence)` is invoked.
Then: `validate_inventory` returns exact `Ok(ValidatedInventory { policy_violation_count: 0, forbidden_dependency_count: 0, finding_ids: [] })`.
Given: `inventory_with_http_dependency` is `Inventory { findings: [], policy_evidence: PolicyEvidence { forbidden_dependencies: ["reqwest"], forbidden_constructs: [] } }`.
When: `validate_inventory(inventory_with_http_dependency)` is invoked.
Then: the exact result is `Err(InventoryError::PolicyViolation { rule: "runtime_core_no_yaml_json_http", field: "dependency" })`.

### Behavior: Repository policy violations fail closed
Test function: `fn static_gate_returns_policy_violation_when_forbidden_construct_is_present()`

Given: `Inventory` contains a runtime-source violation record for `unwrap` at `crates/quality/src/test_loop_inventory.rs:42`.
When: `validate_inventory(inventory)` is invoked.
Then: exact result is `Err(InventoryError::PolicyViolation { rule: "forbidden_construct", field: "unwrap@crates/quality/src/test_loop_inventory.rs:42" })`.

### Behavior: Contracted functions compose into deterministic inventory report
Test function: `fn contracted_functions_return_expected_report_for_exact_fixture_workspace()`

Given: `root` is `WorkspaceRoot("/tmp/vb-5xs4-contract-composition")` containing exactly `crates/core/tests/safe_case_labeled_loop.rs`, `tests/accepted_exception_loop.rs`, `tests/helper_driven_table_cases.rs`, `tests/weak_iterator_for_each_missing_behavior.rs`, and `tests/weak_table_loop_missing_case_label.rs`.
When: `discover_rust_test_files(root, InventoryScope::FirstPartyRustTests)` is invoked.
Then: discovery returns exact `[TestFile { path: "crates/core/tests/safe_case_labeled_loop.rs" }, TestFile { path: "tests/accepted_exception_loop.rs" }, TestFile { path: "tests/helper_driven_table_cases.rs" }, TestFile { path: "tests/weak_iterator_for_each_missing_behavior.rs" }, TestFile { path: "tests/weak_table_loop_missing_case_label.rs" }]`.
When: `scan_test_file` is invoked with exact inputs `[(TestFile { path: "crates/core/tests/safe_case_labeled_loop.rs" }, SourceText::Text("// vb-5xs4-fixture:safe_case_labeled_loop\n#[test]\nfn safe_cases() { for case in [\"empty\", \"whitespace\"] { assert_eq!(case.len(), expected(case), \"behavior=parser rejects invalid ids; case={case}\"); } }")), (TestFile { path: "tests/accepted_exception_loop.rs" }, SourceText::Text("// vb-5xs4-fixture:accepted_exception_loop\n#[test]\nfn smoke_loop() { for case in [1] { assert!(case > 0); } }")), (TestFile { path: "tests/helper_driven_table_cases.rs" }, SourceText::Text("// vb-5xs4-fixture:helper_driven_table_cases\n#[test]\nfn helper_driven() { run_cases(&[1, 2]); }\nfn run_cases(cases: &[i32]) { for case in cases { assert!(*case > 0); } }")), (TestFile { path: "tests/weak_iterator_for_each_missing_behavior.rs" }, SourceText::Text("// vb-5xs4-fixture:weak_iterator_for_each_missing_behavior\n#[test]\nfn iterator_cases() { [1, 2].iter().for_each(|case| assert!(*case > 0)); }")), (TestFile { path: "tests/weak_table_loop_missing_case_label.rs" }, SourceText::Text("// vb-5xs4-fixture:weak_table_loop_missing_case_label\n#[test]\nfn weak_table() { for case in [1, 2] { assert_eq!(case, case); } }"))]` in discovery order.
Then: scan output flattened in discovery order equals `[LoopPattern { path: "crates/core/tests/safe_case_labeled_loop.rs", location: line 6 column 5, kind: TableLoop, assertion_count: 2 }, LoopPattern { path: "tests/accepted_exception_loop.rs", location: line 7 column 5, kind: TableLoop, assertion_count: 1 }, LoopPattern { path: "tests/helper_driven_table_cases.rs", location: line 4 column 30, kind: HelperDrivenTableLoop, assertion_count: 1 }, LoopPattern { path: "tests/weak_iterator_for_each_missing_behavior.rs", location: line 3 column 31, kind: IteratorTableLoop, assertion_count: 1 }, LoopPattern { path: "tests/weak_table_loop_missing_case_label.rs", location: line 7 column 5, kind: TableLoop, assertion_count: 1 }]`.
When: `classify_loop_pattern(pattern, LabelingPolicy::RequireBehaviorAndCaseIdentity)` is invoked for each scanned pattern in discovery order.
Then: classification results in order equal `[Ok(LoopRisk::SafeLabelingProven { finding_id: "crates/core/tests/safe_case_labeled_loop.rs:6:5:TableLoop", behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty", "whitespace"] }), Ok(LoopRisk::Risky { finding_id: "tests/accepted_exception_loop.rs:7:5:TableLoop", reason: AcceptedExceptionRequired, required_action: AcceptedException }), Ok(LoopRisk::Risky { finding_id: "tests/helper_driven_table_cases.rs:4:30:HelperDrivenTableLoop", reason: MissingCaseIdentity, required_action: RepairRequired }), Ok(LoopRisk::Risky { finding_id: "tests/weak_iterator_for_each_missing_behavior.rs:3:31:IteratorTableLoop", reason: MissingBehaviorIdentity, required_action: RepairRequired }), Ok(LoopRisk::Risky { finding_id: "tests/weak_table_loop_missing_case_label.rs:7:5:TableLoop", reason: MissingCaseIdentity, required_action: RepairRequired })]`.
When: `assign_disposition` is invoked with `RepairEvidence { bead: "vb-repair-helper", owner: "Lewis" }`, `RepairEvidence { bead: "vb-repair-iterator", owner: "Lewis" }`, `RepairEvidence { bead: "vb-repair-weak-table", owner: "Lewis" }`, `ExceptionEvidence { reason: "bounded smoke loop", scope: "single deterministic fixture", owner: "Lewis", review_trigger: "mutation refresh" }`, and `SafeLabelEvidence { behavior: "parser rejects invalid ids", cases: ["empty", "whitespace"] }`; then `validate_inventory(inventory)` is invoked.
Then: validation returns exact `Ok(ValidatedInventory { risky_count: 4, repair_required_count: 3, accepted_exception_count: 1, safe_labeling_count: 1, finding_ids: ["crates/core/tests/safe_case_labeled_loop.rs:6:5:TableLoop", "tests/accepted_exception_loop.rs:7:5:TableLoop", "tests/helper_driven_table_cases.rs:4:30:HelperDrivenTableLoop", "tests/weak_iterator_for_each_missing_behavior.rs:3:31:IteratorTableLoop", "tests/weak_table_loop_missing_case_label.rs:7:5:TableLoop"] })`.
When: `render_inventory_report(validated_inventory)` is invoked.
Then: report finding IDs in order equal `["crates/core/tests/safe_case_labeled_loop.rs:6:5:TableLoop", "tests/accepted_exception_loop.rs:7:5:TableLoop", "tests/helper_driven_table_cases.rs:4:30:HelperDrivenTableLoop", "tests/weak_iterator_for_each_missing_behavior.rs:3:31:IteratorTableLoop", "tests/weak_table_loop_missing_case_label.rs:7:5:TableLoop"]` and report mutation fields equal `mutation_evidence: MutationEvidence::NotProvided` and `mutation_improvement_claim: None`.

## 4. Proptest Invariants

### Proptest: scope filtering
Invariant: any requested root outside normalized `tests/**` or `crates/**` is rejected before traversal with exact `InputRootOutOfScope`; vendor/generated candidate paths under otherwise valid roots are absent from the exact returned discovery set.
Strategy: generate relative and absolute paths including `..`, symlinks, Unicode segments, `tests/x.rs`, `crates/a/tests/x.rs`, `vendor/x.rs`, `target/x.rs`.
Anti-invariant: external, vendored, target, and generated paths without whitelist must never appear in discovered files.

### Proptest: scanner stable ordering
Invariant: candidate file map `{ "tests/b.rs": weak-loop-source, "tests/a.rs": safe-loop-source }` normalizes to output order `["tests/a.rs", "tests/b.rs"]` for all input permutations.
Strategy: generate small maps of path -> source fixture category and randomize input order.
Anti-invariant: same normalized files must not produce different report ordering.

### Proptest: unlabeled and ambiguous loops never become safe
Invariant: generated loop patterns with absent labels classify as exact `RepairRequired`; generated loop patterns with duplicated, unstable, or behavior-free labels return exact `AmbiguousCaseLabel`; none produce `SafeLabelingProven`.
Strategy: generate `LoopPattern` values with label states `Absent`, `Duplicate`, `CaseOnly`, `BehaviorOnly`, `Unstable`, `BehaviorAndCase`.
Anti-invariant: absent/ambiguous/partial labels must always fail safe-label classification.

### Proptest: safe labeling requires behavior and case evidence
Invariant: `SafeLabelingProven` is constructed exactly for behavior identity `parser rejects invalid ids` with case evidence `["empty", "whitespace"]`; missing behavior or empty case list returns exact `AmbiguousCaseLabel`.
Strategy: generate evidence records with optional behavior, optional case, duplicate labels, and complete labels.
Anti-invariant: missing behavior evidence returns exact `AmbiguousCaseLabel`; missing case evidence returns exact `AmbiguousCaseLabel`; neither returns safe proof.

### Proptest: disposition exactly one
Invariant: a risky finding validates exactly when disposition flags equal one of `[RepairRequired=true, AcceptedException=false, SafeLabelingProven=false]`, `[false, true, false]`, or `[false, false, true]`.
Strategy: generate triples of optional disposition evidence for lists of findings.
Anti-invariant: zero dispositions returns `UnassignedRiskyPattern`; two or three return `ConflictingDisposition`.

### Proptest: validation completeness
Invariant: inventory `[Finding { id: "tests/weak.rs:7:5:TableLoop", risk: Risky, disposition: None }]` returns exact `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/weak.rs:7:5:TableLoop" })`.
Strategy: generate inventories with mixed risky/non-risky findings and optional dispositions.
Anti-invariant: adding `Finding { id: "tests/safe.rs:4:1:TableLoop", risk: NonRisky, disposition: None }` to the unassigned risky inventory still returns exact `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/weak.rs:7:5:TableLoop" })`.

### Proptest: non-risky cannot suppress risky
Invariant: adding 0, 1, 2, or 16 non-risky findings to `Finding { id: "tests/weak.rs:7:5:TableLoop", risk: Risky, disposition: None }` returns exact `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/weak.rs:7:5:TableLoop" })`.
Strategy: generate one risky finding plus arbitrary non-risky findings, random ordering, duplicate-ish labels.
Anti-invariant: validation cannot return `Ok` until the risky finding has exactly one disposition.

### Proptest: evidence monotonicity
Invariant: generated evidence states map exactly as `Absent -> RepairRequired`, `Duplicate -> AmbiguousCaseLabel`, `BehaviorOnly -> AmbiguousCaseLabel`, `CaseOnly -> AmbiguousCaseLabel`, `BehaviorAndCase -> SafeLabelingProven`.
Strategy: generate chains over evidence lattice: `Absent < Ambiguous < BehaviorOnly/CaseOnly < BehaviorAndCase`.
Anti-invariant: `Absent` or `Ambiguous` cannot classify below `RepairRequired`.

### Proptest: deletion resistance
Invariant: baseline `["tests/deletion_baseline.rs:7:5:TableLoop"]` and current `[]` with repair evidence `None` returns exact `Err(InventoryError::DestructiveChangeDetected { path: "tests/deletion_baseline.rs", previous_finding: "tests/deletion_baseline.rs:7:5:TableLoop" })`.
Strategy: generate baseline/current finding sets with stable finding IDs and optional repair/removal evidence.
Anti-invariant: disappearance alone must never be treated as repaired or safe.

### Proptest: report completeness
Invariant: for canonical finding `tests/weak.rs:7:5:TableLoop`, rendered report contains exactly `[ReportFinding { path: "tests/weak.rs", location: "7:5", kind: TableLoop, risk_reason: MissingCaseIdentity, disposition: RepairRequired, owner: "Lewis", action: "vb-repair-1" }]`.
Strategy: generate validated inventories with one to twenty findings and legal dispositions.
Anti-invariant: omitting any required field or changing any field must fail report validation.

## 5. Fuzz Targets

### Fuzz Target: `scan_test_file` source text boundary
Input type: `SourceText` domain values: `Text(String)`, `ReadFailed { operation: "read_to_string" }`, and `InvalidUtf8 { byte_offset: 3 }`.
Risk: panic, invalid UTF-8 escape, parser recursion, OOM, false `SafeLabelingProven`, missed risky loops.
Corpus seeds: `weak_table_loop_missing_case_label.rs`, `safe_case_labeled_loop.rs`, `ambiguous_label_loop.rs`, `malformed_rust_unrecoverable.rs`, empty file, huge comment, nested macro-looking tokens, invalid UTF-8 bytes.
Required oracle: `SourceText::Text(weak_table_loop_missing_case_label_source)` returns exact `[LoopPattern { path: "tests/weak_table_loop_missing_case_label.rs", location: line 7 column 5, kind: TableLoop, assertion_count: 1 }]`; `SourceText::InvalidUtf8 { byte_offset: 3 }` returns exact `Err(InventoryError::InvalidUtf8 { path: "tests/invalid_utf8.rs", byte_offset: 3 })`; unrecoverable Rust syntax returns exact `Err(InventoryError::ParseFailed { path: "tests/malformed_rust_unrecoverable.rs", location: line 3 column 1 })`; untraceable generated input returns exact `Err(InventoryError::UnsupportedGeneratedSource { path_or_macro: "untraceable_generated_loop", reason: "no_stable_first_party_location" })`; absent labels return exact `Ok(LoopRisk::Risky { reason: MissingCaseIdentity, required_action: RepairRequired })`; ambiguous labels return exact `Err(InventoryError::AmbiguousCaseLabel { label: "invalid", behavior: Some("parser rejects invalid ids"), case_count: 2 })`.

### Fuzz Target: Rust-like macro/generated source mapping
Input type: UTF-8 strings with macro invocations, generated comments, `include!`, nested modules, and unstable spans.
Risk: untraceable generated source returns `SafeLabelingProven` instead of exact `UnsupportedGeneratedSource`, or returns `Ok([])` instead of exact `UnsupportedGeneratedSource`.
Corpus seeds: `traceable_macro_loop.rs`, `untraceable_generated_loop.rs`, macro invocation with repeated assert, generated file header, `include!(concat!(env!("OUT_DIR"), ...))`.
Required oracle: `traceable_macro_loop.rs` returns exact `[LoopPattern { path: "tests/traceable_macro_loop.rs", location: line 5 column 1, kind: TraceableMacroLoop, assertion_count: 1 }]`; untraceable sources return exact `Err(InventoryError::UnsupportedGeneratedSource { path_or_macro: "untraceable_generated_loop", reason: "no_stable_first_party_location" })`.

### Fuzz Target: label sufficiency extraction
Input type: arbitrary assertion message strings and structured label tokens.
Risk: hostile labels forge behavior/case identity, duplicate labels pass, empty labels accepted, Unicode confusables hide ambiguity.
Corpus seeds: empty label, duplicate `case=invalid`, `behavior=` only, `case=` only, passing label `behavior=parser rejects; case=empty`, whitespace-only, long Unicode, ANSI escape codes.
Required oracle: `behavior="parser rejects"` plus `case="empty"` passes sufficiency; `behavior=""` plus `case="empty"` returns exact `Err(InventoryError::AmbiguousCaseLabel { label: "empty", behavior: None, case_count: 1 })`; `behavior="parser rejects"` plus `case=""` returns exact `Err(InventoryError::AmbiguousCaseLabel { label: "parser rejects", behavior: Some("parser rejects"), case_count: 0 })`; duplicate `case="invalid"` returns exact `Err(InventoryError::AmbiguousCaseLabel { label: "invalid", behavior: Some("parser rejects invalid ids"), case_count: 2 })`.

### Fuzz Target: inventory report validation/rendering
Input type: generated in-memory public `ValidatedInventory` values with fields `risky_count`, `repair_required_count`, `accepted_exception_count`, `safe_labeling_count`, `finding_ids`, `mutation_evidence`, and `mutation_improvement_claim`.
Risk: omitted fields, mutation claims without evidence, malformed owner/action text, path confusion.
Corpus seeds: one repair finding, one accepted exception, one safe proof, mixed risky/non-risky, no findings, missing mutation evidence.
Required oracle: when mutation evidence is absent, report contains exact field `mutation_evidence: MutationEvidence::NotProvided` and exact field `mutation_improvement_claim: None`; when mutation evidence is present, report contains the exact supplied mutation evidence ID.

## 6. Kani Harnesses

### Kani Harness: path scope predicate rejects escapes
Property: all bounded symbolic paths of length <= 6 segments outside `tests/**` and `crates/**` are rejected before traversal.
Bound: max 6 path segments, max segment length 16 bytes, include `..`, `.`, absolute marker, `vendor`, `target`.
Rationale: path escape must be proven, not sampled.

### Kani Harness: exactly-one disposition
Property: for a symbolic finding with three boolean disposition flags, validation succeeds iff exactly one flag is true; false/false/false maps to `UnassignedRiskyPattern`; multiple true maps to `ConflictingDisposition`.
Bound: all 8 boolean combinations.
Rationale: core fail-closed algebra.

### Kani Harness: safe proof constructor requires behavior and case
Property: `SafeLabelingProven` is constructed/validated only when behavior evidence and case evidence booleans are both true.
Bound: all 4 evidence combinations plus stable/unstable flags.
Rationale: protects Lean-to-Rust refinement for POST-005 and INV-003.

### Kani Harness: ambiguous labels cannot be safe
Property: label states map exactly as `Absent -> RepairRequired`, `Duplicate -> AmbiguousCaseLabel`, `BehaviorOnly -> AmbiguousCaseLabel`, `CaseOnly -> AmbiguousCaseLabel`, and `Unstable -> AmbiguousCaseLabel`.
Bound: finite enum of label states and pattern kinds.
Rationale: critical ERR-006 contract.

### Kani Harness: quality gate fails closed
Property: `ValidatedInventory` is unreachable when any risky finding lacks a disposition.
Bound: inventories up to 5 findings, each risky/non-risky and disposition optional.
Rationale: POST-007 and INV-001 are release-blocking.

### Kani Harness: non-risky cannot suppress risky
Property: adding non-risky findings to an inventory does not change unassigned-risk failure into success.
Bound: one symbolic risky finding plus up to 4 symbolic non-risky findings.
Rationale: prevents aggregation bugs.

### Kani Harness: deletion is not repair
Property: baseline `["tests/deletion_baseline.rs:7:5:TableLoop"]`, current `[]`, and evidence `None` returns exact `Err(InventoryError::DestructiveChangeDetected { path: "tests/deletion_baseline.rs", previous_finding: "tests/deletion_baseline.rs:7:5:TableLoop" })`.
Bound: baseline/current sets up to 4 finding IDs.
Rationale: deletion-resistant quality gate must be formalized.

## 7. Mutation Testing Checkpoints

Threshold: >=90% overall kill rate via `cargo mutants`; 100% kill required for critical mutants below.

- Remove path-scope rejection -> caught by `discovery_returns_input_root_out_of_scope_when_scope_escapes_tests_or_crates` and path-scope proptest.
- Follow out-of-scope path before rejection -> caught by sentinel traversal assertion in out-of-scope test.
- Include `vendor`/`target` paths -> caught by `discovery_excludes_vendor_generated_and_external_paths_when_not_whitelisted`.
- Drop stable location from scanned pattern -> caught by `scanner_returns_location_kind_and_context_when_source_contains_test_loop_patterns` and report completeness proptest.
- Treat invalid UTF-8 as empty file -> caught by `scanner_returns_invalid_utf8_when_candidate_bytes_are_not_text` and fuzz oracle.
- Treat parse failure as no findings -> caught by `scanner_returns_parse_failed_when_rust_syntax_is_unrecoverable`.
- Remove helper-driven loop detection branch -> caught by `inventory_reports_helper_driven_pattern_when_test_helper_executes_table_cases`.
- Merge nested loop findings -> caught by `scanner_returns_distinct_findings_when_test_contains_nested_loops`.
- Change unlabeled loop from `RepairRequired` to safe/non-risky -> caught by unlabeled classifier BDD, proptest, Kani.
- Accept duplicated or behavior-free labels -> caught by ambiguous label BDD, proptest, fuzz, Kani.
- Allow `SafeLabelingProven` with missing behavior evidence -> caught by safe proof BDD and Kani.
- Allow `SafeLabelingProven` with missing case evidence -> caught by safe proof BDD and Kani.
- Allow accepted exception with missing reason/scope/owner/expiry -> caught by exception metadata BDD and mutation field omission checks.
- Change zero dispositions to success -> caught by unassigned risk BDD and exactly-one Kani.
- Change multiple dispositions to first-wins success -> caught by conflicting disposition BDD and exactly-one Kani.
- Let non-risky findings overwrite risky finding -> caught by mixed risky/non-risky BDD and proptest.
- Treat deleted test as improvement -> caught by deletion BDD, deletion proptest, deletion Kani.
- Omit owner/action from report -> caught by report completeness BDD/proptest.
- Add mutation-improvement claim string without evidence -> caught by no-mutation-claim BDD/static scan.
- Introduce `unwrap`, `expect`, `panic`, `unsafe`, unchecked indexing/casts/arithmetic -> caught by static policy gate.
- Add YAML/JSON/HTTP dependency to runtime core -> caught by runtime-core static scan.

Mutation run command: `moon run :verify-deep` or, if moon lane is not wired, `cargo mutants --package velvet_ballastics --in-place --timeout 300` plus explicit evidence file in `formal-verification-report.md`.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| bounded discovery happy path | workspace with `tests/weak_table_loop_missing_case_label.rs` and `crates/foo/tests/integration.rs` | exact `[TestFile { path: "crates/foo/tests/integration.rs" }, TestFile { path: "tests/weak_table_loop_missing_case_label.rs" }]` | integration |
| unreadable workspace | `/tmp/vb-5xs4-missing-root` | exact `Err(InventoryError::WorkspaceUnreadable { root: "/tmp/vb-5xs4-missing-root" })` | integration/manual |
| out-of-scope path | `../outside/sentinel_unreadable.rs` requested root | exact `Err(InventoryError::InputRootOutOfScope { path: "../outside/sentinel_unreadable.rs" })` | unit/proptest/Kani |
| vendor generated excluded | vendor/target/generated/external symlink | exact returned list `[TestFile { path: "crates/a/tests/real.rs" }]` | integration |
| weak table loop | `for case in cases` with unlabeled assertion | exact `Ok(LoopRisk::Risky { reason: MissingCaseIdentity, required_action: RepairRequired })` | unit/integration |
| iterator weak loop | `.iter().for_each` with unlabeled assertion | exact `Finding { id: "tests/weak_iterator_for_each_missing_behavior.rs:3:31:IteratorTableLoop", kind: IteratorTableLoop, reason: MissingBehaviorIdentity, action: RepairRequired }` | integration |
| helper-driven loop | test delegates repeated assertions to helper | exact `LoopPattern { path: "tests/helper_driven_table_cases.rs", location: line 4 column 30, kind: HelperDrivenTableLoop, assertion_count: 1 }` and `RepairRequired` classification | integration |
| nested loops | two nested repeated assertions | exact `[LoopPattern { path: "tests/nested_loops.rs", location: line 6 column 5, kind: NestedOuterLoop, assertion_count: 0 }, LoopPattern { path: "tests/nested_loops.rs", location: line 8 column 9, kind: NestedInnerLoop, assertion_count: 1 }]` | integration |
| safe labeled loop | behavior `parser rejects invalid ids`, cases `empty`, `whitespace` | exact `Disposition::SafeLabelingProven { behavior_evidence: "parser rejects invalid ids", case_evidence: ["empty", "whitespace"] }` | unit/Kani |
| ambiguous label | duplicate `case=invalid` labels | exact `Err(InventoryError::AmbiguousCaseLabel { label: "invalid", behavior: Some("parser rejects invalid ids"), case_count: 2 })` | unit/proptest/fuzz/Kani |
| accepted exception complete | reason/scope/owner/expiry present | exact `Disposition::AcceptedException { reason: "bounded smoke loop", scope: "single deterministic fixture", owner: "Lewis", review_trigger: "mutation refresh" }` | unit/integration |
| accepted exception missing owner | owner absent | exact `Err(InventoryError::PolicyViolation { rule: "accepted_exception_metadata_complete", field: "owner" })` | unit/mutation |
| unassigned risky | risky finding with no disposition | exact `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/weak.rs:7:5:TableLoop" })` | integration/Kani |
| conflicting dispositions | repair plus exception | exact `Err(InventoryError::ConflictingDisposition { finding_id: "tests/weak.rs:7:5:TableLoop", dispositions: [RepairRequired, AcceptedException] })` | unit/Kani |
| deleted test | baseline finding absent in current without evidence | exact `Err(InventoryError::DestructiveChangeDetected { path: "tests/deletion_baseline.rs", previous_finding: "tests/deletion_baseline.rs:7:5:TableLoop" })` | integration/manual/Kani |
| mixed risk | safe finding plus unassigned risky finding | exact `Err(InventoryError::UnassignedRiskyPattern { finding_id: "tests/mixed_risky_and_non_risky.rs:12:5:TableLoop" })` | integration/proptest |
| invalid UTF-8 | non-UTF-8 bytes | exact `Err(InventoryError::InvalidUtf8 { path: "tests/invalid_utf8.rs", byte_offset: 3 })` | fuzz/integration |
| parse failure | unrecoverable malformed Rust | exact `Err(InventoryError::ParseFailed { path: "tests/malformed_rust_unrecoverable.rs", location: line 3 column 1 })` | fuzz/integration |
| untraceable generated | macro/generated no stable location | exact `Err(InventoryError::UnsupportedGeneratedSource { path_or_macro: "untraceable_generated_loop", reason: "no_stable_first_party_location" })` | fuzz/integration |
| empty scope | no in-scope Rust tests | exact `InventoryReport { risky_count: 0, findings: [], mutation_evidence: MutationEvidence::NotProvided, mutation_improvement_claim: None }` | integration |
| deterministic repeat | same source and policy invoked twice | exact byte-identical `InventoryReport` debug serialization across both invocations | unit/proptest/e2e |
| report fields | validated risky inventory | exact `ReportFinding { path: "tests/weak.rs", location: "7:5", kind: TableLoop, risk_reason: MissingCaseIdentity, disposition: RepairRequired, owner: "Lewis", action: "vb-repair-1" }` | integration/proptest |
| no mutation claim | inventory report without mutation evidence | exact `mutation_evidence: MutationEvidence::NotProvided` and `mutation_improvement_claim: None` | static/integration |
| runtime dependency policy | clean policy evidence | exact `Ok(ValidatedInventory { policy_violation_count: 0, forbidden_dependency_count: 0, finding_ids: [] })` | static |
| forbidden constructs | policy evidence with `unwrap` violation | exact `Err(InventoryError::PolicyViolation { rule: "forbidden_construct", field: "unwrap@crates/quality/src/test_loop_inventory.rs:42" })` from `validate_inventory` | static/e2e |

## 9. Deletion-Resistant Test Requirements

- Fixture-backed tests must assert exact sentinel comments: `weak_table_loop_missing_case_label.rs` has `// vb-5xs4-fixture:weak_table_loop_missing_case_label`, `safe_case_labeled_loop.rs` has `// vb-5xs4-fixture:safe_case_labeled_loop`, and `ambiguous_label_loop.rs` has `// vb-5xs4-fixture:ambiguous_label_loop`.
- Fixture absence is exact `Err(InventoryError::FileReadFailed { path: "tests/fixtures/vb_5xs4/weak_table_loop_missing_case_label.rs", operation: "fixture_preflight" })` for the weak-table fixture, exact `Err(InventoryError::FileReadFailed { path: "tests/fixtures/vb_5xs4/safe_case_labeled_loop.rs", operation: "fixture_preflight" })` for the safe-label fixture, and exact `Err(InventoryError::FileReadFailed { path: "tests/fixtures/vb_5xs4/ambiguous_label_loop.rs", operation: "fixture_preflight" })` for the ambiguous-label fixture.
- Baseline/current deletion tests must use stable finding IDs derived from path + location + pattern kind, not list indexes.
- Reports must include removed/changed findings separately from repaired findings.
- A deleted weak loop may pass only when explicit evidence records `RepairRequired { bead: "vb-repair-deletion", owner: "Lewis" }`, `AcceptedException { reason: "intentional removal", scope: "tests/deletion_baseline.rs", owner: "Lewis", review_trigger: "mutation refresh" }`, or `SafeLabelingProven { behavior_evidence: "deleted weak loop replaced by labeled cases", case_evidence: ["replacement-case"] }`; otherwise exact `DestructiveChangeDetected` is mandatory.

## 10. Commands and Evidence Expectations

Red phase commands after tests are written:

- `moon run :verify-fast` — expected red until static gates and compile surface exist; expected failure evidence is exact missing API/tests or policy evidence.
- `moon run :verify-standard` — expected red until unit, integration, and proptest tests compile and fail against missing implementation.
- `moon run :verify-proof` — expected red until Lean/Kani harnesses are wired.
- `moon run :verify-deep` — expected red until fuzz, Miri/cargo-careful, mutation, and coverage evidence exist.
- If moon lanes are not wired, file a follow-up bead and run direct commands with the same required evidence targets: `cargo test`, `cargo nextest run`, `cargo llvm-cov nextest`, `cargo fuzz build`, `cargo mutants`, `cargo kani`, and `cargo +nightly miri test`.

Green phase acceptance commands before bead closure in later states:

- `moon run :verify-fast`
- `moon run :verify-standard`
- `moon run :verify-proof`
- `moon run :verify-deep`
- `moon run :verify-all`
- `cargo llvm-cov` configured coverage lane proving every `InventoryError` variant branch is exercised.
- `cargo mutants` evidence showing >=90% overall mutation kill and 100% kill for critical mutations listed above.

Evidence files expected:

- `formal-verification-report.md` with PRE/POST/INV/ERR proof obligation statuses.
- `coverage/llvm-cov-report.md` with branch/function coverage for error variants.
- Fuzz corpus and crash-free smoke evidence for scanner/label/report targets.
- Kani proof output for path scope, disposition completeness, safe-label proof, ambiguous labels, fail-closed validation, non-risk suppression, and deletion resistance.
- Mutation report identifying killed/survived mutants and explicit rationale for any surviving non-critical mutant.

## 11. Open Questions for Implementation Owner

1. Final report path/schema is still open in `contract.md`; tests should target public `InventoryReport` fields first and only add file-format assertions after schema is chosen.
2. Bead creation/update API for repair assignments is not fixed; tests should model repair assignment as public `AssignmentEvidence` with owner/action string until the authoritative bd API is selected.
3. Generated dependency code scope is open; this plan requires first-party traceable macro/generated source to be inventoried and untraceable generated source to return exact `UnsupportedGeneratedSource`.
4. Metadata omissions use exact `PolicyViolation` assertions in this plan; adding dedicated variants requires contract update and independent re-review before changing these tests.

## Exit Criteria Checklist

- [x] Every public API behavior has a BDD scenario.
- [x] Every `InventoryError` variant has an exact planned scenario.
- [x] Every pure multi-input behavior has a proptest invariant.
- [x] Parser/scanner/text/report boundaries have fuzz targets.
- [x] Critical state and predicate logic has Kani harnesses.
- [x] Mutation threshold >=90% is stated, with critical 100% kill requirements.
- [x] No planned assertion is merely `is_ok()` or `is_err()`; all assertions name exact values, fields, or variants.
