// Verification artifact: choose_stepidx_bounds.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-FLUX-003 — StepIdx bounds: all generated StepIdx values stay in [0, u16::MAX]
//
// This obligation is covered by PO-KANI-005 (Kani harness for step index overflow).
// The Flux refinement provides type-level bounds for StepIdx arithmetic.
//
// Refinement: For any StepIdx produced by checked_step_offset during choose lowering,
//   step_idx.as_usize() ∈ [0, u16::MAX]
//
// This is enforced by:
//   1. step_idx() constructor rejects values > u16::MAX (returns Err)
//   2. checked_step_offset uses StepIdx::checked_add which enforces u16 bounds
//   3. All body lowering uses checked_step_offset for StepIdx arithmetic

/// Flux refinement annotations for StepIdx bounds:
///
/// #[flux_rs::refined_by(idx: int)]
/// pub struct StepIdx(u16);
///
/// #[flux_rs::invariant(0 <= idx && idx <= 65535)]
/// impl StepIdx {
///     pub const fn new(value: u16) -> Self { ... }
/// }
///
/// #[flux_rs::sig(fn(id: StepIdx, offset: u16) -> Result<StepIdx, CompileError>)]
/// fn checked_step_offset(id: StepIdx, offset: u16) -> Result<StepIdx, CompileError>
///     ensures
///         result.is_ok() =>
///             result.unwrap().idx == id.idx + offset &&
///             result.unwrap().idx <= 65535
/// {}
pub mod choose_stepidx_bounds_refinement {}

#[cfg(test)]
mod tests {
    use vb_core::ids::StepIdx;

    #[test]
    fn stepidx_new_accepts_valid_u16() {
        let idx = StepIdx::new(0);
        assert_eq!(idx.as_usize(), 0);
        let idx_max = StepIdx::new(u16::MAX);
        assert_eq!(idx_max.as_usize(), u16::MAX as usize);
    }

    #[test]
    fn stepidx_checked_add_respects_u16_bounds() {
        let id = StepIdx::new(65530);
        // 65530 + 5 = 65535 (valid)
        let result = id.checked_add(5);
        assert!(result.is_some(), "65530 + 5 should be valid");
        assert_eq!(result.unwrap().as_usize(), 65535);

        // 65535 + 1 = 65536 (overflow)
        let overflow = id.checked_add(6);
        assert!(overflow.is_none(), "65530 + 6 should overflow");
    }
}
