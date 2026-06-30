# Proof-to-Implementation Input — vb-rpch TLC Fix Pass

Do not bridge any TLA claim as proven until `TLC-FIX-001` through `TLC-FIX-008` are executed and proof-reviewer accepts the evidence.

| TLA claim | Rust/behavior surface for later bridge | Current bridge status |
|---|---|---|
| `ReplaySeqOrder` | `crates/vb_storage/src/recovery/replay/core.rs`; sequence-gap and ordering BDD scenarios | blocked pending fresh TLC + non-vacuity |
| `TailCausalAfterSnapshot` | `recover_snapshot_plus_tail`, `hydrate_run_frame` snapshot/tail preconditions | blocked by `SetSnapshot` repair |
| `OnlyIncompleteRuns` | `recover_all_incomplete_runs` latest-attempt terminal filtering | blocked pending reachability evidence |
| `NoResolvedReExecution` | `ActionReplayTracker`, non-idempotent replay blocking | blocked pending reachability evidence |
| `RecoveryErrorExhaustive` | `RecoveryError` variants and recovery error paths | not proven; requires per-error reachability or downgrade |
| `DigestVerificationOrder` | `verify_digests`, `check_workflow_source_digest`, `check_compiled_ir_digest` | blocked pending non-vacuity/order semantics |
