# Proof Review

STATUS: REJECTED

## Findings

- CRITICAL `TLA-REC-001` / `PO-001` / `PO-015`: Required TLA+ recovery proof remains unexecuted. Reviewer reran `java -jar tla2tools.jar verification/tla/RecoveryCrashRestart.tla -config verification/tla/RecoveryCrashRestart.cfg`; it failed with `Error: Unable to access jarfile tla2tools.jar`. The repaired config now names `PROPERTY EventuallyRecoveredOrRejected`, but no TLC output proves invariants, deadlock behavior, or liveness.
- CRITICAL `GATE-REC-001` / `PO-009`: Required canonical proof gate remains blocked. Reviewer reran `moon run :verify-proof`; it exited 2 before reaching proof artifacts because `scripts/rust-verification-gauntlet.sh` is parsed as shell and fails on Rust doc-comment lines. This row's own waiver expires before State 6 approval, so approval is forbidden.
- HIGH `KANI-REC-001` / `PO-003`: Planned proof obligations mark the Kani bounded state/error-classification lane as `required: true`, `status: planned`, `waiver: null`, but State 5 evidence provides no harness artifact, execution output, or approved waiver. If this lane is intentionally deferred to State 7 or replaced by Verus, the obligation record must encode that explicitly rather than remaining an unexecuted required proof row.
- MEDIUM `VERUS-REC-001` / `PO-002`: Direct Verus execution passes locally with `verification results:: 10 verified, 0 errors`, and `verification/verus/recovery_production_mapping.md` improves production-shape mapping. This is acceptable supporting evidence, but it cannot compensate for unexecuted TLA/canonical proof obligations or unexecuted required Kani planning rows.

## Evidence Reviewed

- Verified isolated workspace with `pwd -P`: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6`.
- Validated required JSONL artifacts with `jq -c .`: `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, and `traceability-matrix.jsonl`.
- Read `contract.md`, `traceability-matrix.jsonl`, `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `proof-strategy.md`, `proof-writer-report.md`, `proof-evidence.md`, `verification/tla/RecoveryCrashRestart.tla`, `verification/tla/RecoveryCrashRestart.cfg`, `verification/verus/recovery_hydration_contracts.rs`, and `verification/verus/recovery_production_mapping.md`.
- Ran discovery over proof artifacts for assumptions, trusted boundaries, invariants, properties, verifier markers, and evidence status claims.
- Reran `verus verification/verus/recovery_hydration_contracts.rs`: exit 0, `10 verified, 0 errors`, 11 deprecation warnings.
- Reran `java -jar tla2tools.jar verification/tla/RecoveryCrashRestart.tla -config verification/tla/RecoveryCrashRestart.cfg`: exit 1, missing jar.
- Reran `moon run :verify-proof`: exit 2, gauntlet shell parse failure.

## Decision

Rejected. Required proof obligations are still unexecuted or blocked, and the canonical proof gate still cannot reach the repaired artifacts. Local Verus success is not enough to approve State 6.
