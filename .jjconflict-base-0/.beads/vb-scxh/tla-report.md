# TLA+ Report: vb-scxh

STATUS: APPROVED

## Scope

- State: 11 evidence-environment repair rerun.
- Obligations: `TLA-SCXH-001`, `TLA-SCXH-002`, `TLA-SCXH-003`, `TLA-SCXH-004`, `TLA-SCXH-005`, `ERR-SCXH-010`.
- Model: `.beads/vb-scxh/tla/ScxhRecovery.tla`.
- Config: `.beads/vb-scxh/tla/ScxhRecovery.cfg`.
- Workdir: `/home/lewis/src/vb-scxh`.

## Command Evidence

Repo-local temp/metadir rerun used to avoid prior `/tmp` disk-quota blocker:

```text
mkdir -p .beads/vb-scxh/tla-metadir .beads/vb-scxh/tmp && TMPDIR=/home/lewis/src/vb-scxh/.beads/vb-scxh/tmp JAVA_TOOL_OPTIONS='-Djava.io.tmpdir=/home/lewis/src/vb-scxh/.beads/vb-scxh/tmp' tlc -metadir .beads/vb-scxh/tla-metadir -config .beads/vb-scxh/tla/ScxhRecovery.cfg .beads/vb-scxh/tla/ScxhRecovery.tla
```

The obligation's TLC model/config remained the canonical `.beads/vb-scxh/tla` paths. Temporary TLC work directories were removed after the run.

## Result

- Exit: 0.
- Status: PASS for configured safety invariants only.
- TLC: `TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)`.
- Warning: TLC emitted the standard JVM throughput warning recommending `-XX:+UseParallelGC`; no model warning or invariant violation was reported.
- State graph: `12277 states generated, 984 distinct states found, 0 states left on queue`.
- Depth: `12`.
- Terminal marker: `Model checking completed. No error has been found.`
- Timestamp from TLC output: started `2026-05-14 15:57:20`, finished `2026-05-14 15:57:21`.

## Checked Invariants

- `TypeOK`.
- `NoEngineUnblockBeforeApprovedEvidence`.
- `FalseClosuresVerifiedBeforeClose`.
- `NoAcceptanceFromSubagentRequiredEvidence`.
- `LaunderingAttemptRejected`.
- `MutationUnviableNotPass`.
- `ParityGapOwnershipPreserved`.
- `SafetyAnchorRequiredForApproval`.

## Laundering Repair

- `AttemptLaunderSubagentEvidence` explicitly attempts to use required evidence item `safety_bundle` as `Subagent` and also records `subagent_only_claim` as `Subagent`.
- `RecordRequiredRawEvidence` no longer upgrades `Subagent` evidence to `Raw`; subagent-sourced required evidence remains unsatisfied.
- `TruthSerumRejectLaunderedEvidence` rejects the packaged laundering attempt, marks `safety_bundle` as `Blocked`, sets `finalDecision = "Rejected"`, and keeps `engineBlocked = TRUE`.
- `NoAcceptanceFromSubagentRequiredEvidence` blocks final approval when required evidence remains subagent-only.
- `LaunderingAttemptRejected` checks both the attempted and rejected states: attempted laundering cannot approve or unblock, and rejected laundering is terminally blocked/rejected.

## Liveness/Fairness Stance

- No liveness property is configured as closure evidence in this State 5 repair.
- Liveness/fairness is waiver-candidate only and not closure evidence: this TLC PASS proves finite safety invariants, not eventual State 11 evidence production, bead closure, or engine unblock.
- Downstream State 11/12 raw-evidence audits and Truth Serum final decision remain required before any closure/unblock claim.

## Assumptions And Bounds

- The model abstracts BD, git bundle/bookmark verification, `moon ci`, cargo-mutants, report parsing, and Truth Serum runtime behavior.
- The finite model has 12 disputed closure atoms and 6 named evidence atoms.
- Mutation `FAIL_UNVIABLE` is modeled as deferred/non-pass.
- Generated parity gap is modeled as deferred/non-exhaustive.
- Safety bundle/bookmark verification remains raw-evidence gated; failed or missing safety anchors prevent approval.
- This report does not close `vb-scxh`, unblock `vb-engine-yaml`, or satisfy State 11/12 evidence packaging.
