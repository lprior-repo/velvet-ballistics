# Red Queen Report: vb-5xs4

STATUS: APPROVED

## Verdict

- Final crown state: CROWN DEFENDED.
- Red Queen state machine: `drq-session`, `spec_ref=.beads/vb-5xs4/contract.md`.
- Context: State 5 after latest Mode 2 approval and mutation repair.
- Generations executed: 3.
- Consecutive zero-survivor generations: 3.
- Survivors: 0.
- Red Queen bug beads filed: 0.
- Final `validate`: PASS, 4/4 lineage checks.

## Permanent Lineage Checks

All permanent `done_when` checks passed:

1. `cargo +nightly nextest run --test vb_5xs4_test_loop_inventory_red` — PASS, 78/78.
2. `cargo +nightly clippy --test vb_5xs4_test_loop_inventory_red -- -D warnings` — PASS, 0 errors.
3. Production fixture/path shortcut probe — PASS, no `vb_5xs4`, fixture, or specific test path shortcuts in `src/quality/test_loop_inventory.rs`.
4. Raw public field probe — PASS, no public `String` / `Vec<String>` / `Option<String>` fields on the checked domain/report surfaces.

`nu $L lineage-replay drq-session` defeated 4/4 predecessors.

## Fitness Landscape

| Dimension | Tests | Survivors | Fitness | Final status |
|---|---:|---:|---:|---|
| source-shortcuts | 1 | 0 | 0.0 | EXHAUSTED |
| typed-fields | 2 | 0 | 0.0 | EXHAUSTED |
| real-scan | 3 | 0 | 0.0 | EXHAUSTED |
| roots-scope | 5 | 0 | 0.0 | EXHAUSTED |
| accepted-exception | 5 | 0 | 0.0 | EXHAUSTED |
| case-evidence | 2 | 0 | 0.0 | EXHAUSTED |
| ordering | 3 | 0 | 0.0 | EXHAUSTED |
| exact-errors | 3 | 0 | 0.0 | DORMANT |
| mutation-resilience | 3 | 0 | 0.0 | COOLING |

Total attacks: 27. Total kills/survivors: 0.

## Adversarial Commands Run

Representative challenger commands all exited 0 and were recorded as discards:

- Real non-fixture Rust loop scanning:
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red scan_returns_table_loop_pattern_when_for_loop_contains_unlabeled_assertion`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red scan_returns_iterator_loop_pattern_when_for_each_contains_assertion`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red scan_returns_two_patterns_when_source_contains_nested_loops`
- `InventoryScope::Roots` roots/excludes:
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red mutant_kills_allowed_tests_and_crates_roots`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red mutant_kills_vendor_root_exclusion`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red mutant_kills_target_root_exclusion`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red mutant_kills_generated_root_exclusion`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red mutant_kills_external_root_exclusion`
- Exact `InventoryError` variants:
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red scan_returns_invalid_utf8_when_source_text_is_invalid`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red scan_returns_parse_failed_when_source_is_unrecoverable_rust`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red validate_returns_conflicting_disposition_when_repair_and_exception_are_both_present`
- Accepted-exception metadata validation:
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red assign_returns_accepted_exception_when_exception_metadata_is_complete`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red assign_returns_policy_violation_when_exception_owner_is_missing`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red assign_returns_policy_violation_when_exception_reason_is_missing`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red assign_returns_policy_violation_when_exception_scope_is_missing`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red assign_returns_policy_violation_when_exception_review_trigger_is_missing`
- Empty case evidence rejection:
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red classify_rejects_safe_labeling_when_case_evidence_is_empty`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red assign_returns_ambiguous_case_label_when_safe_proof_case_evidence_is_missing`
- Typed domain / no raw public fields:
  - Static no-raw-field lineage check above.
  - `rg -n 'pub struct (DomainPath|FindingId|ReportLocation|OwnerName|ReportAction|ExceptionReason|ExceptionScope|BehaviorEvidence|CaseLabel|CaseEvidence|MutationImprovementClaim)' src/quality/test_loop_inventory.rs`
- Fixture/path shortcut absence:
  - Static shortcut lineage check above.
- Ordering resilience:
  - `cargo +nightly nextest run --test vb_5xs4_test_loop_inventory_red` — 78/78 PASS.
  - `NEXTEST_TEST_THREADS=1 cargo +nightly nextest run --test vb_5xs4_test_loop_inventory_red` — 78/78 PASS.
  - `NEXTEST_TEST_THREADS=8 cargo +nightly nextest run --test vb_5xs4_test_loop_inventory_red` — 78/78 PASS.
- Mutation-resilience targeted probes:
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red mutant_kills_exception_metadata_validation_deleted`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red mutant_kills_case_evidence_validation_deleted`
  - `cargo +nightly test -q --test vb_5xs4_test_loop_inventory_red mutant_kills_symbolic_safe_label_rejects_empty_case_evidence_guard`

## Additional Gate Evidence

- Coverage rerun: `cargo +nightly llvm-cov --test vb_5xs4_test_loop_inventory_red --fail-under-lines 95` — PASS.
  - `src/quality/test_loop_inventory.rs`: 95.88% line coverage.
- Final deterministic verdict command: `nu $L verdict drq-session` — CROWN DEFENDED.
- Final deterministic validation command: `nu $L validate drq-session` — PASS, 4/4.
- Final lineage replay: 4/4 predecessors defeated.

## Survivor Summary

No survivors found. No Red Queen bug beads were filed.

## Blockers

None for bead-owned Red Queen scope. Broad unrelated repo-wide debt was not treated as a blocker.
