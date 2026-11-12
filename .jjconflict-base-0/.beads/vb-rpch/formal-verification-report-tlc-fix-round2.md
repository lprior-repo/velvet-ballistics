# Formal Verification Report — TLC Fix Round 2 for `vb-rpch`

Status: **FORMAL EXECUTION COMPLETE FOR ROUND-2 REQUIRED TLC OBLIGATIONS**. This is command-evidence closure, not proof-review approval.

Date: 2026-05-24
Agent lane: `formal-verifier`
Scope: `.beads/vb-rpch/proof-obligations.tlc-fix-round2.planned.jsonl`

## Required command evidence

### TLC-R2-001 / TLC-R2-003 / TLC-R2-004 — primary bounded safety model

Exact command run from `/home/lewis/src/vb-jpq7-jj-fix`:

```bash
tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull.tlc.log 2>&1
```

Classification: `PASS`

Raw evidence: `evidence/specs/RecoveryReplayFull.tlc.log`

Observed final evidence:

- `Model checking completed. No error has been found.`
- `36532738 states generated, 4088199 distinct states found, 0 states left on queue.`
- Complete state graph depth: `11`
- Runtime: `01min 24s`

Bounds:

- `RunId = {1}`
- `StepId = {1}`
- `ActionId = {1}`
- `Attempt = {1}`
- `MAX_SEQ = 3`
- `MAX_EVENTS = 3`
- `EnabledEventTypes` includes all modeled event types in `RecoveryReplayFull.cfg`.

Checked invariants:

- `TypeOK`
- `TailCausalAfterSnapshot`
- `ReplaySeqOrder`
- `OnlyIncompleteRuns`
- `NoResolvedReExecution`
- `DigestVerificationOrder`

### TLC-R2-005 through TLC-R2-010 — independent non-vacuity witnesses

All witness commands were run exactly with stdout/stderr redirected to their planned evidence paths. Each obligation expected TLC to find a counterexample to a negated reachability invariant. The expected invariant violation was observed in each log.

| Obligation | Classification | Evidence | Expected violation |
|---|---:|---|---|
| `TLC-R2-005` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-replay-order.tlc.log` | `NotReachReplaySeqOrderAntecedent` |
| `TLC-R2-006` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-snapshot-tail.tlc.log` | `NotReachTailCausalAntecedent` |
| `TLC-R2-007` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-incomplete-runs.tlc.log` | `NotReachRecoveredRunsNonEmpty` |
| `TLC-R2-008` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-terminal-excluded.tlc.log` | `NotReachTerminalExcludedFromRecovered` |
| `TLC-R2-009` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log` | `NotReachResolvedActionGuardAntecedent` |
| `TLC-R2-010` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-digest-order.tlc.log` | `NotReachDigestIrAfterWorkflow` |

These are reachability witnesses only; they do not replace the primary invariant model check.

### TLC-R2-011 through TLC-R2-019 — per-error reachability witnesses

All per-error witness commands were run exactly with stdout/stderr redirected to their planned evidence paths. Each obligation expected TLC to find a counterexample to a negated `last_error = <variant>` reachability invariant. The expected invariant violation was observed in each log.

| Obligation | Classification | Evidence | Expected violation |
|---|---:|---|---|
| `TLC-R2-011` | `PASS` | `evidence/specs/RecoveryReplayFull-error-workflow-source-digest-mismatch.tlc.log` | `NotReachErrorWorkflowSourceDigestMismatch` |
| `TLC-R2-012` | `PASS` | `evidence/specs/RecoveryReplayFull-error-compiled-ir-digest-mismatch.tlc.log` | `NotReachErrorCompiledIrDigestMismatch` |
| `TLC-R2-013` | `PASS` | `evidence/specs/RecoveryReplayFull-error-no-recovery-data.tlc.log` | `NotReachErrorNoRecoveryData` |
| `TLC-R2-014` | `PASS` | `evidence/specs/RecoveryReplayFull-error-corrupt-snapshot.tlc.log` | `NotReachErrorCorruptSnapshot` |
| `TLC-R2-015` | `PASS` | `evidence/specs/RecoveryReplayFull-error-action-abi-mismatch.tlc.log` | `NotReachErrorActionAbiMismatch` |
| `TLC-R2-016` | `PASS` | `evidence/specs/RecoveryReplayFull-error-policy-digest-mismatch.tlc.log` | `NotReachErrorPolicyDigestMismatch` |
| `TLC-R2-017` | `PASS` | `evidence/specs/RecoveryReplayFull-error-non-idempotent-action-blocked.tlc.log` | `NotReachErrorNonIdempotentActionBlocked` |
| `TLC-R2-018` | `PASS` | `evidence/specs/RecoveryReplayFull-error-replay-divergence.tlc.log` | `NotReachErrorReplayDivergence` |
| `TLC-R2-019` | `PASS` | `evidence/specs/RecoveryReplayFull-error-frame-dimension-overflow.tlc.log` | `NotReachErrorFrameDimensionOverflow` |

### TLC-R2-020 — source/evidence sync

Exact command run from `/home/lewis/src/vb-jpq7-jj-fix`:

```bash
cmp -s specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && cmp -s specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg && cmp -s specs/tla/RecoveryReplayFull-large-stress.cfg evidence/specs/RecoveryReplayFull-large-stress.cfg
```

Classification: `PASS`

The command exited successfully and produced no output.

### TLC-R2-002 — large stress cfg

Classification: `WAIVED`

Rationale: `TLC-R2-002` is marked `required:false` and `optional-stress` in `.beads/vb-rpch/proof-obligations.tlc-fix-round2.planned.jsonl`. It was not run in this formal pass. No round-2 proof claim depends on `specs/tla/RecoveryReplayFull-large-stress.cfg` completion. Prior large-bound execution is explicitly superseded by the bounded primary proof and remains bug-finding/stress only unless it drains.

## Limitations and caveats for reviewer

- Round-2 primary proof is bounded to singleton domains and `MAX_SEQ=3`, `MAX_EVENTS=3`.
- This pass proves safety over that finite abstraction only.
- No liveness/fairness properties were run.
- Digest values are finite abstract values, not cryptographic proof.
- `ActionAbiMismatch` and `PolicyDigestMismatch` witnesses are abstract typed error-domain witnesses; they do not prove deferred runtime GAP-3 lookup implementation.
- Witness runs intentionally stop at expected invariant violations; nonzero TLC exit for those commands is expected evidence, not a failed witness.

## Closure summary

- Required round-2 TLC commands: `PASS`
- Optional stress command: `WAIVED` as non-required bug-finding/stress
- Accepted behavior-affecting waivers: none
- Required proof-reviewer disposition: pending
