# Proof Review: vb-scxh

STATUS: APPROVED

## Review Scope

- Role: Go-skill State 6 proof-reviewer constrained through general agent.
- Workdir: `/home/lewis/src/vb-scxh`.
- Write scope: `.beads/vb-scxh/proof-review.md`, `.beads/vb-scxh/proof-findings.jsonl`, `.beads/vb-scxh/proof-repair-guide.md` only.
- Production, contract, proof, TLA, and test artifacts edited: none.
- Skill basis read: `/home/lewis/.claude/skills/proof-reviewer/SKILL.md` lines 17-24 require findings-first, no proof repair, raw evidence, vacuity checks, and mandatory verification; lines 37-49 require discovery and verifier checks; lines 77-80 require explicit status and structured findings.

## Inputs Reviewed

- `.beads/vb-scxh/proof-obligations.planned.jsonl`
- `.beads/vb-scxh/proof-obligations.jsonl`
- `.beads/vb-scxh/proof-evidence.md`
- `.beads/vb-scxh/proof-writer-report.md`
- `.beads/vb-scxh/tla-report.md`
- `.beads/vb-scxh/tla/ScxhRecovery.tla`
- `.beads/vb-scxh/tla/ScxhRecovery.cfg`
- `.beads/vb-scxh/tla-spec.md`
- `.beads/vb-scxh/traceability-matrix.jsonl`
- `.beads/vb-scxh/verification-layers.md`
- `.beads/vb-scxh/STATE.md`
- Prior `.beads/vb-scxh/proof-review.md`, `.beads/vb-scxh/proof-findings.jsonl`, and `.beads/vb-scxh/proof-repair-guide.md`.

## Findings

No blocking proof-review findings remain for the State 5 proof basis.

## Validation Commands

```text
pwd -P
```

Result: `/home/lewis/src/vb-scxh`.

```text
test -s .beads/vb-scxh/proof-obligations.planned.jsonl
```

Result: PASS.

```text
jq -c . .beads/vb-scxh/proof-obligations.planned.jsonl >/dev/null
jq -c . .beads/vb-scxh/proof-obligations.jsonl >/dev/null
```

Result: PASS for both JSONL ledgers.

```text
jq -s length .beads/vb-scxh/proof-obligations.planned.jsonl
jq -s length .beads/vb-scxh/proof-obligations.jsonl
jq -s length .beads/vb-scxh/traceability-matrix.jsonl
```

Result: planned obligations `33`, primary obligations `33`, traceability rows `27`.

```text
test -s .beads/vb-scxh/tla/ScxhRecovery.tla
test -s .beads/vb-scxh/tla/ScxhRecovery.cfg
```

Result: PASS for both canonical TLA artifacts.

```text
tlc -config .beads/vb-scxh/tla/ScxhRecovery.cfg .beads/vb-scxh/tla/ScxhRecovery.tla
```

Result: PASS. TLC 2.19 completed with no error found, 12,277 states generated, 984 distinct states, 0 states left on queue, complete depth 12.

```text
git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle
```

Result: FAIL as expected for current local raw evidence state: `error: could not open '/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle'`.

## Review Decisions

- 33-row plan alignment: APPROVED. `proof-obligations.planned.jsonl` and `proof-obligations.jsonl` both parse and contain 33 rows. The TLA rows, raw-evidence rows, error rows, and waiver rows are represented.
- TLA path mismatch repair: APPROVED. Current authoritative proof ledgers, `tla-spec.md`, `verification-layers.md`, `proof-evidence.md`, `proof-writer-report.md`, and `tla-report.md` use canonical `.beads/vb-scxh/tla/ScxhRecovery.tla` and `.beads/vb-scxh/tla/ScxhRecovery.cfg` paths. Remaining `.beads/vb-scxh/specs/` mentions are historical/rejection-context or explicit negative path guards, not active proof targets.
- TLC evidence: APPROVED. The exact canonical command reran successfully and matches the State 5 report markers: no invariant violation, 12,277 states generated, 984 distinct states, depth 12.
- Subagent-laundering invariant: APPROVED. The model now includes `AttemptLaunderSubagentEvidence` on required evidence item `safety_bundle`, prevents subagent classification from satisfying `NoSubagentRequired`, rejects attempted laundering through `TruthSerumRejectLaunderedEvidence`, and checks `NoAcceptanceFromSubagentRequiredEvidence` plus `LaunderingAttemptRejected`. This is no longer the prior tautological `Subagent != Raw` proof.
- Safety-anchor `BLOCK_LOCAL`: APPROVED AS DEFERRED, NOT PASSED. The failed bundle check remains a real landing/closure blocker, but it is assigned to State 11/12 raw-evidence artifacts (`SAFETY-SCXH-001`, `ERR-SCXH-006`) and is not claimed as a State 5 proof pass. It does not block State 6 proof approval because the proof basis explicitly models missing/failed safety anchors as preventing approval/unblock.
- Liveness/fairness: APPROVED AS NOT CLAIMED. TLC checks safety invariants only. `proof-evidence.md`, `proof-writer-report.md`, and `tla-report.md` explicitly state that liveness/fairness is not closure evidence and does not prove eventual State 11 evidence production, final approval, closure, or engine unblock.
- Later-state deferrals: APPROVED. State 11/12 obligations are present and not falsely claimed as passed. `BD-SCXH-001`, `CI-SCXH-001`, `MUT-SCXH-001`, `SCOPE-SCXH-001`, `TRUTH-SCXH-001`, and related error rows remain downstream evidence gates.

## Non-Approval Boundaries

- This approval is proof-basis approval only.
- This does not approve bead closure.
- This does not approve `vb-engine-yaml` unblock.
- This does not waive the missing safety bundle.
- This does not claim raw BD exact-12 evidence, green CI freshness, mutation adequacy, generated parity exhaustiveness, or final Truth Serum approval.

## Routing

- State 6 proof review: APPROVED.
- State 11 must produce raw evidence audits for BD exact-12 closure recovery, safety anchor, CI, mutation classification, scope control, and assurance bundle packaging.
- State 12 must produce Truth Serum and final evidence decision.
- Landing/closure/unblock remains blocked until State 11/12 raw-evidence gates pass or produce explicit blocking failure packets.
