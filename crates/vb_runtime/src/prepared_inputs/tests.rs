use super::*;
use vb_core::{ListId, ObjectId, SymbolId};

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::*;

#[test]
fn bound_runtime_preparation_leaves_v1_on_its_existing_path() -> TestResult {
    let input = checkpoint(&["a"], 0);
    let roots = [(
        SlotIdx::new(0),
        SlotValue::Symbol(SymbolId::new(0)),
        Taint::Secret,
    )];
    assert_eq!(
        prepare_bound_runtime_inputs(&workflow(None, 8)?, &input, &roots),
        Ok(None)
    );
    assert_eq!(input, checkpoint(&["a"], 0));
    Ok(())
}

#[test]
fn bound_runtime_preparation_empty_prefix_is_still_bound() -> TestResult {
    let input = checkpoint(&["a", "a"], 8);
    let roots = [(
        SlotIdx::new(1),
        SlotValue::Symbol(SymbolId::new(1)),
        Taint::Secret,
    )];
    let prepared = prepare_bound_runtime_inputs(&workflow(Some(&[]), 8)?, &input, &roots)?
        .ok_or("V2 empty prefix returned legacy None")?;
    assert_eq!(prepared.store.checkpoint_v1(), checkpoint(&["a"], 8));
    assert_eq!(
        prepared.roots,
        vec![(
            SlotIdx::new(1),
            SlotValue::Symbol(SymbolId::new(0)),
            Taint::Secret
        )]
    );
    assert_eq!(input, checkpoint(&["a", "a"], 8));
    Ok(())
}

#[test]
fn bound_runtime_preparation_preserves_full_store_and_tuple_identity() -> TestResult {
    let input = golden_input();
    let roots = golden_roots();
    let original_roots = roots.clone();
    let prepared = prepare_bound_runtime_inputs(&workflow(Some(&["k", "p"]), 8)?, &input, &roots)?
        .ok_or("bound preparation returned None")?;
    assert_eq!(prepared.store.checkpoint_v1(), golden_expected());
    assert_eq!(prepared.roots, expected_roots());
    assert_eq!(input, golden_input());
    assert_eq!(roots, original_roots);
    Ok(())
}

#[test]
fn bound_runtime_preparation_capacity_error_preserves_sources() -> TestResult {
    let mut input = checkpoint(&["payload", "key"], 3);
    input.objects = vec![Box::from([vb_core::ObjectField::clean(
        SymbolId::new(1),
        SlotValue::Symbol(SymbolId::new(0)),
    )])];
    let roots = [(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(0)),
        Taint::Clean,
    )];
    let before = input.clone();
    let original_roots = roots;
    assert_eq!(
        prepare_bound_runtime_inputs(&workflow(Some(&["key", "compiler"]), 8)?, &input, &roots),
        Err(InputSymbolBindingError::Core(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 3
        }),)
    );
    assert_eq!(input, before);
    assert_eq!(roots, original_roots);
    Ok(())
}

#[test]
fn bound_runtime_preparation_invalid_root_retains_typed_error() -> TestResult {
    let input = checkpoint(&["a"], 8);
    let roots = [(
        SlotIdx::new(0),
        SlotValue::Symbol(SymbolId::new(999)),
        Taint::Secret,
    )];
    assert_eq!(
        prepare_bound_runtime_inputs(&workflow(Some(&["k"]), 8)?, &input, &roots),
        Err(InputSymbolBindingError::Core(
            CoreError::SymbolOutOfBounds {
                symbol: SymbolId::new(999)
            }
        ),)
    );
    assert_eq!(input, checkpoint(&["a"], 8));
    Ok(())
}
