# ADR 022 (v1): ADR Governance

## Status

Accepted as architecture baseline.

## Decision

The ADR set is a reviewable decomposition of the master contract. It must never become an independent source of truth.

## Invariants

- Master conflict means ADR defect.
- New ADRs update dependency graph, traceability matrix, freeze audit, and review gates.
- ADRs must distinguish decision status from implementation status.
- ADRs must identify deferred scope explicitly.
- ADR changes require bead tracking and review evidence.

## Consequences

- ADRs can guide agents without diluting master authority.
- ADR drift becomes a first-class defect.

## Master Anchors

- Section 43: AI Agent Acceptance Contract
- Section 60: Evidence Artifact Format
- Section 77: AI-Safe Quality Infrastructure
