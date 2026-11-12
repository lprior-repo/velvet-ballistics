# Domain Model Review: vb-scxh

## Verdict

The domain is evidence provenance and closure safety. Illegal acceptance states must be made explicit in downstream artifacts: subagent-only evidence, missing BD IDs, missing safety anchors, mutation `FAIL_UNVIABLE` as PASS, and generated parity scope conflation all block close/unblock.

## Entities

- RecoveryBead: `vb-scxh`, release-blocking evidence-integrity bead.
- BlockedBead: `vb-engine-yaml`, remains blocked until final evidence decision approves.
- FollowUpBead: `vb-gvmt` / `vb-qi37.10`, owns generated parity gaps.
- FalseClosureSet: exactly 12 bead IDs plus raw reopened/linked/follow-up status evidence.
- EvidenceItem: raw command, artifact-derived claim, subagent narrative, deferral, or blocker.
- SafetyAnchor: bundle plus bookmark/ref; current bundle-open failure is a local blocker.
- AssuranceBundle: State 11 packaged evidence consumed by Truth Serum.
- FinalEvidenceDecision: State 12 decision; only this may authorize close/unblock.

## Value objects

- WorkspacePath = `/home/lewis/src/vb-scxh`.
- ArtifactWritePath = `/home/lewis/src/vb-scxh/.beads/vb-scxh/*`.
- BdDbPath = `/home/lewis/src/.beads/dolt`.
- EvidenceClassification = `RAW_COMMAND | ARTIFACT_DERIVED | SUBAGENT_CLAIM | DEFERRED | BLOCKED`.
- MutationClassification = `PASS | FAIL | FAIL_UNVIABLE | DEFERRED`; current permitted classification is `FAIL_UNVIABLE` plus `DEFERRED`.
- TlaArtifactPath = `.beads/vb-scxh/tla/ScxhRecovery.{tla,cfg}`.

## State machine

1. `Claimed`: isolated workspace and BD claim exist.
2. `Discovered`: required inputs and referenced evidence paths are known.
3. `Contracted`: repaired State 3 artifacts define exact obligations.
4. `ProofPlanned`: State 4 plans against `.beads/vb-scxh/tla/`.
5. `ProofExecuted`: State 5 reruns exact TLC command and records path-consistent report.
6. `EvidencePackaged`: State 11 enumerates BD IDs, safety, CI, mutation, scope deferrals.
7. `TruthSerumReviewed`: State 12 rejects unsupported/laundered claims.
8. `DecisionReady`: final decision is `APPROVE_CLOSE_OR_UNBLOCK` or `BLOCKED`.
9. `ClosedOrBlocked`: BD transition has raw command evidence; engine stays blocked unless approved.

## Illegal states

- A `SUBAGENT_CLAIM` satisfies required evidence.
- Exact 12 false-closure IDs are inferred from prose rather than raw BD output.
- Safety bundle/bookmark failure is waived silently or treated as pass.
- `FAIL_UNVIABLE` mutation evidence is presented as mutation adequacy.
- Generated parity caveats are omitted or used as closure proof for this bead.
- TLA obligations target `.beads/vb-scxh/specs/` while executed artifacts are under `.beads/vb-scxh/tla/`.
- State 3 writes outside `.beads/vb-scxh/`.

## Boundary review

- TLA+ owns temporal lifecycle and evidence promotion/rejection.
- Verus has no current Rust target; waiver is valid only while no pure classifier implementation exists.
- Lean is optional only for a tiny evidence lattice; no repository Lean target is known.
- BD, git safety anchors, CI, and report audits require raw command/manual evidence.

## Acceptance implication

Approval requires raw evidence for each required lane or an explicit approved waiver. Current safety anchor failure remains `BLOCK_LOCAL` and prevents close/unblock until repaired downstream.
