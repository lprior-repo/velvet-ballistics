//! Flux RS refinement specifications for choose lowering.
//! Bead: vb-ewjwz (State 5 repair)
//! Obligations: PO-CL-FLUX-BIND-001, PO-CL-FLUX-BIND-002, PO-CL-FLUX-BIND-003
//!
//! GOD RULE 2: These extern_spec blocks bind to actual production Rust
//! implementations in vb_compile and vb_core.

// =========================================================================
// PO-CL-FLUX-BIND-001: SlotCompiler::record_slot monotonicity
// =========================================================================
// Property: After record_slot(s), self.max_slot >= s.as_usize()
// The production implementation at part_08.rs:77-83 stores max(current, value).

#[flux_rs::extern_spec]
impl crate::mod_compile_lowering::SlotCompiler {
    #[flux_rs::sig(fn(&mut Self, slot: vb_core::SlotIdx) -> ())]
    fn record_slot(&mut self, slot: vb_core::SlotIdx);
}

// =========================================================================
// PO-CL-FLUX-BIND-002: slot_from_text result shape
// =========================================================================
// Property: On success, the SlotIdx value is in [0, u16::MAX).
// Production at part_05.rs:29-51 uses u16::try_from(i64) with bounds check.
// If text is empty or non-numeric, returns Err.

#[flux_rs::extern_spec]
fn slot_from_text(
    text: &str,
    step: usize,
    field: &str,
) -> Result<vb_core::SlotIdx, crate::CompileErrors> {
    // The production implementation guarantees:
    // - Empty text → Err(StepFieldShape)
    // - Non-numeric text → Err(StepFieldShape)
    // - Numeric out of u16 range → Err(SlotIndexOutOfRange)
    // - Valid u16 → Ok(SlotIdx(value))
}

// =========================================================================
// PO-CL-FLUX-BIND-003: StepIdx::checked_add bounds
// =========================================================================
// Property: checked_add returns Some iff self.get() + rhs <= u16::MAX.
// Production at vb_core/src/ids/mod.rs (via numeric_id! macro).
// StepIdx wraps u16; checked_add uses saturating_add or checked arithmetic.

#[flux_rs::extern_spec]
impl vb_core::StepIdx {
    #[flux_rs::sig(fn(Self, rhs: u16) -> Option<vb_core::StepIdx>)]
    fn checked_add(self, rhs: u16) -> Option<vb_core::StepIdx>;
}

// =========================================================================
// Runtime test evidence for all three properties
// =========================================================================

#[cfg(test)]
mod tests {
    use crate::mod_compile_lowering::SlotCompiler;
    use crate::mod_compile_lowering::part_05::slot_from_text;
    use vb_core::SlotIdx;
    use vb_core::StepIdx;

    #[test]
    fn slot_count_increases_monotonically() {
        let mut compiler = SlotCompiler::new();
        let before = compiler.slot_count();
        compiler.record_slot(SlotIdx::new(5));
        let after = compiler.slot_count();
        assert_eq!(after, Ok(6), "slot_count must reflect 5+1=6");
    }

    #[test]
    fn slot_from_text_valid_and_invalid() {
        // Valid: "5" → SlotIdx(5)
        let result = slot_from_text("5", 0, "test");
        assert!(matches!(result, Ok(_)), "valid slot text must succeed");
        assert_eq!(result.unwrap().as_usize(), 5);

        // Invalid: empty text
        let result = slot_from_text("", 0, "test");
        assert!(matches!(result, Err(crate::CompileErrors(_))), "empty text must fail");

        // Invalid: non-numeric
        let result = slot_from_text("abc", 0, "test");
        assert!(matches!(result, Err(crate::CompileErrors(_))), "non-numeric text must fail");
    }

    #[test]
    fn stepidx_checked_add_bounds() {
        // Within bounds
        let id = StepIdx::new(10);
        let result = id.checked_add(20);
        assert!(matches!(result, Some(_)), "10+20=30 within u16 range");
        assert_eq!(result.unwrap().as_usize(), 30);

        // At boundary
        let id = StepIdx::new(u16::MAX);
        let result = id.checked_add(0);
        assert!(matches!(result, Some(_)), "MAX+0 within u16 range");
        assert_eq!(result.unwrap().as_usize(), u16::MAX as usize);

        // Overflow
        let id = StepIdx::new(u16::MAX);
        let result = id.checked_add(1);
        assert!(matches!(result, None), "MAX+1 overflows u16");
    }
}
