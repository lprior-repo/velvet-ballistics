// =====================================================================
// 14. Proptest: valid access never panics, deterministic, empty=identity, invalid always errors
// =====================================================================

use super::*;
use crate::engine::eval_accessor;
use crate::engine::eval_accessor_with_store;
use crate::ids::RunId;
use proptest::prelude::*;

fn build_test_run_store(
    run_id_val: u64,
    root_value: SlotValue,
) -> Result<(CompiledWorkflow, RunFrame, ValueStore), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(run_id_val), &workflow)?;
    let store = test_store();
    run.write_slot_with_taint(SlotIdx::new(0), root_value, Taint::Clean)
        .map_err(|e| e.to_string())?;
    Ok((workflow, run, store))
}

fn arb_scalar_value() -> impl proptest::strategy::Strategy<Value = SlotValue> {
    prop_oneof![
        Just(SlotValue::Null),
        any::<bool>().prop_map(SlotValue::Bool),
        any::<i64>().prop_map(SlotValue::I64),
        (0u32..1000).prop_map(|id| SlotValue::Symbol(SymbolId::new(id))),
        (0u32..1000).prop_map(|id| SlotValue::List(ListId::new(id))),
        (0u32..1000).prop_map(|id| SlotValue::Object(ObjectId::new(id))),
        (0u64..1000).prop_map(|id| SlotValue::Blob(BlobId::new(id))),
    ]
}

fn arb_bool_i64_null() -> impl proptest::strategy::Strategy<Value = SlotValue> {
    prop_oneof![
        Just(SlotValue::Null),
        any::<bool>().prop_map(SlotValue::Bool),
        any::<i64>().prop_map(SlotValue::I64),
    ]
}

proptest! {
    #[test]
    fn empty_path_is_identity_for_all_scalar_types(val in arb_scalar_value()) {
        let (workflow, run, mut store) = build_test_run_store(1, val).unwrap();
        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
        prop_assert!(result.is_ok(), "empty path access should never fail");
        prop_assert_eq!(result.unwrap(), val);
    }
}

proptest! {
    #[test]
    fn empty_path_identity_without_store(val in arb_scalar_value()) {
        let (workflow, run, _store) = build_test_run_store(2, val).unwrap();
        let result = eval_accessor(&workflow, &run, AccessorIdx::new(0));
        prop_assert!(result.is_ok(), "empty path access without store should never fail");
        prop_assert_eq!(result.unwrap(), val);
    }
}

proptest! {
    #[test]
    fn empty_path_is_deterministic(val in arb_bool_i64_null()) {
        let (workflow, run, mut store) = build_test_run_store(3, val).unwrap();
        let r1 = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
        let r2 = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
        prop_assert_eq!(r1, r2, "two accesses with same input must produce same result");
    }
}

proptest! {
    #[test]
    fn invalid_index_always_errors(
        index in (1u32..u32::MAX),
    ) {
        let workflow = accessor_workflow(vec![PathSegment::Index(index)].into_boxed_slice()).unwrap();
        let mut run = test_frame(RunId::new(4), &workflow).unwrap();
        let mut store = test_store();
        let list = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice()).unwrap();
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean).unwrap();

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
        prop_assert!(result.is_err(), "invalid index must always produce an error");
    }
}

proptest! {
    #[test]
    fn field_on_non_object_always_errors(
        val in prop_oneof![
            Just(SlotValue::Null),
            any::<bool>().prop_map(SlotValue::Bool),
            any::<i64>().prop_map(SlotValue::I64),
            (0u32..1000).prop_map(|id| SlotValue::Symbol(SymbolId::new(id))),
            (0u32..1000).prop_map(|id| SlotValue::List(ListId::new(id))),
            (0u64..1000).prop_map(|id| SlotValue::Blob(BlobId::new(id))),
        ]
    ) {
        let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(1))].into_boxed_slice()).unwrap();
        let mut run = test_frame(RunId::new(5), &workflow).unwrap();
        let mut store = test_store();
        run.write_slot_with_taint(SlotIdx::new(0), val, Taint::Clean).unwrap();

        let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
        prop_assert!(result.is_err(), "field access on non-object must always produce an error");
    }
}
