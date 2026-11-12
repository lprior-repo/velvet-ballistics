# vb-kyyf Domain Model Review

## Scope Reviewed
- Cross-run determinism and reproducibility BDD acceptance contracts.
- Public surfaces: storage recovery/replay, runtime deterministic drive/inspect, CLI replay/events/inspect, generated-vs-IR parity validation.
- Source context: codebase map, delivery scope, bead JSON from `.beads/vb-kyyf/STATE.md`, baseline report, and master lines cited in `contract.md`.

## Domain Model Verdict
The domain boundary is correct only if the acceptance scenarios model persisted operational truth: accepted artifact digest, binary input, journal/snapshot evidence, action replay policy, normalized public reports, and generated/IR execution observations. YAML source and private helper calls are not domain truth.

## Aggregate Boundaries
- `AcceptedArtifact`: immutable workflow identity plus verification/admission evidence.
- `RunEvidence`: run header, journal events, snapshots, blobs, and indexes bound to one compiled workflow digest.
- `ReplayReport`: normalized public observation of persisted evidence through storage/runtime/CLI surfaces.
- `GeneratedParityEvidence`: paired IR/generated observations for a generated-subset-supported workflow.
- `ScenarioEvidenceRow`: BDD metadata, public surface, evidence artifact path, normalized digest, and mismatch detail.

## Illegal States to Reject
- Scenario passes with no evidence artifact.
- Scenario uses a private helper as the primary behavior path.
- Normalizer hides semantic differences such as event kind, event order, terminal result, taint, digest failure, suspension state, or typed error.
- Replay continues after digest mismatch, corrupt record, sequence gap, or duplicate conflicting completion.
- Recovery re-executes a non-replay-safe side effect after a scheduled journal record.
- Generated parity is claimed for unsupported IR families.

## Determinism Boundary
- Deterministic semantics: terminal result, taint, journal event kind/order/significant payload digest, suspension state, typed errors, digest-check outcome, and side-effect dispatch count.
- Allowed cold nondeterminism: temp path, process id, wall-clock timestamp, and generated run id only when explicitly normalized and recorded.
- Forbidden normalization: any domain value, taint, error variant, digest, action ticket/key, event kind, event sequence, or replay policy outcome.

## Public-Surface Boundary
- Strong evidence surfaces: CLI `replay/events/inspect`, `FjallJournal::events_for_run`, recovery functions, `Runtime::submit_compiled_with_inputs`, `Runtime::inspect_run`, and generated-subset/codegen validation functions.
- Weak/non-evidence surfaces: raw internal structs not reachable through public APIs, source-pattern-only generated comparison without execution, prose catalog rows without runnable scenario targets.

## Release Risks
- Generated-vs-IR parity remains high risk because current `compare_generated_to_ir` is not full semantic execution equivalence.
- CLI replay may lag storage/runtime APIs; contract requires stable typed gap evidence if unavailable.
- Pending-action hydration and digest verification gaps are known release risks and must be represented as deterministic typed outcomes, not ignored.

## Required Design Shape for Later States
- Model observations as typed values before comparison.
- Make normalization explicit and auditable.
- Record both raw and normalized evidence paths when possible.
- Link every scenario to master clauses and `vb-kyyf` in the acceptance catalog.
- Keep fixtures isolated per scenario to prevent cross-test state bleed.
