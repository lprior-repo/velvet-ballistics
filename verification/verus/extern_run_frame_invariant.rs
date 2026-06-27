// SPDX-License-Identifier: MIT
//
// ============================================================================
// EXTERN WRAPPER for run_frame_invariant.rs Verus spec
// ============================================================================
//
// Companion extern file referenced by `run_frame_invariant.rs` via
// `#[path = "extern_run_frame_invariant.rs"] mod production;`. The actual
// production mirror content (types + exec wrappers) lives in the
// `production_inner/run_frame_invariant_production.rs` file. This
// wrapper `#[path]`-includes that mirror and re-exports its items under
// the `production::*` namespace so the spec file's
// `pub use production::{...}` resolution continues to work without
// modification.
//
// This WEAK-binding pattern satisfies
// `scripts/check-verus-production-binding.sh`:
//
//   1. The spec file references this extern via `#[path = "extern_*"]`
//      and uses `assume_specification` (already present).
//   2. THIS extern file has `#[path = "production_inner/..."]` which is
//      the binding-gate WEAK marker.
//
// Production mirror binding ledger (pointing to production source files):
//   - `RunFrame`                            <- crates/vb_core/src/frame.rs:65-78
//   - `StepState`                           <- crates/vb_core/src/frame.rs:10-29
//   - `is_valid_step_state_transition`      <- crates/vb_core/src/frame.rs:33-63
//   - `RunFrame::new`                       <- crates/vb_core/src/frame.rs:82-110
//   - `RunFrame::reinitialize`              <- crates/vb_core/src/frame.rs:113-150
//   - `RunFrame::pc`                        <- crates/vb_core/src/frame.rs:158-162
//   - `RunFrame::step_count`                <- crates/vb_core/src/frame.rs:170-174
//   - `RunFrame::slot_count`                <- crates/vb_core/src/frame.rs:176-180
//   - `RunFrame::set_pc`                    <- crates/vb_core/src/frame.rs:226-234
//   - `RunFrame::write_slot_with_taint`     <- crates/vb_core/src/frame.rs:264-280
//   - `RunFrame::states_snapshot`           <- crates/vb_core/src/frame.rs:304-308
//   - `RunFrame::slots_snapshot`            <- crates/vb_core/src/frame.rs:292-296
//   - `RunFrame::taint_snapshot`            <- crates/vb_core/src/frame.rs:298-302
//   - `Taint` discriminant set              <- crates/vb_core/src/value.rs:14-25
//   - `SlotValue` discriminant set          <- crates/vb_core/src/value.rs:125-142
//   - `CoreError` relevant variants         <- crates/vb_core/src/errors.rs
//   - `RunId`, `StepIdx`, `SlotIdx`         <- crates/vb_core/src/ids/mod.rs
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in the production mirror are NOT
// verified by Verus. Each exec fn is `#[verifier::external]` so Verus
// skips body verification, and the contracts attached via
// `assume_specification` in the companion spec file state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt item
// outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

#[path = "production_inner/run_frame_invariant_production.rs"]
pub mod run_frame_invariant_production;

pub use run_frame_invariant_production::{
    CoreError, CoreResult, RunFrame, RunId, SlotIdx, SlotValue, StepIdx,
    StepState, Taint, is_valid_step_state_transition, run_frame_new,
    run_frame_pc, run_frame_reinitialize, run_frame_set_pc,
    run_frame_slot_count, run_frame_slots_snapshot, run_frame_states_snapshot,
    run_frame_step_count, run_frame_taint_snapshot,
    run_frame_write_slot_with_taint,
};

} // verus!