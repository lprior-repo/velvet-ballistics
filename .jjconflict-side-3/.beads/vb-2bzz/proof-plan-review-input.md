# Proof Plan Review Input — vb-2bzz

## Plan Summary

- **Bead**: `vb-2bzz`
- **Title**: storage: expose action ABI and policy digest recovery mismatch checks
- **Strategy**: Wire `verify_digests` → `check_action_abi_digests`/`check_policy_digests`, expose ABI digests through `recover_full_journal`, un-ignore GAP-3 BDD scenarios
- **Planned obligations**: 9 (5 unit-test, 2 Kani, 1 TLA+, 1 proptest)
- **Status**: planned (awaiting proof-reviewer approval)

## Planned Obligations

```jsonl
{"id":"PO-001","requirement_id":"EARS-1+EARS-2","contract_clause":"ActionAbiMismatch/PolicyDigestMismatch reachable through verify_digests at Full level","risk":"api-surface reachability","verifier":"unit-test","artifact":"crates/vb_storage/tests/recovery_bdd_tests.rs (or vb_runtime equivalent)","command":"cargo test -p vb_runtime --test recovery_bdd_tests -- verify_digests_full_level_checks_abis_and_policies","expected_evidence":"test passes; error variant is ActionAbiMismatch or PolicyDigestMismatch on mismatch","assumptions":["verify_digests implementation wires check_*_digests calls"],"required":true,"mode":"unit-test","owner_state":6,"rerun_from":6,"status":"planned"}
{"id":"PO-002","requirement_id":"EARS-3","contract_clause":"verify_digests Full level calls check_action_abi_digests and check_policy_digests","risk":"api-surface wiring","verifier":"unit-test","artifact":"crates/vb_storage/tests/recovery_bdd_tests.rs","command":"cargo test -p vb_runtime --test recovery_bdd_tests -- verify_digests_full_level_calls_checks","expected_evidence":"test passes; Full level checks both ABI and policy digests","assumptions":["verify_digests implementation extended per contract spec"],"required":true,"mode":"unit-test","owner_state":6,"rerun_from":6,"status":"planned"}
{"id":"PO-003","requirement_id":"INV-1+INV-2","contract_clause":"Empty input returns Ok without guessing","risk":"false-positive prevention","verifier":"unit-test","artifact":"crates/vb_storage/tests/recovery_bdd_tests.rs","command":"cargo test -p vb_runtime --test recovery_bdd_tests -- verify_digests_full_empty_inputs_ok","expected_evidence":"empty expected_abis and expected_policy_digests return Ok at Full level","assumptions":["verify_digests passes empty slices to check_*_digests"],"required":true,"mode":"unit-test","owner_state":6,"rerun_from":6,"status":"planned"}
{"id":"PO-004","requirement_id":"INV-4","contract_clause":"Error carries exact action_id/step","risk":"diagnostic accuracy","verifier":"unit-test","artifact":"crates/vb_storage/tests/recovery_bdd_tests.rs","command":"cargo test -p vb_runtime --test recovery_bdd_tests -- verify_digests_full_error_exact_fields","expected_evidence":"ActionAbiMismatch carries correct action_id; PolicyDigestMismatch carries correct step","assumptions":["check_*_digests propagate identifiers correctly"],"required":true,"mode":"unit-test","owner_state":6,"rerun_from":6,"status":"planned"}
{"id":"PO-005","requirement_id":"EARS-3 gap","contract_clause":"recover_full_journal passes ABI digests (not discarded)","risk":"parameter discarding","verifier":"unit-test","artifact":"crates/vb_storage/tests/recovery_bdd_tests.rs","command":"cargo test -p vb_runtime --test recovery_bdd_tests -- recover_full_journal_abis_used","expected_evidence":"ABI mismatch in expected_abis propagates through recover_full_journal as ActionAbiMismatch","assumptions":["replay_events updated to use ABI digest parameter"],"required":true,"mode":"unit-test","owner_state":6,"rerun_from":6,"status":"planned"}
{"id":"PO-006","requirement_id":"R1","contract_clause":"check_action_abi_digests and check_policy_digests panic-free","risk":"panic-freedom","verifier":"kani","artifact":"crates/vb_storage/src/kani_digest_checks.rs","command":"cargo kani --harness kani_check_action_abi_digests_panic_free && cargo kani --harness kani_check_policy_digests_panic_free","expected_evidence":"Kani proves no panic on arbitrary WorkflowDigest inputs","assumptions":["kani feature enabled for vb_storage"],"required":true,"mode":"verify-proof","owner_state":7,"rerun_from":7,"status":"planned"}
{"id":"PO-007","requirement_id":"R1","contract_clause":"verify_digests with Full level panic-free","risk":"panic-freedom new path","verifier":"kani","artifact":"crates/vb_storage/src/kani_digest_checks.rs","command":"cargo kani --harness kani_verify_digests_full_panic_free","expected_evidence":"Kani proves no panic for any digest combination at Full level","assumptions":["kani feature enabled; verify_digests wired per PO-001"],"required":true,"mode":"verify-proof","owner_state":7,"rerun_from":7,"status":"planned"}
{"id":"PO-008","requirement_id":"R3","contract_clause":"DigestCheck hierarchy invariant: Full ⊇ WorkflowAndIr ⊇ WorkflowSourceOnly","risk":"level hierarchy correctness","verifier":"tla-plus","artifact":".beads/vb-2bzz/specs/verify_digests_ordering.tla","command":"java -jar tla2tools.jar .beads/vb-2bzz/specs/verify_digests_ordering.tla -config .beads/vb-2bzz/specs/verify_digests_ordering.cfg","expected_evidence":"TLC model checker proves FullChecks ⊇ WorkflowAndIrChecks ⊇ WorkflowSourceOnlyChecks","assumptions":["model bounds: 3 levels, 3 digest sources"],"required":true,"mode":"verify-proof","owner_state":7,"rerun_from":7,"status":"planned"}
{"id":"PO-009","requirement_id":"INV-1+INV-2+INV-3","contract_clause":"Error taxonomy exhaustive: every input combo maps to exactly one outcome","risk":"taxonomic completeness","verifier":"proptest","artifact":"crates/vb_storage/src/recovery/tests.rs (or new proptest module)","command":"cargo test -p vb_storage --test recovery_bdd_tests --error_taxonomy_exhaustive --exact","expected_evidence":"All property-based tests pass; mutation testing shows expected kill rate ≥ 80%","assumptions":["proptest dependency available"],"required":true,"mode":"property-test","owner_state":5,"rerun_from":5,"status":"planned"}
```

## Waivers

```jsonl
{"id":"W-001","obligation":"Verus","reason":"No formal specification in contract; unit tests sufficient for typed error returns per contract clause","owner":"proof-planner","compensating":"Kani panic-freedom (PO-006, PO-007) + unit tests (PO-001-PO-005)","follow_up_trigger":"contract updated to require Verus verification"}
{"id":"W-002","obligation":"Flux","reason":"No refinement types or type-state constraints in scope","owner":"proof-planner","compensating":"Unit tests verify correct error variant selection","follow_up_trigger":"refined types introduced to RecoveryError"}
{"id":"W-003","obligation":"Loom","reason":"No concurrency in recovery functions; single-threaded execution","owner":"proof-planner","compensating":"No concurrent code to test","follow_up_trigger":"concurrency introduced to recovery module"}
{"id":"W-004","obligation":"Miri","reason":"#![forbid(unsafe_code)] in all recovery files; no UB risk","owner":"proof-planner","compensating":"Compiler-enforced safety via forbid(unsafe_code)","follow_up_trigger":"unsafe code introduced to recovery module"}
{"id":"W-005","obligation":"Fuzz","reason":"Functions are pure comparisons over small caller-provided slices; proptest (PO-009) covers equivalent space","owner":"proof-planner","compensating":"Property tests cover all combinatorial edge cases","follow_up_trigger":"functions accept untrusted external input"}
```

## Risk Summary

| Risk | Severity | Coverage |
|---|---|---|
| R1: Reachability gap | HIGH | PO-001, PO-006, PO-007 |
| R2: Parameter discarding | HIGH | PO-005 |
| R3: False sense of security | MEDIUM | PO-008 |
| R4: ABI digest not passed through replay | MEDIUM | PO-005 |

## Reviewer Checklist

- [ ] Verify that PO-001 through PO-005 unit-test plans cover the full integration path
- [ ] Verify that Kani harnesses (PO-006, PO-007) use `kani::any()` for digest generation (no hardcoded inputs)
- [ ] Verify that TLA+ model (PO-008) captures the `DigestCheck` hierarchy correctly
- [ ] Verify that proptest properties (PO-009) cover all combinatorial edge cases
- [ ] Verify that waivers (W-001 through W-005) are justified
- [ ] Confirm: no obligation claims PASS (planner does not execute proofs)
