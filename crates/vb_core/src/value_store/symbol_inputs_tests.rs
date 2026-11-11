//! Independent literal oracles for the public cold symbol-composition boundary.

use crate::errors::CoreError;
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::limits::{MAX_SYMBOL_BYTES_PER_VALUE, MAX_VALUES_PER_RUN};
use crate::value::{FiniteF64, SlotValue, Taint};
use crate::value_store::{
    InputSymbolBindingError, ObjectField, ValueStore, ValueStoreCheckpointV1,
    compose_compiler_symbols,
};
use crate::workflow::source_metadata::SourceMetadataError;

#[path = "symbol_inputs_fixture_tests.rs"]
mod fixtures;
use fixtures::*;

#[path = "symbol_inputs_capacity_tests.rs"]
mod capacity;
#[path = "symbol_inputs_edge_tests.rs"]
mod edge;
#[path = "symbol_inputs_error_tests.rs"]
mod errors;

fn assert_success(
    compiler: &[Box<str>],
    input: &ValueStoreCheckpointV1,
    roots: &[SlotValue],
    expected: ValueStoreCheckpointV1,
    expected_roots: &[SlotValue],
) -> Result<ValueStore, String> {
    let original_input = input.clone();
    let original_roots = roots.to_vec();
    let (store, actual_roots) = compose_compiler_symbols(compiler, input, roots)
        .map_err(|error| format!("unexpected composition failure: {error:?}"))?;
    assert_eq!(store.checkpoint_v1(), expected);
    assert_eq!(actual_roots.as_ref(), expected_roots);
    assert_eq!(*input, original_input);
    assert_eq!(roots, original_roots);
    Ok(store)
}

fn assert_error(
    compiler: &[Box<str>],
    input: &ValueStoreCheckpointV1,
    roots: &[SlotValue],
    expected: InputSymbolBindingError,
) -> Result<(), String> {
    let original_input = input.clone();
    let original_roots = roots.to_vec();
    assert_eq!(
        compose_compiler_symbols(compiler, input, roots),
        Err(expected)
    );
    assert_eq!(*input, original_input);
    assert_eq!(roots, original_roots);
    Ok(())
}

#[test]
fn compose_golden_dedup_and_remap() -> Result<(), String> {
    // Given prefix overlap, duplicate input payloads and every nested arena.
    let input = golden_input();
    let roots = golden_roots();
    // When composed, the independent old-ID map is [2, 0, 2, 3].
    let store = assert_success(
        &symbols(&["k", "p"]),
        &input,
        &roots,
        golden_expected(),
        &golden_expected_roots(),
    )?;
    // Then duplicate keys retain the first field's value and taint.
    assert_eq!(
        store.object_field_with_taint(ObjectId::new(0), SymbolId::new(0)),
        Ok((SlotValue::Symbol(SymbolId::new(2)), Taint::Secret)),
    );
    Ok(())
}

#[test]
fn compose_preserves_input_state() -> Result<(), String> {
    let input = state_input();
    let expected = state_expected();
    let roots = [
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::List(ListId::new(0)),
    ];
    let expected_roots = [
        SlotValue::Symbol(SymbolId::new(1)),
        SlotValue::List(ListId::new(0)),
    ];
    assert_success(
        &symbols(&["compiler"]),
        &input,
        &roots,
        expected,
        &expected_roots,
    )?;
    Ok(())
}

#[test]
fn compose_non_symbol_values_unchanged() -> Result<(), String> {
    let number = FiniteF64::new(3.25).map_err(|error| format!("{error:?}"))?;
    let values = [
        SlotValue::I64(42),
        SlotValue::Bool(true),
        SlotValue::F64(number),
        SlotValue::Null,
    ];
    let mut input = checkpoint(&["a"]);
    input.lists = vec![Box::from(values)];
    input.list_taints = vec![Box::from([Taint::Clean; 4])];
    let mut expected = checkpoint(&["x", "a"]);
    expected.lists = vec![Box::from(values)];
    expected.list_taints = vec![Box::from([Taint::Clean; 4])];
    assert_success(&symbols(&["x"]), &input, &values, expected, &values)?;
    Ok(())
}
