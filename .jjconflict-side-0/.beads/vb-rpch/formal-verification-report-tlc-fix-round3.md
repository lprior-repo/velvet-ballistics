# Formal Verification Report — TLC Fix Round 3 for `vb-rpch`

Status: **FORMAL EXECUTION COMPLETE FOR ROUND-3 TLC CAUSAL REPAIR**. This is command-evidence closure; proof-reviewer approval remains a separate gate.

Date: 2026-05-24
Agent lane: `formal-verifier`
Scope: round-3 causal repairs for `.beads/vb-rpch/proof-obligations.tlc-fix-round2.planned.jsonl`

## Primary bounded safety command

Exact command run from `/home/lewis/src/vb-jpq7-jj-fix`:

```bash
tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull.tlc.log 2>&1
```

Classification: `PASS`

Raw evidence: `evidence/specs/RecoveryReplayFull.tlc.log`

Observed final evidence:

- `Model checking completed. No error has been found.`
- `31707925 states generated, 3448392 distinct states found, 0 states left on queue.`
- Complete state graph depth: `12`
- Runtime: `01min 17s`

Checked invariants:

- `TypeOK`
- `TailCausalAfterSnapshot`
- `ReplaySeqOrder`
- `OnlyIncompleteRuns`
- `NoResolvedReExecution`
- `DigestVerificationOrder`

## Independent non-vacuity witnesses

All independent witness commands were rerun against the cleaned current `RecoveryReplayFull.tla`. Each expected negated reachability invariant violation was observed.

| Obligation | Classification | Evidence | Expected violation |
|---|---:|---|---|
| `TLC-R2-005` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-replay-order.tlc.log` | `NotReachReplaySeqOrderAntecedent` |
| `TLC-R2-006` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-snapshot-tail.tlc.log` | `NotReachTailCausalAntecedent` |
| `TLC-R2-007` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-incomplete-runs.tlc.log` | `NotReachRecoveredRunsNonEmpty` |
| `TLC-R2-008` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-terminal-excluded.tlc.log` | `NotReachTerminalExcludedFromRecovered` |
| `TLC-R2-009` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-no-reexecution.tlc.log` | `NotReachResolvedActionGuardAntecedent` |
| `TLC-R2-010` | `PASS` | `evidence/specs/RecoveryReplayFull-nonvacuity-digest-order.tlc.log` | `NotReachDigestIrAfterWorkflow` |

The combined legacy non-vacuity command was also rerun:

```bash
tlc -config specs/tla/RecoveryReplayFull-nonvacuity.cfg specs/tla/RecoveryReplayFull.tla > evidence/specs/RecoveryReplayFull-nonvacuity.tlc.log 2>&1
```

It produced the expected violation of `NotAllNonVacuityWitnessesReached` after the combined predicate was updated to require `ReachResolvedActionGuardAntecedent`.

## Per-error causal reachability witnesses

All per-error witness commands were rerun against the cleaned current `RecoveryReplayFull.tla`. Each expected negated error reachability invariant violation was observed.

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

Round-3 causal repairs visible in traces:

- ABI mismatch logs show `abi_expected = 1`, `abi_found = 2`.
- Policy mismatch logs show `policy_expected = 1`, `policy_found = 2`.
- Corrupt snapshot logs show `snapshot_inputs = {"Corrupt"}` and a loaded snapshot before `CorruptSnapshot`.
- Non-idempotent blocked logs show `replay_candidates = {[step |-> 1, action |-> 1, attempt |-> 1]}` after an action completion.

## Source/evidence sync

Exact command run from `/home/lewis/src/vb-jpq7-jj-fix`:

```bash
cmp -s specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && cmp -s specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg && cmp -s specs/tla/RecoveryReplayFull-large-stress.cfg evidence/specs/RecoveryReplayFull-large-stress.cfg
```

Classification: `PASS`

The command exited successfully and produced no output.

## Optional large stress cfg

Classification: `WAIVED`

`TLC-R2-002` remains `required:false` / optional stress. It was not run. No round-3 proof claim depends on the large-stress cfg.

## Limitations for reviewer

- Primary proof remains bounded to the configured finite abstraction: singleton run/step/action/attempt, `MAX_SEQ=3`, `MAX_EVENTS=3`.
- No liveness/fairness properties were run.
- Snapshot corruption, ABI digests, and policy digests are finite abstract inputs, not byte-decoder, cryptographic, or runtime lookup implementation proof.
- Witness commands intentionally terminate at expected invariant violations; those nonzero TLC outcomes are classified as `PASS` only for reachability-witness obligations.

## Closure summary

- Required round-3 TLC commands: `PASS`
- Optional stress command: `WAIVED`
- Behavior-affecting waivers accepted: none
- Required proof-reviewer disposition: pending
