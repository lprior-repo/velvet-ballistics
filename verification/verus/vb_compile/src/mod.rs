// Verus verification artifacts for vb_compile crate.
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
//
// This directory's previous contents
// (`choose_bool_invariant.rs`, `choose_depth_invariant.rs`) were
// VACUUM Verus files per `scripts/check-verus-production-binding.sh`
// (no production `#[path]` binding + no `assume_specification`
// bridge). They were deleted as part of bead vb-czg3q because:
//
//   - `choose_bool_invariant.rs` — vacuous `is_boolean_slot`
//     predicate returning `true` for all slots without exercising
//     any production code; cannot be bound without a full rewrite.
//
//   - `choose_depth_invariant.rs` — duplicates the FANOUT
//     invariant already discharged by
//     `verification/verus/vb_awhr_fanout_spec.rs` (which has a
//     proper WEAK companion extern binding to
//     `production_inner/lower_choose_fanout_production.rs`).
//
// No replacement modules are needed; the FANOUT, depth, and layout
// parity properties are covered by the Kani harnesses (PO-KANI-005,
// PO-KANI-008, PO-KANI-012) and proptest harnesses (PO-PROPTEST-001,
// PO-PROPTEST-004) in the production crate, plus the WEAK-bound
// Verus spec at `verification/verus/vb_awhr_fanout_spec.rs`.
