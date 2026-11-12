# Proof Repair Guide

## Required Repairs

1. Execute the TLA+ recovery model.

Provide `tla2tools.jar` in the isolated workspace or replace the command with an equivalent checked TLC runner. Rerun `RecoveryCrashRestart.tla` with `RecoveryCrashRestart.cfg` and record raw output proving no invariant violations, no unexpected deadlock, and active checking of `EventuallyRecoveredOrRejected`.

2. Repair the canonical proof gate.

Fix or correctly invoke `scripts/rust-verification-gauntlet.sh` so `moon run :verify-proof` reaches the scoped proof artifacts. State 6 cannot approve while the canonical gate fails on script parsing before proof execution.

3. Resolve `PO-003`.

`KANI-REC-001` is currently required, planned, and unwaived. Either add/run the bounded Kani harness evidence under the canonical proof gate or rewrite the planned obligation as an explicit approved lifecycle deferral/waiver with owner, expiry, rationale, and compensating Verus evidence.

4. Preserve current Verus evidence.

Keep `verification/verus/recovery_hydration_contracts.rs` and `verification/verus/recovery_production_mapping.md` mapped to `VERUS-REC-001`, and rerun `verus verification/verus/recovery_hydration_contracts.rs` after any proof edits. The current local result is `10 verified, 0 errors`.

## Rerun Targets

- `pwd -P`
- `jq -c . .beads/vb-qi37.1.6/proof-obligations.jsonl`
- `jq -c . .beads/vb-qi37.1.6/proof-obligations.planned.jsonl`
- `jq -c . .beads/vb-qi37.1.6/traceability-matrix.jsonl`
- `verus verification/verus/recovery_hydration_contracts.rs`
- `java -jar tla2tools.jar verification/tla/RecoveryCrashRestart.tla -config verification/tla/RecoveryCrashRestart.cfg`
- `moon run :verify-proof`

## Approval Criteria

- `proof-review.md` can say approved only after all required proof obligations due for State 6 have raw PASS evidence or explicit valid waiver/defer records.
- `TLA-REC-001` must have raw TLC or equivalent model-checker evidence, not just repaired artifacts.
- `GATE-REC-001` must no longer fail before reaching proof artifacts.
- Required planned rows must not remain `status: planned` with `waiver: null` when used as State 6 proof evidence.
