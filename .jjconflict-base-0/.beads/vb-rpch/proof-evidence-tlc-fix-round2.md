# Proof Evidence — TLC Fix Round 2 (`vb-rpch`)

## Tool discovery

Command run:

```sh
command -v tlc && tlc -version || true
```

Observed output:

```text
/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Error: Error: unrecognized option: -version
```

Classification: `TOOL_AVAILABLE`; wrapper prints version before rejecting `-version`.

## Primary bounded proof command

Obligations: `TLC-R2-001`, `TLC-R2-003`, `TLC-R2-004`.

```sh
tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull.tlc.log 2>&1
```

Classification: `PASS_BOUNDED_SAFETY`.

Evidence path: `evidence/specs/RecoveryReplayFull.tlc.log`.

Key lines:

- `Model checking completed. No error has been found.`
- `36532738 states generated, 4088199 distinct states found, 0 states left on queue.`
- `The depth of the complete state graph search is 11.`

Bounds checked: `RunId={1}`, `StepId={1}`, `ActionId={1}`, `Attempt={1}`, all event types, `MAX_SEQ=3`, `MAX_EVENTS=3`.

Invariants checked: `TypeOK`, `TailCausalAfterSnapshot`, `ReplaySeqOrder`, `OnlyIncompleteRuns`, `NoResolvedReExecution`, `DigestVerificationOrder`.

## Independent non-vacuity witnesses

Each command intentionally checks a negated reachability invariant. TLC returning an invariant violation is the expected result for that witness.

Execution note: an initial parallel witness attempt hit TLC's timestamp-based `states/` metadir collision for several witnesses. Those logs were overwritten by the sequential reruns below; the retained evidence paths listed here are the final raw logs used for classification.

| Obligation | Command | Log | Classification |
|---|---|---|---|
| `TLC-R2-005` | `tlc -config specs/tla/RecoveryReplayFull-nonvacuity-replay-order.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-replay-order.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-nonvacuity-replay-order.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachReplaySeqOrderAntecedent` |
| `TLC-R2-006` | `tlc -config specs/tla/RecoveryReplayFull-nonvacuity-snapshot-tail.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-snapshot-tail.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-nonvacuity-snapshot-tail.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachTailCausalAntecedent` |
| `TLC-R2-007` | `tlc -config specs/tla/RecoveryReplayFull-nonvacuity-incomplete-runs.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-incomplete-runs.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-nonvacuity-incomplete-runs.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachRecoveredRunsNonEmpty` |
| `TLC-R2-008` | `tlc -config specs/tla/RecoveryReplayFull-nonvacuity-terminal-excluded.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-terminal-excluded.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-nonvacuity-terminal-excluded.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachTerminalExcludedFromRecovered` |
| `TLC-R2-009` | `tlc -config specs/tla/RecoveryReplayFull-nonvacuity-no-reexecution.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachResolvedActionGuardAntecedent` |
| `TLC-R2-010` | `tlc -config specs/tla/RecoveryReplayFull-nonvacuity-digest-order.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity-digest-order.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-nonvacuity-digest-order.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachDigestIrAfterWorkflow` |

## Per-error reachability witnesses

| Obligation | Command | Log | Classification |
|---|---|---|---|
| `TLC-R2-011` | `tlc -config specs/tla/RecoveryReplayFull-error-workflow-source-digest-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-workflow-source-digest-mismatch.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-error-workflow-source-digest-mismatch.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachErrorWorkflowSourceDigestMismatch` |
| `TLC-R2-012` | `tlc -config specs/tla/RecoveryReplayFull-error-compiled-ir-digest-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-compiled-ir-digest-mismatch.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-error-compiled-ir-digest-mismatch.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachErrorCompiledIrDigestMismatch`; trace shows `WorkflowChecked` before IR mismatch |
| `TLC-R2-013` | `tlc -config specs/tla/RecoveryReplayFull-error-no-recovery-data.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-no-recovery-data.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-error-no-recovery-data.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachErrorNoRecoveryData` |
| `TLC-R2-014` | `tlc -config specs/tla/RecoveryReplayFull-error-corrupt-snapshot.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-corrupt-snapshot.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-error-corrupt-snapshot.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachErrorCorruptSnapshot` |
| `TLC-R2-015` | `tlc -config specs/tla/RecoveryReplayFull-error-action-abi-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-action-abi-mismatch.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-error-action-abi-mismatch.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachErrorActionAbiMismatch` |
| `TLC-R2-016` | `tlc -config specs/tla/RecoveryReplayFull-error-policy-digest-mismatch.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-policy-digest-mismatch.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-error-policy-digest-mismatch.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachErrorPolicyDigestMismatch` |
| `TLC-R2-017` | `tlc -config specs/tla/RecoveryReplayFull-error-non-idempotent-action-blocked.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-non-idempotent-action-blocked.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-error-non-idempotent-action-blocked.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachErrorNonIdempotentActionBlocked` |
| `TLC-R2-018` | `tlc -config specs/tla/RecoveryReplayFull-error-replay-divergence.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-replay-divergence.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-error-replay-divergence.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachErrorReplayDivergence` |
| `TLC-R2-019` | `tlc -config specs/tla/RecoveryReplayFull-error-frame-dimension-overflow.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-error-frame-dimension-overflow.tlc.log 2>&1` | `evidence/specs/RecoveryReplayFull-error-frame-dimension-overflow.tlc.log` | `EXPECTED_WITNESS_VIOLATION`: `NotReachErrorFrameDimensionOverflow` |

## Evidence sync

Obligation: `TLC-R2-020`.

Commands run:

```sh
cp specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && cp specs/tla/RecoveryReplayFull*.cfg evidence/specs/ && cmp -s specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && cmp -s specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg && cmp -s specs/tla/RecoveryReplayFull-large-stress.cfg evidence/specs/RecoveryReplayFull-large-stress.cfg
```

```sh
cmp -s specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && cmp -s specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg && cmp -s specs/tla/RecoveryReplayFull-large-stress.cfg evidence/specs/RecoveryReplayFull-large-stress.cfg > evidence/specs/RecoveryReplayFull-sync.cmp.log 2>&1
```

Classification: `PASS_SYNC_CHECK`.

## Artifact syntax hygiene

Command run:

```sh
python -c 'import json, pathlib; p=pathlib.Path(".beads/vb-rpch/trusted-base-ledger.tlc-fix-round2.jsonl"); [json.loads(line) for line in p.read_text().splitlines() if line.strip()]'
```

Classification: `PASS_JSONL_PARSE`; command exited silently.

## Pending stress

Not run in this pass:

```sh
tlc -config specs/tla/RecoveryReplayFull-large-stress.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-large-stress.tlc.log 2>&1
```

Classification: `PENDING_OPTIONAL_STRESS`; no proof claim depends on it.
