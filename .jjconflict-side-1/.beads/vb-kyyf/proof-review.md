# vb-kyyf Proof Review — shared macro cfg consolidation attempt 4

## Findings

No rejection findings.

## Reviewed obligations

| Obligation | Review result | Evidence |
|---|---|---|
| PO-009 / VERUS-KYYF-001 | APPROVED | Verus verified the production-probe branch with `42 verified, 0 errors`; cargo branch now consumes the same shared macro decision bodies for digest match, normalization projection, replay taxonomy, generated/IR taxonomy, normalized equality, terminal-result equality, taint equality, and normalized comparison. |

## Raw evidence checked

```text
$ pwd -P
/home/lewis/src/bd-vb-kyyf-bdd

$ verus verification/verus/vb_kyyf_normalization.rs
verification results:: 42 verified, 0 errors

$ rtk cargo test -p vb_proof_kernels vb_kyyf_normalization --all-features
cargo test: 3 passed, 34 filtered out (1 suite, 0.00s)

$ rtk grep -n 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom|admit|sorry|unimplemented|todo' verification/verus/vb_kyyf_normalization.rs crates/vb_proof_kernels/src/vb_kyyf_normalization.rs --glob '*.rs'
0 matches for 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom|admit|sorry|unimplemented|todo'

$ macro/cfg seam inspection
crates/vb_proof_kernels/src/vb_kyyf_normalization.rs:9   macro_rules! digest_all_match_body
crates/vb_proof_kernels/src/vb_kyyf_normalization.rs:19  macro_rules! normalize_observation_body
crates/vb_proof_kernels/src/vb_kyyf_normalization.rs:38  macro_rules! normalized_observations_equal_body
crates/vb_proof_kernels/src/vb_kyyf_normalization.rs:59  macro_rules! terminal_results_equal_body
crates/vb_proof_kernels/src/vb_kyyf_normalization.rs:72  macro_rules! taint_statuses_equal_body
crates/vb_proof_kernels/src/vb_kyyf_normalization.rs:84  macro_rules! compare_replay_body
crates/vb_proof_kernels/src/vb_kyyf_normalization.rs:102 macro_rules! compare_generated_ir_body
crates/vb_proof_kernels/src/vb_kyyf_normalization.rs:118 macro_rules! compare_normalized_observations_body
Verus branch macro uses: lines 181, 221, 332, 342, 356, 366, 373, 380.
Cargo branch macro uses: lines 423, 463, 477, 484, 491, 498, 502, 506.
```

## Decision

The previous blocker was hand-maintained cfg-branch executable duplication. That blocker is closed: both cfg branches route their cargo-observable decision algebra through shared macro bodies. The remaining cfg split is type/signature/contract scaffolding needed because Cargo cannot parse Verus contracts. The Verus branch binds the shared bodies to executable postconditions, and the Cargo branch compiles/tests the same macro bodies, so PO-009 is mechanically bound enough under GOD RULE 2 for this sublane.

STATUS: APPROVED
