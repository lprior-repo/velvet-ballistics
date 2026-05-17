bead_id: vb-qi37.4.4
bead_title: runtime: Add admission durability errors
phase: State 3 - contract
updated_at: 2026-05-11T00:00:00Z

# Contract Specification

## Preconditions
- PRE-001: Runtime admission failures have a lossless semantic cause.
- PRE-002: Storage/header persistence failures during admission are distinguishable from post-admission runtime failures.

## Postconditions
- POST-001: Every admission rejection maps to a stable `RuntimeError` variant and diagnostic code.
- POST-002: API/CLI/IPC envelopes can expose the stable code without string parsing.

## Invariants
- INV-001: `Display`, `diagnostic_code`, `runtime_code`, `PartialEq`, and `Error::source` do not erase admission durability cause.

## Error Taxonomy
- ERR-artifact-not-found: accepted artifact digest is absent from the artifact store.
- ERR-artifact-invalid-stale-digest-mismatch: accepted artifact envelope is invalid, stale, failed gate proof, or has digest mismatch.
- ERR-capability-denied: accepted artifact lacks a capability required by the workflow.
- ERR-idempotency-duplicate: duplicate run id/idempotency gate prevents a second admission.
- ERR-header-persistence-failed: durable run header/admission persistence fails before acknowledgement.

## Non-goals
- No public dependency additions.
