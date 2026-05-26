# Proof-to-Implementation Input — TLC Fix Round 2 for `vb-rpch`

Status: blocked pending proof-writer edits, formal-verifier execution, and proof-reviewer acceptance. Do not bridge any claim as proven yet.

## Planned proof claims to bridge later

| Planned TLA claim | Required TLA repair/evidence before bridge | Rust/behavior surface for bridge |
|---|---|---|
| `ReplaySeqOrder` | New primary cfg completes; independent replay-order antecedent witness fires. | `crates/vb_storage/src/recovery/replay/core.rs`; sequence ordering and BDD B-008/B-013. |
| `TailCausalAfterSnapshot` | New primary cfg completes; independent snapshot-tail witness fires; `SetSnapshot` remains well-typed. | `recover_snapshot_plus_tail`, `hydrate_run_frame`; PRE-001/PRE-005. |
| `OnlyIncompleteRuns` | New primary cfg completes; separate recovered-runs and terminal-excluded witnesses fire. | `recover_all_incomplete_runs`; INV-006/POST-008 tests. |
| `NoResolvedReExecution` | New primary cfg completes; witness reaches resolved action plus same-action replay candidate blocked or `NonIdempotentActionBlocked`. | `ActionReplayTracker`, `replay_events`; POST-009/POST-010 and BDD B-007. |
| `DigestVerificationOrder` | TLA model has `digest_stage`; invariant is `IrChecked => WorkflowChecked`; witness reaches IR after workflow and mismatch paths. | `verify_digests`, `check_workflow_source_digest`, `check_compiled_ir_digest`; POST-001..POST-003 and INV-005. |
| `RecoveryErrorExhaustive` for current TLA `ErrorDomain` | Causal transitions exist and per-error witness cfgs fire for all non-`None` current TLA variants. | `RecoveryError` taxonomy in `crates/vb_storage/src/recovery/types.rs`; error BDDs B-001b/B-001c/B-004/B-005/B-007/B-008/B-011. |

## Required implementation caveats for bridge

- The new primary cfg is intentionally bounded to singleton domains and `MAX_EVENTS=3`. Bridge may cite it only as finite-state evidence for that abstraction.
- `ActionAbiMismatch` and `PolicyDigestMismatch` TLA witnesses, if added, demonstrate typed error-domain reachability in an abstract model only. Contract.md still marks GAP-3 runtime lookup as deferred.
- If TLA `ErrorDomain` omits Rust `Journal` or `TerminalStateMismatch`, bridge must not claim those variants are covered by the TLA exhaustiveness evidence.
- Snapshot corruption in TLA is an abstract marker/mismatch and must not be bridged as byte-level decoder proof.
- Digest values are symbolic; TLA proves check ordering and mismatch branching, not cryptographic digest computation.

## Exact planned commands bridge should expect in evidence

```bash
tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-nonvacuity-replay-order.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-replay-order.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-nonvacuity-snapshot-tail.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-snapshot-tail.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-nonvacuity-incomplete-runs.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-incomplete-runs.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-nonvacuity-terminal-excluded.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-terminal-excluded.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-nonvacuity-no-reexecution.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-nonvacuity-digest-order.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-digest-order.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-error-workflow-source-digest-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-workflow-source-digest-mismatch.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-error-compiled-ir-digest-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-compiled-ir-digest-mismatch.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-error-no-recovery-data.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-no-recovery-data.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-error-corrupt-snapshot.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-corrupt-snapshot.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-error-action-abi-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-action-abi-mismatch.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-error-policy-digest-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-policy-digest-mismatch.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-error-non-idempotent-action-blocked.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-non-idempotent-action-blocked.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-error-replay-divergence.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-replay-divergence.tlc.log 2>&1
tlc -config specs/tla/RecoveryReplayFull-error-frame-dimension-overflow.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-frame-dimension-overflow.tlc.log 2>&1
cmp -s specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && cmp -s specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg && cmp -s specs/tla/RecoveryReplayFull-large-stress.cfg evidence/specs/RecoveryReplayFull-large-stress.cfg
```
