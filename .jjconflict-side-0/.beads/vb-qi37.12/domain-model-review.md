# Domain Model Review: vb-qi37.12

STATUS: READY_FOR_INDEPENDENT_REVIEW

## Reviewed Model
- Aggregate: `FallibleSite` classified by crate, path, API, operation kind, criticality, and discard classification.
- Core lattice: `must_propagate`, `must_accumulate`, `typed_optional`, `typed_best_effort_discard`, with `unclassified` forbidden for production release-critical paths.
- Error envelopes: storage, runtime, resume, compiler, and discard diagnostics must preserve cause and boundary metadata.

## Strengths
- Distinguishes destructor limitations from normal fallible APIs: `FjallJournal::drop` cannot return `Result`, so required persistence must move to an explicit close/persist or a typed best-effort contract.
- Distinguishes optional payload absence from corruption: `Option` accessors are acceptable only outside recovery-critical decode paths.
- Separates compiler accumulation from silent discard: `if let Err(e)` accumulation can be valid when all causes remain in `CompileErrors`.

## Risks For Next States
- `typed_best_effort_discard` can become a loophole unless mechanically inventoryable and barred from durability/recovery/validation-critical paths.
- Exact Verus and TLA+ target files are not created in this State 3 scope; State 4 must bind these contracts to executable proof targets or explicit approved waivers.
- `RuntimeEngineResult` cause preservation needs a concrete runtime error envelope; otherwise `Err(_) => terminal failed` remains lossy.

## Required Reviewer Checks
- Verify every contract clause maps to a proof obligation and traceability row.
- Reject any later implementation that uses logging-only continuation for release-critical storage/runtime/compiler failures.
- Reject any proof plan that treats optional accessor behavior as sufficient evidence for recovery-critical corrupt payloads.

## Self-Approval Boundary
- This artifact is not an approval. The independent `contract-verification-reviewer` must produce `contract-verification-review.md` with APPROVED or REJECTED status before tests/proofs/implementation consume these contracts.
