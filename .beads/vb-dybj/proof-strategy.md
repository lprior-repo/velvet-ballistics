# Proof Strategy - vb-dybj State 4

## Scope

Bead `vb-dybj` adds fixed-wire Postcard compatibility tests for selected VB newtypes and persisted record identifiers. This State 4 artifact plans proof and verification obligations only; it does not write tests, production code, models, harnesses, or reviewer dispositions.

Inputs read: `contract.md`, `proof-seeds.jsonl`, `traceability-matrix.jsonl`, `domain-model.md`, `type-contracts.md`, `workflow-model.md`, `error-taxonomy.md`, `boundary-map.md`, `hazard-analysis.md`, `delivery-scope.jsonl`, `codebase-map.md`, `state3-validation-evidence.json`, and dispatch manifest.

## Risk Classification

| Risk class | Present? | Evidence | Planning response |
|---|---:|---|---|
| Temporal/state-machine | Yes | `workflow-model.md` states fixture lifecycle and migration terminal states. | TLA+ required for seed 007 migration lifecycle. |
| Rust-local invariant | Yes | `RunId::new/get/ZERO`, `WorkflowDigest::from_bytes/as_bytes`, `RecordKind::id()` contracts. | Verus required for pure/type-level invariants where production functions can be bound. |
| Bounded state / panic / overflow / indexing | Yes | `u64::MAX`, 60-byte storage envelope, short inputs. | Kani required for RunId bounds and storage malformed decode ordering. |
| Refinement/type-state | Limited | Existing Rust types already make many illegal states unrepresentable; surface-name discipline is string/test governance. | Flux required only for digest shape if practical; otherwise explicit not-applicable rows. |
| Concurrency | No | Boundary map states async/network/time out of scope; bead creates compatibility tests only. | Loom not applicable for every seed. |
| Unsafe/UB | No first-party unsafe in scope | Boundary map forbids unsafe/FFI; no production changes planned. | Miri not applicable for every seed unless downstream adds unsafe, which is forbidden. |
| Untrusted input / parser / codec | Yes | Malformed trailing/missing byte seeds and storage decode boundary. | proptest and cargo-fuzz required for seeds 004 and 005; proptest required for golden roundtrip properties. |
| Dependency/supply-chain | Yes | No JSON/Bilrost/Protobuf dependency allowed. | Source/dependency scan obligation planned; core verifier lanes classify non-applicability where they cannot prove manifest policy. |
| Performance | No | Contract says speed/performance claims out of scope. | No benchmark proof lane. |
| Release-critical gates | Yes | Golden byte changes require named migration. | TLA+ lifecycle plus tests/mutation-sensitive obligations in bridge input. |

## Lane Strategy

- TLA+: model only the fixture governance lifecycle, especially `MigrationRequired` vs silent acceptance. It is not used for static value-object facts.
- Verus: bind pure Rust-local invariants to source targets: `RunId::new/get/ZERO`, `WorkflowDigest::from_bytes/as_bytes`, and `RecordKind::id()`/surface distinction obligations. No vacuum proofs: any Verus artifact must refer to production functions or wrapper specs with explicit source refs.
- Kani: bounded arbitrary harnesses for `RunId` selected values, storage short input boundaries, and trailing decode rejection. No hardcoded-only shapes; use `kani::Arbitrary` or safe exhaustive generators for relevant bounded structures.
- Flux RS: practical only for shape/refinement checks around exact digest length if a downstream annotation lane is available. Other seeds are not refinement-type problems.
- Loom: no implementation concurrency, shared memory, task cancellation, or interleaving risk exists in this bead.
- Miri: no unsafe, FFI, aliasing, provenance, or layout-sensitive unsafe code is in scope; if downstream introduces unsafe, this plan becomes stale and must be rejected.
- proptest: required for codec roundtrips, malformed input classes, and surface-name/golden governance properties where executable tests can vary inputs.
- cargo-fuzz: required for hostile external byte slices at raw Postcard/storage decode surfaces; not required for pure fixed constants with no parser boundary.
- source-scan: non-core obligation for forbidden dependency/path tokens in touched test/manifest files.

## Planned Evidence Posture

Every required obligation is `status: planned`. No proof success is claimed. Expected raw evidence includes exact command, workdir, tool version, bounds/seeds, exit status, and log artifact in later formal execution states.

## Non-Goals

- No production code edits.
- No test implementation.
- No proof/model/harness implementation.
- No reviewer approvals (`proof-plan-review.md` and `verifier-lane-review.jsonl` are intentionally not written).
