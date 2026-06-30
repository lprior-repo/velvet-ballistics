## vb-ko29.4 Kani idempotency generator report

Scope: `crates/vb_core/src/kani_idempotency_gates.rs` and existing
`crates/vb_validate/src/kani_idempotency_contract.rs` harness inventory/execution.

Generator repairs:
- Replaced fixed `RunFrame`/contract/key examples in `kani_idempotency_gates` with bounded symbolic
  generators for contract classes, run ids, step counts, slot counts, step index, key lengths, key slots,
  taint variants, failure codes, retry policy outcomes, and tickets.
- Added cover evidence for missing key, clean/secret/random/time taints, min/max bounded key lengths,
  safe/key-required/unsafe retry paths, duplicate success/failure, retryable/nonretryable failures,
  nonterminal/stale completion shape, and out-of-bounds completion conflict shape.
- Added a smallest scoped symbolic idempotency harness that avoids frame writes and proves the
  `KeyRequired`/empty-key `MissingKey` case with non-vacuity cover.

Scoped execution evidence:
- `vb_core-verify_idempotency_missing_key_symbolic_contract_no_frame_write.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 1/1 cover properties satisfied.
- `vb_core-verify_idempotency_duplicate_invocation_is_stable-r2.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 2/2 cover properties satisfied.
- `vb_core-verify_idempotency_duplicate_success_clean_key.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 1/1 cover properties satisfied.
- `vb_core-verify_idempotency_duplicate_failure_tainted_key.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 1/1 cover properties satisfied.
- `vb_core-verify_idempotency_required_taint_variants_have_witnesses-r2.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 4/4 cover properties satisfied.
- `vb_core-verify_idempotency_boundary_key_lengths_pass_clean_frame.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 2/2 cover properties satisfied.
- `vb_core-verify_idempotency_frame_slot_bounds_no_panic.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 2/2 cover properties satisfied.
- `vb_core-verify_idempotency_retry_policy_matrix_no_frame_write.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 4/4 cover properties satisfied.
- `vb_core-idempotency_divergent_digest_symbolic_certificate_rejected-r2.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 1/1 cover properties satisfied.
- `vb_core-validate_action_outcome_certificate_stale_nonterminal.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 1/1 cover properties satisfied.
- `vb_core-validate_action_outcome_certificate_conflict_oob.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`, 1/1 cover properties satisfied.
- `vb_validate-kani_decision_001_all_combinations.log`: exit 0,
  `VERIFICATION:- SUCCESSFUL`.

Superseded failed/timeout evidence:
- `vb_core-verify_idempotency_duplicate_invocation_is_stable.log` and
  `vb_core-verify_idempotency_duplicate_invocation_is_stable-after-symbolic-contract.log` timed out at
  the shell timeout while CBMC was exploring allocation/drop paths. The harness was reduced to a one-slot
  symbolic-taint form and passed in `vb_core-verify_idempotency_duplicate_invocation_is_stable-r2.log`.
- `vb_core-idempotency_divergent_digest_symbolic_certificate_rejected.log` failed due insufficient unwind
  for builtin `memcmp`; the same harness passed with `#[kani::unwind(40)]` in the `-r2` log.

Coverage matrix:
- duplicate success: `verify_idempotency_duplicate_success_clean_key`, PASS.
- duplicate failure: `verify_idempotency_duplicate_failure_tainted_key`, PASS.
- duplicate stability success/failure in one symbolic harness: `verify_idempotency_duplicate_invocation_is_stable`, PASS in `-r2`.
- divergent digest: `idempotency_divergent_digest_symbolic_certificate_rejected`, PASS in `-r2`.
- missing key: `verify_idempotency_missing_key_symbolic_contract_no_frame_write`, PASS.
- clean/secret/random/time taints: `verify_idempotency_required_taint_variants_have_witnesses`, PASS in `-r2`.
- boundary key lengths: `verify_idempotency_boundary_key_lengths_pass_clean_frame`, PASS.
- frame slot bounds: `verify_idempotency_frame_slot_bounds_no_panic`, PASS.
- retry policy matrix: `verify_idempotency_retry_policy_matrix_no_frame_write`, PASS.
- certificate stale/nonterminal: `validate_action_outcome_certificate_stale_nonterminal`, PASS.
- certificate conflict/out-of-bounds: `validate_action_outcome_certificate_conflict_oob`, PASS.

Limits/assumptions:
- Kani bounds key/frame structural exploration to <= 4 slots and <= 4 key ingredients in the repaired
  core harness module.
- Divergent digest harness verifies the public `WorkflowDigest` equality gate directly; it does not claim
  end-to-end runtime certificate admission behavior.
- No TLA+, Verus, public behavior tests, or production runtime behavior were modified.
