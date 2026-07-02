# Proof Plan Review Input: vb-f04l State 4 Attempt 3

## Decision Request

Review the refreshed proof plan after repaired State 3. Previous State 4/5/6 outputs are invalidated where obligations changed or proof adequacy was rejected.

## Inputs Read

- Repaired State 3: `contract.md`, `traceability-matrix.jsonl`, `delivery-scope.jsonl`, `proof-obligations.jsonl`, `tla-spec.md`, `verification-layers.md`.
- State 6 rejection: `proof-review.md`, `proof-findings.jsonl`, `proof-repair-guide.md`, `contract-verification-review.md`.
- Prior proof evidence context only: `proof-evidence.md`, `proof-writer-report.md`.

## Coverage Shape

- Planned rows cover PRE-001..PRE-007, POST-001..POST-014, INV-001..INV-010, and ERR-001..ERR-011.
- TLA+ rows cover ForEach, Together, Collect, Reduce, Repeat, Wait, Ask, and aggregate lifecycle safety.
- Verus rows cover checked bounds, dense targets, slot coverage, determinism, and primitive shape preservation for POST-006..POST-012.
- Skipped verifier families are explicit obligation rows, not silent omissions.

## Discovery Summary

- Path and artifact gates passed.
- Scoped risk scan found state/transition/retry/serialization markers and test-only panic/expect/assert hits.
- Scoped verifier scan found current Verus proof markers; this confirms the old proof surface exists but does not approve it.
- No discovery command was blocked.

## Reviewer Bar

- Reject if any row lacks canonical mapping, executable command or explicit waiver, owner state, rerun point, or expected evidence.
- Reject if the plan treats prior State 5 verifier success as current accepted proof.
- Reject if TLA+ or Verus can pass by assuming the exact graph-shape property under review.

---

# State 4 Repair Review Input After State 11 Rejection

## Decision Request

Review only the command-parity repair. State 11 proved the prior cargo-test obligation filters were stale because they matched zero tests while exiting 0. The repaired plan now points cargo-test obligations at real tests in `crates/vb_compile/tests/v1_primitive_lowering.rs` and keeps all non-cargo proof lanes unchanged.

## Changed Artifacts

- `.beads/vb-f04l/proof-obligations.jsonl`: repaired command/evidence/expected_evidence fields for stale cargo-test rows.
- `.beads/vb-f04l/proof-obligations.planned.jsonl`: same repaired command/evidence mapping for State 4 planner output.
- `.beads/vb-f04l/proof-strategy.md`: records the State 11 rejection and the repaired mapping.
- `.beads/vb-f04l/STATE.md`: appended State 4 repair transition evidence.

## Reviewer Bar For This Repair

- Reject if any repaired cargo-test command can still select zero tests while returning success.
- Reject if any repaired command points outside the isolated workspace target `crates/vb_compile/tests/v1_primitive_lowering.rs` without an explicit reason.
- Reject if this repair edits source, tests, proof harnesses, dependencies, or CI config; it must remain planning-artifact-only.
- Reject if JSONL validation fails for either proof obligation file.

## Selection Evidence Captured

- `compile_source_returns_exact_error_variants_for_contract_taxonomy`: `1 passed, 14 filtered out`.
- `yaml_compiler_compile_emits_supported_ir_when_each_scoped_primitive_is_valid`: `1 passed, 14 filtered out`.
- `compile_workflow_returns_step_field_shape_when_each_scoped_primitive_required_field_is_empty`: `1 passed, 14 filtered out`.
- `public_helpers_return_exact_step_index_slot_index_limit_and_workflow_error_variants`: `1 passed, 14 filtered out`.
- `public_compile_apis_preserve_set_and_terminal_finish_regression`: `1 passed, 14 filtered out`.
- `public_lowering_helpers_return_exact_range_and_workflow_errors`: `1 passed, 14 filtered out`.
- `yaml_compiler_compile_returns_canonical_yaml_when_source_parse_fails`: `1 passed, 14 filtered out`.
- Full integration target `cargo test -p vb_compile --test v1_primitive_lowering`: `15 passed`.
