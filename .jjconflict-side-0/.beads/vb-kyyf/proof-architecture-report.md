# vb-kyyf proof architecture report

## Status

Owner-authorized unblock attempt 2 for `VERUS-KYYF-001`: `UNBLOCKED_FOR_REVIEW_WITH_RESIDUAL_EXTRACTION_CAVEAT`.

## Production-owned seam

Added `crates/vb_proof_kernels/src/vb_kyyf_normalization.rs` and exported it through `crates/vb_proof_kernels/src/lib.rs`.

The seam is pure, sequential, safe Rust and owns these decisions:

- normalize allowed cold metadata away;
- compare semantic replay/cross-run observations exactly;
- preserve replay failure taxonomy;
- preserve generated/IR failure taxonomy.

## Concrete mapping to replay path

The concrete public surfaces remain the trusted projection boundary:

- `FjallJournal::events_for_run`
- `recover_full_journal`
- `recover_runtime_summary`
- `recover_runtime_frame_seed`
- `verify_digests`
- `Runtime::submit_compiled_with_inputs`
- `Runtime::inspect_run`
- `compare_generated_to_ir`
- `validate_generated_subset`

Those surfaces project observations into `PublicObservation` scalar signatures. The pure kernel then compares `NormalizedObservation` values. Verus verifies the scalar projection seam, not Fjall I/O, concrete hashing, CLI execution, filesystem metadata capture, or source generation.

## Trusted projection boundary

Trusted inputs:

- event kind/order signature;
- significant payload signature;
- semantic slot/action/taint signatures;
- digest booleans from replay digest checks;
- replay policy blocked flag;
- unsupported generated subset flag;
- cold metadata signatures for temp path, process id, wall-clock timestamp, and generated run id.

Only the four cold metadata signatures are normalized away. All semantic signatures and status fields are compared exactly.

## Verus binding

`verification/verus/vb_kyyf_normalization.rs` now imports the production-owned source file inside the Verus proof crate:

```rust
#[path = "../../crates/vb_proof_kernels/src/vb_kyyf_normalization.rs"]
mod production_probe;
```

That makes Verus parse and verify the actual `crates/vb_proof_kernels/src/vb_kyyf_normalization.rs` source as part of the proof artifact instead of proving only a detached hand mirror. The production source was adjusted to be Verus-checkable: no `Debug`/`PartialEq` derive expansion is required by the seam, and normalized equality is expressed through explicit scalar comparison helpers.

Attempt 2 also adds checked Verus taxonomy obligations over the production-owned types for `compare_cross_run`, `compare_replay`, and `compare_generated_ir` semantics:

- cold metadata drift normalizes away;
- semantic normalized mismatch maps to `NondeterministicObservation`;
- replay digest mismatch precedes policy and sequence errors;
- replay policy blocked precedes sequence errors;
- replay sequence mismatch maps to `ReplaySequenceViolation`;
- generated unsupported subset precedes divergence;
- generated/IR normalized mismatch maps to `GeneratedIrDivergence`.

Verus command evidence:

```text
verus verification/verus/vb_kyyf_normalization.rs
verification results:: 37 verified, 0 errors
```

Trust scan evidence: no `assume`, `external_body`, `external`, or `axiom` matches in `verification/verus` or the production proof seam.

## Remaining limits

This unblock does not certify full public-surface extraction. The next state must wire BDD/test evidence to prove the adapters project concrete runtime/storage/CLI observations into this seam correctly.

Residual extraction caveat: Verus verifies the production source by including it as a module and proves taxonomy specs over the production-owned types. The cargo-compiled functions themselves still do not carry inline Verus `requires`/`ensures`, because raw Verus contract syntax cannot be inserted into the production item definitions without a dedicated cfg/extraction layout. A future hardening path is to split the seam into a shared Verus-compatible core source generated/consumed by both Cargo and Verus, or to introduce a cfg-gated Verus crate target that owns the annotated function definitions directly.
