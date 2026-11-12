# vb-kyyf blocker report

bead_id: vb-kyyf
phase: owner-authorized-unblock
attempt: 2
status: UNBLOCKED_FOR_REVIEW_WITH_RESIDUAL_EXTRACTION_CAVEAT

## Blocker status

`PO-009 / VERUS-KYYF-001` is no longer blocked on a fully detached standalone model. Attempt 2 makes `verification/verus/vb_kyyf_normalization.rs` include and verify the actual production-owned source file `crates/vb_proof_kernels/src/vb_kyyf_normalization.rs` as `production_probe`, then adds taxonomy obligations over the production-owned types.

Residual caveat: the cargo-compiled production functions still do not carry inline Verus `requires`/`ensures` contracts. Verus verifies the production source as an included module and checks taxonomy specs over those source-level types. If the reviewer requires literal contracts on the compiled function items, the remaining extraction path is: split the kernel into a shared Verus-compatible core source or cfg-gated Verus crate target consumed by both Cargo and Verus.

## Raw artifact evidence retained from attempt 7

Source artifact: `.beads/vb-kyyf/proof-writer-report.md`

- State/attempt: `State 5 / Attempt 7`.
- Verdict: `FINAL_PARTIAL`.
- TLA obligation `PO-008 / TLA-KYYF-001`: `PASS_TLC_WITH_LOCAL_TEMP_WORKAROUND` with TLC complete, no errors, `42,907,696 states generated`, `16,483,704 distinct states found`, depth `9`.
- Verus obligation `PO-009 / VERUS-KYYF-001`: `BLOCK_LOCAL/VERUS_NOT_IMPLEMENTATION_BOUND`.
- Verus command evidence: `verus verification/verus/vb_kyyf_normalization.rs` produced `verification results:: 12 verified, 0 errors`, but the report states the artifact is standalone spec/proof functions and not bound to the executable normalization/comparison kernel.

## Routing

Cleared for femdation/controller proof-review decision on this sublane. Do not treat this as full release closure; public adapter projection tests and full gate remain outside this one owner-authorized unblock lane.

## Workspace preservation

Preserve `/home/lewis/src/bd-vb-kyyf-bdd` as evidence because attempt 7 failure blocks landing.

## Owner-authorized unblock evidence

- Production seam: `crates/vb_proof_kernels/src/vb_kyyf_normalization.rs`.
- Module export: `crates/vb_proof_kernels/src/lib.rs`.
- Verus seam: `verification/verus/vb_kyyf_normalization.rs` now includes the production-owned source file as a Verus module and proves cold metadata normalization/taxonomy obligations over `production_probe` types.
- Trusted projection boundary: actual public replay/runtime/codegen surfaces project concrete observations into scalar semantic signatures before invoking the pure kernel. The boundary is documented in `.beads/vb-kyyf/proof-architecture-report.md`.
- Exact failure taxonomy preserved in the seam: `NondeterministicObservation`, `ReplayDigestMismatch`, `ReplaySequenceViolation`, `ReplayPolicyBlocked`, `GeneratedIrDivergence`, and `UnsupportedGeneratedSubset`.

## Remaining status

Cleared for femdation/controller review: `VERUS_SOURCE_INCLUDED_PRODUCTION_TYPES_BOUND`; residual caveat `INLINE_REQUIRES_ENSURES_NOT_ON_CARGO_ITEMS`.

Remaining release obligations are outside this owner-authorized unblock sublane: full public-surface BDD wiring, scoped storage/runtime/codegen tests, and canonical `moon ci` release gate.
