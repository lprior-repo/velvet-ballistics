# Proof Evidence: vb-scxh

## Scope

- State: 5 proof-writer repair.
- Workspace: `/home/lewis/src/vb-scxh`.
- Source checkout `/home/lewis/src/Velvet-ballistics`: write-forbidden and not used for artifact writes.
- Artifact root used for writes: `.beads/vb-scxh/`.

## State 5 Covered Obligations

- `TLA-SCXH-001`: PASS. TLC checked `NoEngineUnblockBeforeApprovedEvidence` and `FalseClosuresVerifiedBeforeClose`.
- `TLA-SCXH-002`: PASS. TLC checked explicit `AttemptLaunderSubagentEvidence`, `NoAcceptanceFromSubagentRequiredEvidence`, and `LaunderingAttemptRejected`.
- `TLA-SCXH-003`: PASS. TLC checked `MutationUnviableNotPass`; `FAIL_UNVIABLE` remains non-pass/deferred.
- `TLA-SCXH-004`: PASS. TLC checked `ParityGapOwnershipPreserved`; generated parity remains deferred/non-exhaustive.
- `TLA-SCXH-005`: PASS. TLC command and artifacts use canonical `.beads/vb-scxh/tla/` paths only.
- `ERR-SCXH-010`: PASS for State 5 path evidence. Repaired plan/evidence paths use `.beads/vb-scxh/tla/`.

## Downstream Obligations

- `PATH-SCXH-001`: owner_state=11, rerun_from=3, State 5 status=`BLOCKED_SCOPE`; only workspace guard evidence captured here.
- `ART-SCXH-001`: owner_state=11, rerun_from=3, State 5 status=`NOT_RUN`.
- `BD-SCXH-001`: owner_state=11, rerun_from=3, State 5 status=`BLOCKED_SCOPE`; exact 12 false-closure IDs and raw statuses remain State 11.
- `BD-SCXH-002`: owner_state=11, rerun_from=3, State 5 status=`NOT_RUN`.
- `SAFETY-SCXH-001`: owner_state=11, rerun_from=3, State 5 status=`BLOCK_LOCAL`; bundle/bookmark verification remains visible blocker until raw verification passes.
- `CI-SCXH-001`: owner_state=11, rerun_from=3, State 5 status=`NOT_RUN`.
- `MUT-SCXH-001`: owner_state=11, rerun_from=3, State 5 status=`BLOCKED_SCOPE`; final mutation audit remains State 11.
- `SCOPE-SCXH-001`: owner_state=11, rerun_from=3, State 5 status=`NOT_RUN`.
- `TRUTH-SCXH-001`: owner_state=12, rerun_from=3, State 5 status=`NOT_RUN`.
- `SCOPEWRITE-SCXH-001`: owner_state=11, rerun_from=3, State 5 status=`NOT_RUN`.
- `ERR-SCXH-001`: owner_state=11, rerun_from=3, State 5 status=`NOT_RUN`.
- `ERR-SCXH-002`: owner_state=11, rerun_from=3, State 5 status=`NOT_RUN`.
- `ERR-SCXH-003`: owner_state=12, rerun_from=3, State 5 status=`NOT_RUN`.
- `ERR-SCXH-004`: owner_state=12, rerun_from=3, State 5 status=`BLOCKED_SCOPE`; State 5 supplies TLA support only, Truth Serum report remains State 12.
- `ERR-SCXH-005`: owner_state=12, rerun_from=3, State 5 status=`NOT_RUN`.
- `ERR-SCXH-006`: owner_state=12, rerun_from=3, State 5 status=`BLOCK_LOCAL`; safety anchor failure must map to `Error::SafetyAnchorMissing` downstream.
- `ERR-SCXH-007`: owner_state=12, rerun_from=3, State 5 status=`NOT_RUN`.
- `ERR-SCXH-008`: owner_state=12, rerun_from=3, State 5 status=`NOT_RUN`.
- `ERR-SCXH-009`: owner_state=12, rerun_from=3, State 5 status=`NOT_RUN`.

## Waiver/Non-State-5 Rows

- `WAIVE-VERUS-SCXH-001`: owner_state=4, rerun_from=3, State 5 status=`NOT_RUN`; no Verus proof claimed.
- `WAIVE-LEAN-SCXH-001`: owner_state=4, rerun_from=3, State 5 status=`NOT_RUN`; no Lean/Aeneas/Hax proof claimed.
- `WAIVE-KANI-SCXH-001`: owner_state=4, rerun_from=3, State 5 status=`NOT_RUN`; no Kani proof claimed.
- `WAIVE-FLUX-SCXH-001`: owner_state=4, rerun_from=3, State 5 status=`NOT_RUN`; no Flux proof claimed.
- `WAIVE-LOOM-SCXH-001`: owner_state=4, rerun_from=3, State 5 status=`NOT_RUN`; no Loom/Shuttle proof claimed.
- `WAIVE-MIRI-SCXH-001`: owner_state=4, rerun_from=3, State 5 status=`NOT_RUN`; no Miri/cargo-careful proof claimed.
- `WAIVE-PROPFUZZ-SCXH-001`: owner_state=4, rerun_from=3, State 5 status=`NOT_RUN`; no proptest/fuzz proof claimed.
- `WAIVE-PERF-API-REL-SCXH-001`: owner_state=4, rerun_from=3, State 5 status=`NOT_RUN`; no performance/API/release proof claimed.

## Command Evidence

### Workspace Guard

- Workdir: `/home/lewis/src/vb-scxh`.
- Command: `pwd -P`.
- Status: PASS.
- Output:

```text
/home/lewis/src/vb-scxh
```

### Planned JSONL Parse/Count

- Workdir: `/home/lewis/src/vb-scxh`.
- Command: `python -c 'import json, pathlib; rows=[json.loads(line) for line in pathlib.Path(".beads/vb-scxh/proof-obligations.planned.jsonl").read_text().splitlines() if line.strip()]; print(len(rows))'`.
- Status: PASS.
- Output:

```text
33
```

### TLA+ Execution

- Workdir: `/home/lewis/src/vb-scxh`.
- Command: `tlc -config .beads/vb-scxh/tla/ScxhRecovery.cfg .beads/vb-scxh/tla/ScxhRecovery.tla`.
- Exit: 0.
- Status: PASS for configured safety invariants only.
- Output markers:

```text
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Warning: Please run the Java VM which executes TLC with a throughput optimized garbage collector by passing the "-XX:+UseParallelGC" property.
Model checking completed. No error has been found.
12277 states generated, 984 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 12.
Finished in 00s at (2026-05-14 13:32:13)
```

### Required Outputs Non-Empty

- Workdir: `/home/lewis/src/vb-scxh`.
- Command: `test -s .beads/vb-scxh/tla/ScxhRecovery.tla && test -s .beads/vb-scxh/tla/ScxhRecovery.cfg && test -s .beads/vb-scxh/tla-report.md && test -s .beads/vb-scxh/proof-evidence.md && test -s .beads/vb-scxh/proof-writer-report.md`.
- Status: PASS.

### Safety Anchor

- Planned command: `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle && git show-ref rescue-vb-scxh-ci-green-20260513T030158Z`.
- Status: `BLOCK_LOCAL` unless rerun by State 11 and raw verification passes.
- Prior raw failure preserved:

```text
error: could not open '/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle'
```

## Liveness/Fairness Stance

- Liveness/fairness is explicitly waiver-candidate and not closure evidence in State 5.
- The repaired TLC run is safety-only. It does not prove eventual raw evidence production, eventual Truth Serum approval, eventual bead closure, or eventual engine unblock.
- Closure/unblock remains blocked until State 11 evidence packaging and State 12 Truth Serum produce passing raw evidence or approved waivers.

## Failure Packet Seeds

- Exact 12 false-closure IDs and raw reopened/linked BD evidence remain uncaptured until State 11.
- Safety bundle/bookmark verification remains `BLOCK_LOCAL` until raw verification passes.
- Green `moon ci` evidence freshness and raw marker audit remain State 11.
- Mutation adequacy remains unsatisfied; `FAIL_UNVIABLE` is not PASS.
- Generated parity gaps remain deferred to `vb-gvmt` / `vb-qi37.10` and cannot close engine-only acceptance.
