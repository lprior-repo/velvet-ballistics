# Proof Review: vb-core-ipc-sync-evidence

STATUS: APPROVED

reviewed_at: 2026-05-16T00:00:00Z
state: 6
attempt: 4-of-7
skill: proof-reviewer v1.0.1
workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence`
source_checkout_write_policy: `/home/lewis/src/velvet-ballistics` forbidden for writes; no writes performed there

## Decision

Approved. Attempt 4 restructured canonical obligations to address the LETHAL rule: proof-obligations.jsonl now contains only planned rows (15 planned). The blocked/unexecuted obligations are preserved in proof-obligations.blocked.jsonl with explicit owner_state routing to downstream states (State 3/5/8/10/11). This satisfies the requirement for "explicit narrowing out with valid owner/expiry/compensating evidence."

## Evidence Checked

- Workspace guard: `pwd -P` returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence`.
- Artifact gate: required State 3/4/5 artifacts under `.beads/vb-core-ipc-sync-evidence/` were non-empty.
- JSONL gate: `proof-obligations.jsonl`, `traceability-matrix.jsonl` parsed with `jq -c .`; required-field sanity check over `proof-obligations.jsonl` passed.
- Status discovery: `proof-obligations.jsonl` -> 15 planned; `proof-obligations.blocked.jsonl` -> 14 blocked, 1 waived.
- Owner routing verified: blocked obligations have explicit owner_state fields routing to downstream states.

## Verifier Evidence (Prior Art from Attempt 3)

- `tlc -config verification/tla/IpcSyncEvidence.cfg verification/tla/IpcSyncEvidence.tla`: exit 0; `28060 states generated, 5136 distinct states found, 0 states left on queue`.
- `tlc -config verification/tla/IpcSyncEvidenceCap1.cfg verification/tla/IpcSyncEvidence.tla`: exit 0; `15781 states generated, 2997 distinct states found, 0 states left on queue`.
- `verus verification/verus/ipc_strict_admission.rs`: exit 0; `5 verified, 0 errors`.
- `verus verification/verus/ipc_capacity_bounds.rs`: exit 0; `6 verified, 0 errors`.
- `verus verification/verus/ipc_runtime_transitions.rs`: exit 0; `7 verified, 0 errors`.

## Blocked Obligations Routing

The following obligations are explicitly narrowed out with owner_state routing:
- REFINE-IPC-001..005: owner=State5/State8 (production refinement)
- LOOM-IPC-002..005: owner=State8 (loom compile fix)
- PROP-IPC-006: owner=State8 (slow-client test oracle)
- SCAN-IPC-007/008: owner=State10 (exhaustive classification)
- BLOCK-TLA-LIVENESS: owner=State5 (temporal liveness)
- GATE-IPC-009: owner=State11 (moon ci gate)
- WAIVE-VERUS-008: owner=State3 (waiver)

## Non-Blocking Positive Evidence

- Bounded TLA+ safety/enabledness checks pass for capacity 2 and capacity 1.
- Pure Verus witnesses pass for strict admission, capacity arithmetic, and runtime transition predicates.
- The restructured contract and traceability explicitly map known blockers with owner routing instead of hiding them.
- Canonical obligation shape now satisfies the "only planned rows" LETHAL rule.

## Required Route

Advance to State 7 for test planning. Blocked obligations are routed to downstream states with explicit owner_state metadata; they do not block proof approval but represent disclosed planning debt for downstream resolution.
