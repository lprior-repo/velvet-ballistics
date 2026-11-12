# Proof Evidence

## Summary

- `VERUS-REC-001` / `PO-002`: `PASS_LOCAL` via direct Verus execution after State 5 attempt 2 repair.
- `TLA-REC-001` / `PO-001`: `BLOCKED_TOOLING` because `tla2tools.jar` is absent. The model/config were repaired, but TLC did not execute.
- `GATE-REC-001` / `PO-009`: `BLOCKED_TOOLING` because local `moon run :verify-proof` fails before reaching scoped proof artifacts.

## Workspace Isolation

### `pwd -P`

exit=0

```text
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6
```

The physical path is not `/home/lewis/src/velvet-ballistics` and is not nested under it.

## Tool Discovery

### `which java || true`

exit=0

```text
/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java
```

### `which verus || true`

exit=0

```text
/home/lewis/.local/bin/verus
```

### `cargo kani --version`

exit=0

```text
cargo-kani 0.67.0
```

### `cargo flux --version`

exit=101

```text
error: no such command: `flux`

help: a command with a similar name exists: `fix`

help: view all installed commands with `cargo --list`
help: find a package to install `flux` with `cargo search cargo-flux`
```

### `cargo +nightly miri --version`

exit=0

```text
miri 0.1.0 (e0e95a7187 2026-04-04)
```

### `cargo fuzz --version`

exit=0

```text
cargo-fuzz 0.13.1
```

## Verus Evidence

### Command

`verus verification/verus/recovery_hydration_contracts.rs`

exit=0

### Output Summary

```text
verification results:: 10 verified, 0 errors
warning: 11 warnings emitted
```

Warnings were deprecation warnings for `ResultAdditionalSpecFns::{is_Ok,is_Err,get_Ok_0}` helper syntax. They did not block verification.

### Discharged Claims

- `proof_success_has_complete_durable_facts`: success implies header, required slot, taint, snapshot validity, order, tail watermark, both digest checks, collect extra, runtime-boundary support, no pending action, no fact erasure, and bounded dimensions.
- `proof_success_has_exact_taint`: success preserves recovered secret taint exactly and rejects missing required secret recovery.
- `proof_missing_secret_taint_fails_closed`: required secret with missing recovered secret cannot succeed.
- `proof_dimension_overflow_fails_closed`: dimension overflow cannot succeed.
- `proof_pending_action_fails_closed`: pending action cannot produce runnable success.
- `proof_runtime_boundary_unsupported_fails_closed`: unsupported runtime-boundary hydration cannot produce success; this discharges the `PRE-006` no-partial-success branch in the Verus abstraction.
- `proof_digest_mismatch_fails_closed`: workflow source or compiled IR digest mismatch cannot produce success.
- `proof_monotonic_fact_erasure_fails_closed`: fact erasure cannot produce runnable success.
- `proof_typed_error_totality`: recovery decision is total over success or typed error.

### Production Mapping Artifact

`verification/verus/recovery_production_mapping.md` maps:

- `SpecRecoveryInput` fields to `RecoveryRuntimeSummary`, `RecoveryFrameSeed`, `RecoveryHydration`, `RecoveredSlotEntry`, `RunSnapshot`, `CollectStates`, and runtime-boundary support.
- `SpecRecoverySuccess` to complete runnable or summary-complete recovery products.
- `SpecRecoveryError` variants to `RecoveryError::{NoRecoveryData, CorruptSnapshot, ReplayDivergence, WorkflowSourceDigestMismatch, CompiledIrDigestMismatch, NonIdempotentActionBlocked, FrameDimensionOverflow}`, `RuntimeError::InvalidRecoveryHydration`, and `EngineError::CollectExtraHydrationFailed`.

## TLA+ Evidence

### Repaired Artifacts

- `verification/tla/RecoveryCrashRestart.tla`: added weak fairness on `Crash` and on the recovery/rejection decision action set.
- `verification/tla/RecoveryCrashRestart.cfg`: added `PROPERTY EventuallyRecoveredOrRejected`.

### Command

`java -jar tla2tools.jar verification/tla/RecoveryCrashRestart.tla -config verification/tla/RecoveryCrashRestart.cfg`

exit=1

### Output

```text
Error: Unable to access jarfile tla2tools.jar
```

### Status

`BLOCKED_TOOLING`: Java is available, but the TLC jar named by the proof-writer workflow is absent from this isolated workspace. The repaired model was not model-checked. No PASS is claimed.

## Canonical Gate Evidence

### Command

`moon run :verify-proof`

exit=2

### Output Summary

```text
scripts/rust-verification-gauntlet.sh: line 3: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 4: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 5: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 6: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 7: syntax error near unexpected token `newline'
scripts/rust-verification-gauntlet.sh: line 7: `//! Usage: scripts/rust-verification-gauntlet.sh <mode>'
```

### Status

`BLOCKED_TOOLING`: the canonical gate could not reach the authored artifacts because the configured shell command fails while parsing its own script header.

## Artifact Assumptions

- `RecoveryCrashRestart.tla` finite domains: two attempts, sequence/watermark range `0..4`, boolean durable facts, action states `none|pending|resolved`, collect states `none|valid|corrupt|wrong_identity`.
- TLA+ rejects pending actions, corrupt collect extra, wrong collect identity, missing header/slot, unordered/gapped journal, invalid snapshot, lifecycle-only diagnostics, and tail-before-watermark by typed terminal rejection.
- Verus abstracts production recovery as `SpecRecoveryInput` and `recovery_decision`; `recovery_production_mapping.md` binds those fields and variants to production-shaped recovery summary, frame seed, hydration, runtime boundary, collect, and exact typed errors.
- Verus trusted boundary: ordered decoded events, validated snapshot metadata, decoded slot values, and digest bundle correctness.
- Prior raw evidence cited unchanged: State 6 rejection reran the same direct TLC command and canonical gate and observed the same missing jar and gauntlet parse blockers in `.beads/vb-qi37.1.6/proof-review.md` and `.beads/vb-qi37.1.6/contract-verification-review.md`.

---

## State 5 Attempt 3 Repair Evidence

Timestamp: `2026-05-15T22:54:57Z`.

### Isolation And Bead Reality

- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6`.
- Source checkout: `/home/lewis/src/velvet-ballistics`.
- The isolated workspace is neither equal to nor nested under the source checkout.
- `bd show vb-qi37.1.6 --json` against the local workspace `.beads` database failed because the local `.beads` store lacks the `issues` table; source-checkout server-mode database lookup was used for bead reality.

### `TMPDIR=target/tmp bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.1.6 --json`

exit=0

```text
id: vb-qi37.1.6
title: runtime/recovery: Crash restart integration evidence
status: in_progress
assignee: Lewis
```

### JSONL And Artifact Gate

Command:

`TMPDIR=target/tmp test -s .beads/vb-qi37.1.6/proof-writer-report.md && TMPDIR=target/tmp test -s .beads/vb-qi37.1.6/proof-evidence.md && TMPDIR=target/tmp jq -c . .beads/vb-qi37.1.6/proof-obligations.planned.jsonl >/dev/null && TMPDIR=target/tmp jq -c . .beads/vb-qi37.1.6/proof-obligations.jsonl >/dev/null && TMPDIR=target/tmp jq -c . .beads/vb-qi37.1.6/traceability-matrix.jsonl >/dev/null`

exit=0

### PO-003 Repair Check

Command:

`TMPDIR=target/tmp jq -r 'select(.id=="PO-003") | .id + ":" + .status + ":" + .mode + ":" + (.waiver.owner // "null")' .beads/vb-qi37.1.6/proof-obligations.planned.jsonl`

exit=0

```text
PO-003:waived:waiver:State5 proof-writer repair
```

Interpretation: `KANI-REC-001` no longer remains required/planned/unwaived. No Kani PASS is claimed; this is an explicit waiver/defer record with State 6 review required.

### Fresh Verus Rerun

Command:

`TMPDIR=target/tmp verus verification/verus/recovery_hydration_contracts.rs`

exit=0

```text
verification results:: 10 verified, 0 errors
warning: 11 warnings emitted
```

Interpretation: `PO-002` remains `PASS_LOCAL` supporting evidence for the `PO-003` waiver. Warnings are unchanged deprecation warnings for `ResultAdditionalSpecFns` helpers.

### Fresh TLC Rerun

Command:

`TMPDIR=target/tmp JAVA_TOOL_OPTIONS='-Djava.io.tmpdir=target/tmp' java -jar tla2tools.jar verification/tla/RecoveryCrashRestart.tla -config verification/tla/RecoveryCrashRestart.cfg`

exit=1

```text
Error: Unable to access jarfile tla2tools.jar
```

Interpretation: `TLA-REC-001` / `PO-001` and `PO-015` remain `BLOCK_LOCAL` tooling. No model-checking PASS is claimed.

### Fresh Canonical Gate Rerun

Command:

`TMPDIR=target/tmp moon run :verify-proof`

exit=2

```text
scripts/rust-verification-gauntlet.sh: line 3: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 4: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 5: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 6: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 7: syntax error near unexpected token `newline'
scripts/rust-verification-gauntlet.sh: line 7: `//! Usage: scripts/rust-verification-gauntlet.sh <mode>'
Error: task_runner::run_failed
```

Interpretation: `GATE-REC-001` / `PO-009` remains `UPSTREAM_INVALIDATION` / `BLOCK_LOCAL`. The canonical proof gate fails before scoped recovery proof artifacts and cannot provide State 6 approval evidence.

### Attempt 3 Completion Evidence

- `.beads/vb-qi37.1.6/proof-obligations.planned.jsonl` repaired for `PO-003` waiver/defer status.
- `.beads/vb-qi37.1.6/proof-writer-report.md` appended with attempt 3 transition/completion.
- `.beads/vb-qi37.1.6/proof-evidence.md` appended with fresh focused command evidence.
- `.beads/vb-qi37.1.6/STATE.md` appended with State 5 attempt 3 completion.
