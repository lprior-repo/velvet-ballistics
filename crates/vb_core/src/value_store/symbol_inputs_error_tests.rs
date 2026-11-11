use super::*;

#[test]
fn compose_duplicate_compiler_symbols() -> Result<(), String> {
    assert_error(
        &symbols(&["x", "x"]),
        &checkpoint(&["a"]),
        &symbol_roots(&[0]),
        InputSymbolBindingError::Metadata(SourceMetadataError::DuplicateSymbol {
            first: SymbolId::new(0),
            duplicate: SymbolId::new(1),
        }),
    )
}

#[test]
fn compose_invalid_list_taint_shape() -> Result<(), String> {
    assert_error(
        &symbols(&["x"]),
        &invalid_taints(),
        &symbol_roots(&[0]),
        InputSymbolBindingError::Core(CoreError::InternalInvariantViolation {
            reason: "list_taint_length_mismatch".into(),
        }),
    )
}

#[test]
fn compose_invalid_root_out_of_bounds() -> Result<(), String> {
    assert_error(
        &symbols(&["x"]),
        &checkpoint(&["a"]),
        &symbol_roots(&[999]),
        InputSymbolBindingError::Core(CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(999),
        }),
    )
}

#[test]
fn compose_input_unchanged_on_error() -> Result<(), String> {
    assert_error(
        &symbols(&["k", "p"]),
        &golden_input(),
        &symbol_roots(&[999]),
        InputSymbolBindingError::Core(CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(999),
        }),
    )
}

#[test]
fn compose_overlong_compiler_symbol() -> Result<(), String> {
    let length = MAX_SYMBOL_BYTES_PER_VALUE
        .checked_add(1)
        .ok_or("length overflow")?;
    let text = "x".repeat(length);
    assert_error(
        &symbols(&[&text]),
        &checkpoint(&["a"]),
        &symbol_roots(&[0]),
        InputSymbolBindingError::Metadata(SourceMetadataError::SymbolTooLong {
            id: SymbolId::new(0),
            bytes: length,
            max: MAX_SYMBOL_BYTES_PER_VALUE,
        }),
    )
}

#[test]
fn compose_combined_arena_bound_exceeded() -> Result<(), String> {
    let mut input = checkpoint(&["a", "b", "c", "d", "e"]);
    input.max_arena_entries = 6;
    assert_error(
        &symbols(&["x", "y", "z"]),
        &input,
        &symbol_roots(&[0]),
        InputSymbolBindingError::Core(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 6,
        }),
    )
}

#[test]
fn compose_invalid_nested_symbol_rejects_without_fallback() -> Result<(), String> {
    let mut input = checkpoint(&["a"]);
    input.lists = vec![symbol_roots(&[999])];
    input.list_taints = vec![Box::from([Taint::Secret])];
    assert_error(
        &symbols(&["x"]),
        &input,
        &[SlotValue::List(ListId::new(0))],
        InputSymbolBindingError::Core(CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(999),
        }),
    )
}

#[test]
fn compose_invalid_object_key_rejects_without_fallback() -> Result<(), String> {
    let mut input = checkpoint(&["a"]);
    input.objects = vec![Box::from([ObjectField::clean(
        SymbolId::new(999),
        SlotValue::I64(1),
    )])];
    assert_error(
        &symbols(&["x"]),
        &input,
        &[SlotValue::Object(ObjectId::new(0))],
        InputSymbolBindingError::Core(CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(999),
        }),
    )
}
