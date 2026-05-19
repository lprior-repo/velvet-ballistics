# vb-njju Domain Model Review

## Scope reviewed

- Acceptance-catalog domain: executable BDD scenarios as release evidence.
- Quality-gate domain: mutation, fuzz smoke, property parity, unsafe-boundary fuzz, and release blocking.
- Reviewed inputs: `codebase-map.md`, `delivery-scope.jsonl`, manifest, rust-contract startup files.

## Ubiquitous language check

- `Scenario` is the catalog row, not the test implementation.
- `Evidence` is executable, scoped, and exact; prose-only or build-only evidence is weak evidence.
- `Release gate` is fail-closed: missing required local evidence is a release failure, not a warning.
- `Taint parity` is part of generated-vs-IR semantic equality.
- `Unsafe boundary` covers unsafe/decoder/binary boundary surfaces discovered by boundary inventory and fuzz targets.

## Aggregate boundaries

- `AcceptanceCatalog` aggregate: owns Given/When/Then completeness, public-surface requirement, fixture isolation, expected outcome/error, related bead, and evidence target.
- `MutationGateEvidence` aggregate: owns target scope, admission-branch inclusion, kill/survivor disposition, and blocker/follow-up mapping.
- `FuzzSmokeEvidence` aggregate: owns required target names and run/seed invocation evidence. It must distinguish build evidence from run evidence.
- `GeneratedIrParityEvidence` aggregate: owns equality dimensions: result, taint, journal/event signature, typed errors, slots/signals.
- `BoundaryReleaseEvidence` aggregate: owns boundary inventory rows, required fuzz coverage, manual QA/follow-up exceptions, and release-blocking status.

## Illegal states that must be unrepresentable or rejected

- vb-njju acceptance row without Given/When/Then.
- Mutation closure for admission branch satisfied by unrelated `vb_core/src/diagnostic.rs` mutation smoke.
- Fuzz-smoke closure satisfied by `cargo fuzz build` only.
- Generated-vs-IR equality that ignores taint.
- Release success while any required unsafe/decoder/binary boundary has no fuzz evidence or approved blocker/follow-up.

## Public-surface rule

- State 4 must prefer `velvet_ballastics_workspace_tests::acceptance_catalog::{Scenario,catalog,validate_catalog}` and public quality validation APIs listed in `delivery-scope.jsonl`.
- If a missing public quality API blocks exact evidence, State 4 should add a workspace-test-facing public helper rather than couple tests to private module internals.

## Review result

- Domain model is adequate for contract-first State 3.
- Highest-risk concept gap: existing `fuzz-smoke` is build-only per codebase map; State 4 must make run/seed semantics explicit or fail closed.
