# Proof Strategy — vb-7m21 State 4

## Scope
Plan proof coverage for the deterministic `vb_storage` blackhat corruption fixture corpus. State 4 writes planning artifacts only; no proof/model/harness/test/production code is written here.

## Inputs Read
- contract.md
- proof-seeds.jsonl
- traceability-matrix.jsonl
- delivery-scope.jsonl
- domain-model.md
- type-contracts.md
- workflow-model.md
- error-taxonomy.md
- boundary-map.md
- hazard-analysis.md
- codebase-map.md
- state3-validation-evidence.json

## Risk Classification
- Temporal/state-machine: journal gaps, duplicate event lifecycle, stale snapshot recovery, manifest/keyspace parity, side-index parity.
- Rust-local invariant: header validation order, payload bound before allocation, schema classification, typed outcome classification.
- Bounded state: short buffers, finite schema versions, finite keyspace/index states, bounded sequence gaps.
- Refinement/type-state: payload length <= family max, schema version states, sequence successor, closed expected outcome families.
- Concurrency: not applicable; corpus is local deterministic synchronous test infrastructure.
- Unsafe/UB: not applicable; unsafe/FFI/raw pointer work is out of scope and forbidden.
- Untrusted input: binary envelope/header/payload corruption requires proptest and cargo-fuzz where codec input is present.
- Dependency/supply-chain/provenance: REQ-16 no-copy fence is planned via review/bridge obligations, with verifier lanes marked not applicable where mathematical tools cannot prove provenance.
- Performance/resource: allocation-before-bound-check for oversized declared payload.
- Release-critical gates: nextest corpus command and `moon ci` remain downstream execution gates.

## Lane Summary
- Core lane decisions: 72 rows = 9 proof seeds × 8 verifiers.
- Required proof obligations: 39.
- Not-applicable lane decisions with evidence: 33.
- Blocked tooling rows: 0.

## Strategy
1. Use TLA+ only for persistence/recovery lifecycle state where temporal ordering matters.
2. Use Verus as the Rust-core spine for classifier/order/bounds invariants and require exec/spec binding to production targets.
3. Use Kani for bounded panic-freedom and exact typed classification over generated structures; hardcoded dummy structures are forbidden.
4. Use Flux where illegal states are representable as simple refinements over lengths, versions, sequences, keyspace membership, or typed outcomes.
5. Use proptest for deterministic fixture generation and typed outcome coverage.
6. Use cargo-fuzz for hostile byte codec/envelope surfaces only.
7. Mark Loom and Miri not applicable with concrete evidence unless downstream introduces concurrency or unsafe, which would invalidate this plan and require replanning.

## Non-Behavior Waiver Candidate
One non-behavior candidate is recorded for external Restate source/layout comparison unavailability. It does not waive any behavior, fixture outcome, or no-copy VB-provenance requirement.
