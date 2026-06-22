#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]

//! Tests for object_list module.

use vb_core::errors::EngineError;
use vb_core::ids::{RunId, SlotIdx, StepIdx, SymbolId};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;

use vb_core::engine::{build_list, build_list_with_taint, build_object, build_object_with_taint};

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

fn test_frame(slot_count: u16) -> Result<vb_core::frame::RunFrame, String> {
    vb_core::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, slot_count)
        .map_err(|e| e.to_string())
}

// ===== build_object tests =====

#[test]
fn build_object_empty_fields_creates_empty_object() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = test_frame(1)?;
    let obj = build_object(&mut store, &run, &[]).map_err(|e| e.to_string())?;
    let fields = store.object(obj).map_err(|e| e.to_string())?;
    ensure_equal(fields.is_empty(), true)
}

#[test]
fn build_object_single_field_reads_correct_slot() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(10))
        .map_err(|e| e.to_string())?;
    let obj = build_object(&mut store, &run, &[(SymbolId::new(5), SlotIdx::new(0))])
        .map_err(|e| e.to_string())?;
    let fields = store.object(obj).map_err(|e| e.to_string())?;
    ensure_equal(fields.len(), 1)?;
    ensure_equal(fields[0].key, SymbolId::new(5))?;
    ensure_equal(fields[0].value, SlotValue::I64(10))
}

#[test]
fn build_object_rejects_out_of_bounds_slot() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = test_frame(1)?;
    let result = build_object(&mut store, &run, &[(SymbolId::new(0), SlotIdx::new(5))]);
    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => {
            ensure_equal(store.object_count(), 0)
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== build_object_with_taint tests =====

#[test]
fn build_object_with_taint_all_clean_produces_clean() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(2))
        .map_err(|e| e.to_string())?;
    let (obj, taint) = build_object_with_taint(
        &mut store,
        &run,
        &[
            (SymbolId::new(0), SlotIdx::new(0)),
            (SymbolId::new(1), SlotIdx::new(1)),
        ],
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Clean)?;
    let fields = store.object(obj).map_err(|e| e.to_string())?;
    ensure_equal(fields.len(), 2)
}

#[test]
fn build_object_with_taint_joins_secret_from_one_field() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let (_obj, taint) = build_object_with_taint(
        &mut store,
        &run,
        &[
            (SymbolId::new(0), SlotIdx::new(0)),
            (SymbolId::new(1), SlotIdx::new(1)),
        ],
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Secret)
}

#[test]
fn build_object_with_taint_joins_derived_from_secret() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;
    let (_obj, taint) = build_object_with_taint(
        &mut store,
        &run,
        &[
            (SymbolId::new(0), SlotIdx::new(0)),
            (SymbolId::new(1), SlotIdx::new(1)),
        ],
    )
    .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::DerivedFromSecret)
}

// ===== build_list tests =====

#[test]
fn build_list_empty_items_creates_empty_list() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = test_frame(1)?;
    let list = build_list(&mut store, &run, &[]).map_err(|e| e.to_string())?;
    let items = store.list(list).map_err(|e| e.to_string())?;
    ensure_equal(items.is_empty(), true)
}

#[test]
fn build_list_single_item_reads_correct_slot() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Bool(false))
        .map_err(|e| e.to_string())?;
    let list = build_list(&mut store, &run, &[SlotIdx::new(0)]).map_err(|e| e.to_string())?;
    let items = store.list(list).map_err(|e| e.to_string())?;
    ensure_equal(items.len(), 1)?;
    ensure_equal(items[0], SlotValue::Bool(false))
}

#[test]
fn build_list_preserves_order() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(3)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(2))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(2), SlotValue::I64(3))
        .map_err(|e| e.to_string())?;
    let list = build_list(
        &mut store,
        &run,
        &[SlotIdx::new(2), SlotIdx::new(0), SlotIdx::new(1)],
    )
    .map_err(|e| e.to_string())?;
    let items = store.list(list).map_err(|e| e.to_string())?;
    ensure_equal(items[0], SlotValue::I64(3))?;
    ensure_equal(items[1], SlotValue::I64(1))?;
    ensure_equal(items[2], SlotValue::I64(2))
}

#[test]
fn build_list_rejects_out_of_bounds_slot_without_inserting() -> Result<(), String> {
    let mut store = ValueStore::new();
    let run = test_frame(1)?;
    let result = build_list(&mut store, &run, &[SlotIdx::new(10)]);
    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(10) => {
            ensure_equal(store.list_count(), 0)
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// ===== build_list_with_taint tests =====

#[test]
fn build_list_with_taint_all_clean_produces_clean() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(1))
        .map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(1), SlotValue::I64(2))
        .map_err(|e| e.to_string())?;
    let (_list, taint) =
        build_list_with_taint(&mut store, &run, &[SlotIdx::new(0), SlotIdx::new(1)])
            .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Clean)
}

#[test]
fn build_list_with_taint_joins_secret_from_one_slot() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Secret)
        .map_err(|e| e.to_string())?;
    let (_list, taint) =
        build_list_with_taint(&mut store, &run, &[SlotIdx::new(0), SlotIdx::new(1)])
            .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Secret)
}

#[test]
fn build_list_with_taint_joins_derived_from_secret() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = test_frame(2)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Null, Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Clean)
        .map_err(|e| e.to_string())?;
    let (_list, taint) =
        build_list_with_taint(&mut store, &run, &[SlotIdx::new(0), SlotIdx::new(1)])
            .map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::DerivedFromSecret)
}

#[test]
fn build_object_with_taint_rejects_out_of_bounds_slot() {
    let mut store = ValueStore::new();
    let run = test_frame(1).expect("frame");
    let result = build_object_with_taint(&mut store, &run, &[(SymbolId::new(0), SlotIdx::new(5))]);
    assert_eq!(
        result,
        Err(EngineError::SlotOutOfBounds {
            slot: SlotIdx::new(5),
        })
    );
    assert_eq!(store.object_count(), 0);
}

#[test]
fn build_list_with_taint_rejects_out_of_bounds_slot() {
    let mut store = ValueStore::new();
    let run = test_frame(1).expect("frame");
    let result = build_list_with_taint(&mut store, &run, &[SlotIdx::new(10)]);
    assert_eq!(
        result,
        Err(EngineError::SlotOutOfBounds {
            slot: SlotIdx::new(10),
        })
    );
    assert_eq!(store.list_count(), 0);
}

#[test]
fn build_object_with_taint_rejects_uninitialized_slot() {
    let mut store = ValueStore::new();
    let run = test_frame(2).expect("frame");
    // Slot 0 is uninitialized
    let result = build_object_with_taint(&mut store, &run, &[(SymbolId::new(0), SlotIdx::new(0))]);
    assert_eq!(
        result,
        Err(EngineError::SlotUninitialized {
            slot: SlotIdx::new(0),
        })
    );
}

#[test]
fn build_list_with_taint_rejects_uninitialized_slot() {
    let mut store = ValueStore::new();
    let run = test_frame(2).expect("frame");
    // Slot 0 is uninitialized
    let result = build_list_with_taint(&mut store, &run, &[SlotIdx::new(0)]);
    assert_eq!(
        result,
        Err(EngineError::SlotUninitialized {
            slot: SlotIdx::new(0),
        })
    );
}

// ===== ValueStore::insert_object duplicate-key rejection (CF-006) =====

#[test]
fn insert_object_rejects_duplicate_keys() {
    use vb_core::value_store::ObjectField;

    let mut store = ValueStore::new();
    let sym_a = store.insert_symbol("a").expect("symbol a");
    let fields: Box<[ObjectField]> = vec![
        ObjectField {
            key: sym_a,
            value: SlotValue::I64(1),
            taint: Taint::Clean,
        },
        ObjectField {
            key: sym_a,
            value: SlotValue::I64(2),
            taint: Taint::Clean,
        },
    ]
    .into_boxed_slice();
    let result = store.insert_object(fields);
    assert_eq!(
        result,
        Err(vb_core::errors::CoreError::InvalidCompiledWorkflow {
            reason: "duplicate_object_key",
        })
    );
    assert_eq!(store.object_count(), 0);
}

#[test]
fn insert_object_accepts_distinct_keys() {
    use vb_core::value_store::ObjectField;

    let mut store = ValueStore::new();
    let sym_a = store.insert_symbol("a").expect("symbol a");
    let sym_b = store.insert_symbol("b").expect("symbol b");
    let fields: Box<[ObjectField]> = vec![
        ObjectField {
            key: sym_a,
            value: SlotValue::I64(1),
            taint: Taint::Clean,
        },
        ObjectField {
            key: sym_b,
            value: SlotValue::I64(2),
            taint: Taint::Clean,
        },
    ]
    .into_boxed_slice();
    let obj = store.insert_object(fields).expect("insert_object");
    assert_eq!(store.object_count(), 1);
    assert_eq!(
        store.object_field(obj, sym_a).expect("field a"),
        SlotValue::I64(1)
    );
    assert_eq!(
        store.object_field(obj, sym_b).expect("field b"),
        SlotValue::I64(2)
    );
    let slice = store.object(obj).expect("object");
    assert_eq!(slice.len(), 2);
    assert_eq!(slice[0].value, SlotValue::I64(1));
    assert_eq!(slice[1].value, SlotValue::I64(2));
}
