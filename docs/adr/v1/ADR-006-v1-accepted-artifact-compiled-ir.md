# ADR 006 (v1): Accepted Artifact and Compiled IR

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Production runtime execution is bound to accepted artifacts. An accepted artifact wraps compiled IR with workflow digest, IR digest, action contract digest, resource budget, capability requirements, warnings, and verification proof metadata.

`CompiledWorkflow` is checked numeric IR. It is not trusted until construction validates references and artifact verification succeeds.

## Invariants

- Empty node arrays are rejected.
- Entry points, transition targets, slots, constants, expressions, accessors, and action IDs are range checked.
- Digest canonicalization is stable and Postcard-oriented.
- Raw `CompiledWorkflow` submission is internal/test only unless production policy explicitly allows it.

## Consequences

- Recovery can load by digest instead of reparsing source.
- Tests that bypass accepted artifacts are not production admission evidence.

## Master Anchors

- Section 14: Core Rust Types
- Section 15: Final IR Contract
- Section 51: Digest Canonicalization and Schema Versioning
- Section 63: Plan Verifier and Accepted Artifacts
