# Architecture Decision Records

This directory contains the architecture decision set derived from `velvet-ballistics-MASTER.md`.

The master document remains the source of truth. An ADR that conflicts with the master is wrong and must be corrected.

## Contents

| File | Purpose |
|------|---------|
| `v1/` | Current `velvet-ballistics/v1` ADRs for the Backend / IR Interpreter Complete milestone. |
| `ADR_DEPENDENCY_GRAPH.md` | Dependency ordering across the ADR set. |
| `ADR_FREEZE_AUDIT.md` | Contradictions, sharp edges, and freeze status. |
| `ADR-TO-VERIFICATION-TRACEABILITY-MATRIX.md` | Mapping from ADRs to master sections, docs, evidence, and known gaps. |
| `ADR_REVIEW_GATES.md` | Required review checks before new ADR changes land. |

## Freeze Set

The current implementation-critical freeze set is:

```text
ADR-001, ADR-002, ADR-003, ADR-004, ADR-005, ADR-006,
ADR-007, ADR-008, ADR-009, ADR-010, ADR-011, ADR-012,
ADR-013, ADR-014, ADR-015, ADR-016, ADR-018, ADR-019,
ADR-020, ADR-022, ADR-023, ADR-024
```

Guardrail ADRs:

```text
ADR-017, ADR-021
```

Guardrail means the decision prevents scope drift, but it does not add new current-scope implementation work by itself.

## Rules

1. ADR acceptance is architecture acceptance only; it is not implementation proof.
2. Implementation status must be proven by command evidence, tests, proofs, recovery logs, and benchmark artifacts.
3. New ADRs must update the dependency graph, freeze audit, traceability matrix, and review gates when applicable.
4. Deferred codegen, maxperf, PGO, native UI, distributed execution, HTTP core, and JSON core need explicit future ADRs before reactivation.
