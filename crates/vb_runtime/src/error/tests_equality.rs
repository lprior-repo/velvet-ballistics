use super::RuntimeError;
use vb_core::ids::ActionId;
use vb_core::SlotIdx;

fn slot(index: u16) -> SlotIdx {
    SlotIdx::new(index)
}

fn input_kind_null() -> vb_core::InputSlotKind {
    vb_core::InputSlotKind::Null
}

fn input_kind_bool() -> vb_core::InputSlotKind {
    vb_core::InputSlotKind::Bool
}

fn input_kind_i64() -> vb_core::InputSlotKind {
    vb_core::InputSlotKind::I64
}

fn action_id(id: u16) -> ActionId {
    ActionId::new(id)
}

#[test]
fn recovery_reflexivity() {
    let error = RuntimeError::Recovery {
        error: "artifact load failed".to_string(),
    };
    assert_eq!(error.clone(), error);
}

#[test]
fn recovery_identical_strings_equal() {
    let a = RuntimeError::Recovery {
        error: "artifact IR decode failed".to_string(),
    };
    let b = RuntimeError::Recovery {
        error: "artifact IR decode failed".to_string(),
    };
    assert_eq!(a, b);
}

#[test]
fn recovery_different_strings_unequal() {
    let a = RuntimeError::Recovery {
        error: "load failed".to_string(),
    };
    let b = RuntimeError::Recovery {
        error: "decode failed".to_string(),
    };
    assert_ne!(a, b);
}

#[test]
fn contract_validation_reflexivity() {
    let error = RuntimeError::ContractValidationError {
        action: action_id(42),
        error: "duplicate action id".to_string(),
    };
    assert_eq!(error.clone(), error);
}

#[test]
fn contract_validation_identical_equal() {
    let a = RuntimeError::ContractValidationError {
        action: action_id(42),
        error: "duplicate action id".to_string(),
    };
    let b = RuntimeError::ContractValidationError {
        action: action_id(42),
        error: "duplicate action id".to_string(),
    };
    assert_eq!(a, b);
}

#[test]
fn contract_validation_different_action_unequal() {
    let a = RuntimeError::ContractValidationError {
        action: action_id(1),
        error: "duplicate".to_string(),
    };
    let b = RuntimeError::ContractValidationError {
        action: action_id(2),
        error: "duplicate".to_string(),
    };
    assert_ne!(a, b);
}

#[test]
fn contract_validation_different_message_unequal() {
    let a = RuntimeError::ContractValidationError {
        action: action_id(42),
        error: "duplicate".to_string(),
    };
    let b = RuntimeError::ContractValidationError {
        action: action_id(42),
        error: "invalid schema".to_string(),
    };
    assert_ne!(a, b);
}

#[test]
fn contract_validation_different_both_unequal() {
    let a = RuntimeError::ContractValidationError {
        action: action_id(1),
        error: "duplicate".to_string(),
    };
    let b = RuntimeError::ContractValidationError {
        action: action_id(2),
        error: "invalid schema".to_string(),
    };
    assert_ne!(a, b);
}

#[test]
fn slot_kind_mismatch_reflexivity() {
    let error = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_null(),
        actual: input_kind_bool(),
    };
    assert_eq!(error.clone(), error);
}

#[test]
fn slot_kind_mismatch_identical_equal() {
    let a = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_null(),
        actual: input_kind_bool(),
    };
    let b = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_null(),
        actual: input_kind_bool(),
    };
    assert_eq!(a, b);
}

#[test]
fn slot_kind_mismatch_different_slot_unequal() {
    let a = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_null(),
        actual: input_kind_bool(),
    };
    let b = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(1),
        expected: input_kind_null(),
        actual: input_kind_bool(),
    };
    assert_ne!(a, b);
}

#[test]
fn slot_kind_mismatch_different_expected_unequal() {
    let a = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_null(),
        actual: input_kind_bool(),
    };
    let b = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_i64(),
        actual: input_kind_bool(),
    };
    assert_ne!(a, b);
}

#[test]
fn slot_kind_mismatch_different_actual_unequal() {
    let a = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_null(),
        actual: input_kind_bool(),
    };
    let b = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_null(),
        actual: input_kind_i64(),
    };
    assert_ne!(a, b);
}

#[test]
fn cross_variant_recovery_vs_contract() {
    let a = RuntimeError::Recovery {
        error: "duplicate action id".to_string(),
    };
    let b = RuntimeError::ContractValidationError {
        action: action_id(42),
        error: "duplicate action id".to_string(),
    };
    assert_ne!(a, b);
}

#[test]
fn cross_variant_contract_vs_slot_mismatch() {
    let a = RuntimeError::ContractValidationError {
        action: action_id(42),
        error: "mismatch".to_string(),
    };
    let b = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_null(),
        actual: input_kind_bool(),
    };
    assert_ne!(a, b);
}

#[test]
fn cross_variant_slot_mismatch_vs_recovery() {
    let a = RuntimeError::RuntimeInputSlotKindMismatch {
        slot: slot(0),
        expected: input_kind_null(),
        actual: input_kind_bool(),
    };
    let b = RuntimeError::Recovery {
        error: "decode failed".to_string(),
    };
    assert_ne!(a, b);
}
