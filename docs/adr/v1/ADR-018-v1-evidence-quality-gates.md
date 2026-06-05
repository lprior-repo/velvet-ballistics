# ADR 018 (v1): Evidence and Quality Gates

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Architecture claims close only with raw evidence tied to the bead, source revision, command output, and production behavior. `moon ci` is the canonical full gate.

## Invariants

- Tool version checks are setup evidence only.
- Kani harnesses must not prove one hardcoded shape.
- Verus proofs must bind to production implementation behavior.
- TLA+ models must include bounded hardware limits where relevant.
- Proof failures fix implementation or contract defects; they do not get papered over.
- Evidence bundles map requirements to source, tests, proofs, commands, and residual gaps.

## Master Anchors

- Section 36: Mandatory Test Coverage
- Section 37: Fuzz Targets
- Section 38: Property Tests
- Section 39: Mandatory Benchmarks
- Section 40: CI Gate
- Section 43: AI Agent Acceptance Contract
- Section 60: Evidence Artifact Format
- Section 77: AI-Safe Quality Infrastructure
