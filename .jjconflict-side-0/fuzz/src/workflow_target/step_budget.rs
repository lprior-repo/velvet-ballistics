//! Step budget fuzz target body.

pub fn fuzz_step_budget_new(data: &[u8]) {
    if data.is_empty() {
        return;
    }
    let budget_value = if data.len() >= 8 {
        let mut bytes = [0u8; 8];
        let src = &data[..8.min(data.len())];
        bytes[..src.len()].copy_from_slice(src);
        u64::from_le_bytes(bytes)
    } else {
        u64::from(data[0])
    };
    let budget = vb_core::StepBudget::new(budget_value);
    let remaining = budget.remaining();
    assert!(remaining <= vb_core::limits::MAX_STEP_BUDGET);
    let expected = budget_value.min(vb_core::limits::MAX_STEP_BUDGET);
    assert_eq!(remaining, expected);
    let mut mutable_budget = budget;
    let result = mutable_budget.try_take();
    assert!(result.is_ok());
    if expected > 0 {
        let ok = match result {
            Ok(value) => value,
            Err(_) => return,
        };
        let decremented = match expected.checked_sub(1) {
            Some(value) => value,
            None => return,
        };
        assert!(ok);
        assert_eq!(mutable_budget.remaining(), decremented);
    } else {
        let ok = match result {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(!ok);
        assert_eq!(mutable_budget.remaining(), 0);
    }
}
