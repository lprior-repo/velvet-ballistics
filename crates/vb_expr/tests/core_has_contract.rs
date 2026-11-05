#![forbid(unsafe_code)]

use vb_core::EngineError;
use vb_core::engine::{eval_expr_with_store, new_run_frame};
use vb_core::ids::{
    BlobId, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use vb_core::value::{FiniteF64, SlotValue, Taint};
use vb_core::value_store::{ObjectField, ValueStore};
use vb_core::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprOp, ResourceContract, WorkflowParts, node,
};
use vb_expr::compile_expr;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type HasResult = Result<(SlotValue, Taint), EngineError>;

fn check_has(
    store: &mut ValueStore,
    container: SlotValue,
    key: SlotValue,
    taints: [Taint; 2],
    expected: HasResult,
) -> TestResult {
    let resolver = |reference: &str| match reference {
        "$a" => Some(SlotIdx::new(0)),
        "$b" => Some(SlotIdx::new(1)),
        _ => None,
    };
    let (program, constants) = compile_expr("has($a, $b)", &resolver)?;
    assert_eq!(
        program.ops.as_ref(),
        &[
            ExprOp::LoadSlot(SlotIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Has,
        ]
    );
    assert_eq!(program.max_stack, 2);
    assert!(constants.is_empty());
    let plan = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "core_has_contract".into(),
        digest: WorkflowDigest::from_bytes([0x48; 32]),
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: vec![program].into_boxed_slice(),
        accessors: Box::new([]),
        constants: constants.into_boxed_slice(),
        slot_count: 2,
        symbols_count: u32::try_from(store.symbol_count())?,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })?;
    let mut run = new_run_frame(RunId::new(1), &plan)?;
    let [container_taint, key_taint] = taints;
    run.write_slot_with_taint(SlotIdx::new(0), container, container_taint)?;
    run.write_slot_with_taint(SlotIdx::new(1), key, key_taint)?;
    let run_before = run.clone();
    let store_before = store.clone();
    assert_eq!(
        eval_expr_with_store(&plan, &run, store, ExprIdx::new(0)),
        expected
    );
    assert_eq!(*store, store_before);
    assert_eq!(run, run_before);
    Ok(())
}

fn check_clean_has(
    store: &mut ValueStore,
    container: SlotValue,
    key: SlotValue,
    expected: Result<bool, EngineError>,
) -> TestResult {
    check_has(
        store,
        container,
        key,
        [Taint::Clean; 2],
        expected.map(|found| (SlotValue::Bool(found), Taint::Clean)),
    )
}

#[test]
fn object_has_tests_presence_even_for_null_and_false_values() -> TestResult {
    let mut store = ValueStore::new();
    let null_key = store.insert_symbol("null-field")?;
    let false_key = store.insert_symbol("false-field")?;
    let number_key = store.insert_symbol("number-field")?;
    let object = store.insert_object(
        vec![
            ObjectField::clean(null_key, SlotValue::Null),
            ObjectField::clean(false_key, SlotValue::Bool(false)),
            ObjectField::clean(number_key, SlotValue::I64(9)),
        ]
        .into_boxed_slice(),
    )?;
    for key in [null_key, false_key, number_key] {
        check_clean_has(
            &mut store,
            SlotValue::Object(object),
            SlotValue::Symbol(key),
            Ok(true),
        )?;
    }
    Ok(())
}

#[test]
fn object_has_returns_false_for_missing_keys_and_empty_objects() -> TestResult {
    let mut store = ValueStore::new();
    let present = store.insert_symbol("present")?;
    let missing = store.insert_symbol("missing")?;
    let populated = store
        .insert_object(vec![ObjectField::clean(present, SlotValue::I64(1))].into_boxed_slice())?;
    let empty = store.insert_object(Vec::<ObjectField>::new().into_boxed_slice())?;
    for object in [populated, empty] {
        check_clean_has(
            &mut store,
            SlotValue::Object(object),
            SlotValue::Symbol(missing),
            Ok(false),
        )?;
    }
    Ok(())
}

#[test]
fn object_has_rejects_every_non_symbol_key_type() -> TestResult {
    let mut store = ValueStore::new();
    let object = store.insert_object(Vec::<ObjectField>::new().into_boxed_slice())?;
    for (key, found) in [
        (SlotValue::Null, "null"),
        (SlotValue::Bool(false), "boolean"),
        (SlotValue::I64(7), "number"),
        (SlotValue::F64(FiniteF64::new(1.5)?), "number"),
        (SlotValue::List(ListId::new(u32::MAX)), "list"),
        (SlotValue::Object(ObjectId::new(u32::MAX)), "object"),
        (SlotValue::Blob(BlobId::new(u64::MAX)), "blob"),
    ] {
        check_clean_has(
            &mut store,
            SlotValue::Object(object),
            key,
            Err(EngineError::TypeMismatch {
                expected: "text",
                found,
            }),
        )?;
    }
    Ok(())
}

#[test]
fn object_has_checks_key_type_before_object_handle() -> TestResult {
    let mut store = ValueStore::new();
    let key = store.insert_symbol("valid-key")?;
    let invalid = ObjectId::new(u32::MAX);
    check_clean_has(
        &mut store,
        SlotValue::Object(invalid),
        SlotValue::I64(7),
        Err(EngineError::TypeMismatch {
            expected: "text",
            found: "number",
        }),
    )?;
    check_clean_has(
        &mut store,
        SlotValue::Object(invalid),
        SlotValue::Symbol(key),
        Err(EngineError::ObjectOutOfBounds { object: invalid }),
    )?;
    Ok(())
}

#[test]
fn object_has_compares_key_ids_without_dereferencing_symbols() -> TestResult {
    let mut store = ValueStore::new();
    let first = store.insert_symbol("same-spelling")?;
    let second = store.insert_symbol("same-spelling")?;
    assert_ne!(first, second);
    let dangling = SymbolId::new(u32::MAX);
    assert_eq!(
        store.symbol(dangling),
        Err(EngineError::SymbolOutOfBounds { symbol: dangling })
    );
    let ordinary =
        store.insert_object(vec![ObjectField::clean(first, SlotValue::Null)].into_boxed_slice())?;
    for absent in [second, dangling] {
        check_clean_has(
            &mut store,
            SlotValue::Object(ordinary),
            SlotValue::Symbol(absent),
            Ok(false),
        )?;
    }
    let uninterned_key = store.insert_object(
        vec![ObjectField::clean(dangling, SlotValue::Bool(false))].into_boxed_slice(),
    )?;
    check_clean_has(
        &mut store,
        SlotValue::Object(uninterned_key),
        SlotValue::Symbol(dangling),
        Ok(true),
    )
}

#[test]
fn object_presence_preserves_operand_taint_without_reading_field_value_taint() -> TestResult {
    let mut store = ValueStore::new();
    let present = store.insert_symbol("secret-value")?;
    let missing = store.insert_symbol("missing")?;
    let object = store.insert_object(
        vec![ObjectField::with_taint(
            present,
            SlotValue::Null,
            Taint::Secret,
        )]
        .into_boxed_slice(),
    )?;
    for (key, taints, found, expected_taint) in [
        (present, [Taint::Clean, Taint::Clean], true, Taint::Clean),
        (present, [Taint::Secret, Taint::Clean], true, Taint::Secret),
        (
            present,
            [Taint::Clean, Taint::DerivedFromSecret],
            true,
            Taint::DerivedFromSecret,
        ),
        (
            missing,
            [Taint::Random, Taint::TimeDependent],
            false,
            Taint::TimeDependent,
        ),
    ] {
        check_has(
            &mut store,
            SlotValue::Object(object),
            SlotValue::Symbol(key),
            taints,
            Ok((SlotValue::Bool(found), expected_taint)),
        )?;
    }
    Ok(())
}

#[test]
fn has_rejects_populated_empty_and_invalid_lists_for_every_key_type() -> TestResult {
    let mut store = ValueStore::new();
    let symbol = store.insert_symbol("key")?;
    let populated = store.insert_list(vec![SlotValue::I64(20)].into_boxed_slice())?;
    let empty = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
    let keys = [
        SlotValue::Null,
        SlotValue::Bool(false),
        SlotValue::I64(20),
        SlotValue::F64(FiniteF64::new(7.5)?),
        SlotValue::Symbol(symbol),
        SlotValue::List(ListId::new(u32::MAX)),
        SlotValue::Object(ObjectId::new(u32::MAX)),
        SlotValue::Blob(BlobId::new(u64::MAX)),
    ];
    for list in [populated, empty, ListId::new(u32::MAX)] {
        for key in keys {
            check_clean_has(
                &mut store,
                SlotValue::List(list),
                key,
                Err(EngineError::TypeMismatch {
                    expected: "object",
                    found: "list",
                }),
            )?;
        }
    }
    Ok(())
}

#[test]
fn has_checks_receiver_type_before_key_type() -> TestResult {
    let mut store = ValueStore::new();
    let symbol = store.insert_symbol("key")?;
    for (receiver, found) in [
        (SlotValue::Null, "null"),
        (SlotValue::Bool(false), "boolean"),
        (SlotValue::I64(7), "number"),
        (SlotValue::F64(FiniteF64::new(1.5)?), "number"),
        (SlotValue::Symbol(symbol), "symbol"),
        (SlotValue::Blob(BlobId::new(u64::MAX)), "blob"),
    ] {
        for key in [SlotValue::Symbol(symbol), SlotValue::Null] {
            check_clean_has(
                &mut store,
                receiver,
                key,
                Err(EngineError::TypeMismatch {
                    expected: "object",
                    found,
                }),
            )?;
        }
    }
    Ok(())
}
