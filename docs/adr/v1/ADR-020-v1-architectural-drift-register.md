# ADR 020 (v1): Architectural Drift Register

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Known architectural drift is tracked explicitly. Drift entries must state the defect, root cause, resolution contract, evidence, and remaining gaps.

## Invariants

- A resolved drift entry must cite evidence.
- A partially resolved drift entry remains a release risk.
- Drift must create or link follow-up beads when work remains.
- Docs that conflict with master are drift, not alternative truth.

## Current High-Risk Drift

- Crash recovery pending-action hydration and strict acknowledgement behavior.
- Existing docs using stale names or stale scope language.
- Existing docs describing current master requirements as future-only.

## Master Anchors

- Section 67: Architectural Drift Register
- Section 43: AI Agent Acceptance Contract
