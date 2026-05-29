# Proof Writer Final Review Repair Report - vb-dybj State 5 attempt 7

## Scope

Delegate: proof-writer. Parent: femdation controller. Bead: `vb-dybj`. Isolated workdir: `/home/lewis/isolated/femdation-velvet-ballistics/vb-dybj`.

I edited proof/test/fuzz/Kani harness/evidence/provenance artifacts only. I did not edit production runtime behavior.

## Archived rejected State 6 review

- Archived active rejected State 6 attempt 3 artifacts to `.beads/vb-dybj/archive/state6-rejected-20260525-final-003/`.
- Active bead surface no longer contains `proof-review.md`, `proof-findings.jsonl`, or `transcript-state6-proof-reviewer.md`.

## Obligations touched

- PO-VB-DYBJ-012: reran storage-short cargo-fuzz at the planned bound `-max_total_time=60 -runs=10000`; passed.
- PO-VB-DYBJ-013: repaired the Kani trailing-byte harness to use an explicit exact/no-trailing Postcard boundary via `postcard::take_from_bytes`; Kani verified the bounded suffix model.
- PO-VB-DYBJ-014: repaired the executable proptest property to use the same explicit exact/no-trailing Postcard boundary; trailing-byte property now passes.
- PO-VB-DYBJ-015: repaired the fuzz target to use the exact/no-trailing boundary; fuzz smoke now passes.
- PO-VB-DYBJ-005, PO-VB-DYBJ-008, PO-VB-DYBJ-010: reran/retained honest blockers. Flux still cannot resolve `flux_rs`; selected vb_storage Kani lanes still require unrelated cfg(kani) compile repair before discharge.

## Raw command outcomes

- PASS: `rtk cargo test -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests trailing_bytes -- --nocapture` -> `cargo test: 1 passed, 8 filtered out`.
- PASS: `cargo kani -p velvet-ballistics-workspace-tests --harness kani_vb_dybj_trailing_bytes_rejected --output-format regular` -> `VERIFICATION:- SUCCESSFUL`; `0 of 238 failed`; Kani 0.67.0 / CBMC 6.8.0; unwind bound 9; suffix length 1..=8.
- PASS: `cargo fuzz run vb_dybj_trailing_decode --target x86_64-unknown-linux-gnu -- -max_total_time=10 -runs=1000` -> `#1000 DONE`, no crash.
- PASS: `cargo fuzz run vb_dybj_storage_short_decode --target x86_64-unknown-linux-gnu -- -max_total_time=60 -runs=10000` -> `#10000 DONE`, no crash.
- RECORDED_TOOLING_GAP: `cargo flux --manifest-path verification/flux/Cargo.toml` -> unresolved `flux_rs` and Flux attributes; PO-VB-DYBJ-005 not discharged.

## Blockers still routed to implementation/formal owner

1. PO-VB-DYBJ-005 needs a working Flux attribute crate/integration strategy or approved replanning/waiver.
2. PO-VB-DYBJ-008/010 need unrelated existing vb_storage cfg(kani) compile repairs or Kani target isolation outside proof-writer scope.
3. PO-VB-DYBJ-001/004/007 remain Verus standalone mapped model evidence, not mechanically production-bound proof; no stronger claim is made here.

No final proof success is claimed for unresolved obligations.
