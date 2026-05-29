# Proof Strategy — vb-7m21 State 4 Replan (Reduced Scope)

## Provenance

- **Original planner invocation**: `proof-planner-vb-7m21-state4-001` (39 obligations, rejected at State 6)
- **Replan reviewer invocation**: `proof-plan-reviewer-vb-7m21-state4-replan-001` (APPROVED reduced scope)
- **This planner invocation**: `proof-planner-vb-7m21-state4-replan-001`
- **Scope reduction root cause**: Original plan over-scoped for a test-first bead. Default Rust behavior profile (Verus + Kani + Flux + proptest + TLA+) was applied to a bead whose primary deliverable is a test fixture corpus file (`crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs`). Verus, Flux, and TLA+ require production implementation targets that do not exist in this delivery scope.

## Bead Classification

This is a **test-first bead**. The bead `vb-7m21` title is "storage: Add blackhat corruption fixture corpus". Per `codebase-map.md:7-8` and `contract.md:26-27`, the primary deliverable is a test file — `crates/workspace_tests/tests/restate_storage_blackhat_fixture_corpus.rs` — with minimal implementation changes only if red tests require them. The bead does not introduce new production implementation functions, new behavior-affecting Rust code for refinement annotations, temporal/distributed protocol behaviors, or new concurrency/unsafe/FFI surfaces.

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
- proof-plan-review.md (reduced-scope approval)
- proof-plan-repair-guide.md
- verifier-lane-review.jsonl
- agent-invocation-ledger.jsonl

## Risk Classification

- **Temporal/state-machine**: Not applicable for this test-first bead. Journal gaps, duplicate event lifecycle, stale snapshot recovery, manifest/keyspace parity, and side-index parity are observable through deterministic behavior test assertions on public APIs, not through TLA+ model checking.
- **Rust-local invariant**: Header validation order, payload bound before allocation, schema classification, typed outcome classification — covered by Kani bounded model checking on codec-boundary seeds.
- **Bounded state**: Short buffers, finite schema versions — covered by Kani proptest fixture generation and exact typed outcome assertions.
- **Refinement/type-state**: Not applicable. No new production implementation functions exist to annotate with Flux refinement types.
- **Concurrency**: Not applicable. Corpus is local deterministic synchronous test infrastructure per `boundary-map.md:36-39`.
- **Unsafe/UB**: Not applicable. Unsafe/FFI/raw pointer work is out of scope and forbidden per `boundary-map.md:41-44`.
- **Untrusted input**: Binary envelope/header/payload corruption on codec-boundary seeds PS-001/PS-002/PS-003 — covered by cargo-fuzz smoke runs.
- **Dependency/supply-chain/provenance**: REQ-16 no-copy fence is a review/bridge obligation, not a verifier proof. All 8 verifier lanes for PS-009 marked not_applicable.
- **Performance/resource**: Allocation-before-bound-check for oversized declared payload — covered by Kani bounded model checking.
- **Release-critical gates**: Nextest corpus command and `moon ci` remain downstream execution gates.

## Reduced-Scope Lane Profile

| Seed | REQ | TLA+ | Verus | Kani | Flux | Loom | Miri | Proptest | Fuzz |
|------|-----|------|-------|------|------|------|------|----------|------|
| PS-001 | REQ-5 Oversized | NA | NA | **REQ** | NA | NA | NA | **REQ** | **REQ** |
| PS-002 | REQ-3 Schema | NA | NA | **REQ** | NA | NA | NA | **REQ** | **REQ** |
| PS-003 | REQ-6 Truncated | NA | NA | **REQ** | NA | NA | NA | **REQ** | **REQ** |
| PS-004 | REQ-4 Side-Index | NA | NA | NA | NA | NA | NA | **REQ** | NA |
| PS-005 | REQ-8 Journal Gap | NA | NA | NA | NA | NA | NA | **REQ** | NA |
| PS-006 | REQ-9 Duplicate | NA | NA | NA | NA | NA | NA | **REQ** | NA |
| PS-007 | REQ-10 Stale Snap | NA | NA | NA | NA | NA | NA | **REQ** | NA |
| PS-008 | REQ-11 Manifest | NA | NA | NA | NA | NA | NA | **REQ** | NA |
| PS-009 | REQ-16 No-Copy | NA | NA | NA | NA | NA | NA | NA | NA |

REQ = required. NA = not_applicable (all NA rows cite concrete evidence in verifier-lane-decisions.jsonl).

## Lane Summary

- **Core lane decisions**: 72 rows = 9 proof seeds × 8 verifiers.
- **Required proof obligations**: 14 (3 Kani + 8 proptest + 3 cargo-fuzz).
- **Not-applicable lane decisions with evidence**: 58.
- **Blocked tooling rows**: 0.

## Strategy

1. **Kani (3 obligations)**: Bounded codec-boundary seeds PS-001/PS-002/PS-003 for panic-freedom, typed outcome classification, and error precedence over RECORD_HEADER_BYTES frames. Use `kani::Arbitrary`/`kani::any()` generators; no hardcoded structure-only proofs.
2. **Proptest (8 obligations)**: All 8 behavior-affecting seeds (PS-001 through PS-008) require proptest obligations for deterministic fixture generation with exact typed outcome assertions in `restate_storage_blackhat_fixture_corpus.rs`. This is the primary verification mechanism for a test-first bead.
3. **Cargo-fuzz (3 obligations)**: Binary envelope hostile input surfaces for codec-boundary seeds PS-001/PS-002/PS-003 only. 60-second smoke runs with VB-derived seed corpora.
4. **Excluded verifiers**:
   - **Verus**: Not applicable — no production implementation in scope until State 11; no exec/spec binding targets exist. Evidence: `contract.md:26-27`, `codebase-map.md:7-8`, `proof-seeds.jsonl` suggested_layers exclude verus.
   - **Flux**: Not applicable — no new behavior-affecting Rust code to annotate with refinement types. Evidence: `contract.md:26-27`, `proof-seeds.jsonl` suggested_layers exclude flux-rs.
   - **TLA+**: Not applicable — no temporal protocol, retry, lease, lifecycle, distributed, or interleaving behavior. Evidence: `boundary-map.md:36-39` local deterministic synchronous corpus.
   - **Loom**: Not applicable — no implementation concurrency, cancellation, shutdown, atomics, locks, wakers, or interleaving risk. Evidence: `boundary-map.md:36-39`.
   - **Miri**: Not applicable — no unsafe, FFI, raw pointer, aliasing, provenance, or first-party unsafe. Evidence: `boundary-map.md:41-44`.

## Non-Behavior Waiver Candidate

One non-behavior candidate (`WC-vb-7m21-001`) remains valid for external Restate source/layout comparison unavailability. It does not waive any behavior, fixture outcome, or no-copy VB-provenance requirement. The reduced scope does not change this assessment.

## Planned Obligation Summary

| ID | Verifier | Seed | Risk |
|----|----------|------|------|
| PO-vb-7m21-kani-001 | kani | PS-001/REQ-5 | codec panic-freedom for oversized payload |
| PO-vb-7m21-kani-002 | kani | PS-002/REQ-3 | header validation for unknown schema version |
| PO-vb-7m21-kani-003 | kani | PS-003/REQ-6 | payload bounds for truncated header |
| PO-vb-7m21-prop-001 | proptest | PS-001/REQ-5 | fixture gen: oversized payload |
| PO-vb-7m21-prop-002 | proptest | PS-002/REQ-3 | fixture gen: unknown schema |
| PO-vb-7m21-prop-003 | proptest | PS-003/REQ-6 | fixture gen: truncated header |
| PO-vb-7m21-prop-004 | proptest | PS-004/REQ-4 | fixture gen: missing side-index |
| PO-vb-7m21-prop-005 | proptest | PS-005/REQ-8 | fixture gen: journal gap |
| PO-vb-7m21-prop-006 | proptest | PS-006/REQ-9 | fixture gen: duplicate event |
| PO-vb-7m21-prop-007 | proptest | PS-007/REQ-10 | fixture gen: stale snapshot |
| PO-vb-7m21-prop-008 | proptest | PS-008/REQ-11 | fixture gen: missing manifest |
| PO-vb-7m21-fuzz-001 | cargo-fuzz | PS-001/REQ-5 | fuzz: envelope decode |
| PO-vb-7m21-fuzz-002 | cargo-fuzz | PS-002/REQ-3 | fuzz: header parse |
| PO-vb-7m21-fuzz-003 | cargo-fuzz | PS-003/REQ-6 | fuzz: payload decode |

Total: 14 required obligations.
