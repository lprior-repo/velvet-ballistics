//! Edge case tests for expression evaluator helpers.
//!
//! Covers ALL 12 helpers from vb_core/src/engine/expr_eval/ops_text_list.rs and
//! vb_core/src/engine/expr_eval/ops.rs:
//! - eval_contains, eval_starts_with, eval_ends_with
//! - eval_has, eval_length, eval_empty
//! - eval_sum, eval_count, eval_append, eval_append_if, eval_unique, eval_merge
//!
//! This module addresses the 34+ missing scenarios identified in the test plan
//! and resolves LETHAL findings from test-review-helper-coverage.md:
//!   LETHAL #1: eval_merge is at ops.rs:136 (not in ops_text_list.rs)
//!   LETHAL #2: eval_length and eval_count were absent from the test plan
//!   LETHAL #3: Internal contradiction resolved — tests written for all gaps

#![forbid(unsafe_code)]

use vb_core::errors::EngineError;
use vb_core::ids::{ConstIdx, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::limits::MAX_EXPRESSION_STACK;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts, check_expr_stack_bound,
};
use vb_core::engine::expr_eval::ops_text_list::{
    eval_append, eval_append_if, eval_contains, eval_count, eval_empty, eval_ends_with, eval_has,
    eval_length, eval_starts_with, eval_sum, eval_unique,
};
use vb_core::engine::expr_eval::ops::{eval_exists, eval_merge};
use vb_core::engine::expr_eval::stack::{push_value, ExprStack};
use vb_core::value_store::{ObjectField, ValueStore};

// ---------------------------------------------------------------------------
// Test infrastructure
// ---------------------------------------------------------------------------

fn eval_ops_with_slots(
    ops: Vec<ExprOp>,
    slots: Vec<SlotValue>,
    constants: Vec<ConstValue>,
    store: &mut ValueStore,
) -> Result<SlotValue, String> {
    let max_stack =
        check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK).map_err(|e| e.to_string())?;
    let expr = ExprProgram::try_from_parts(ops.into_boxed_slice(), max_stack)
        .map_err(|e| e.to_string())?;
    let slot_count = if slots.is_empty() {
        1u16
    } else {
        u16::try_from(slots.len()).map_err(|_| "slot count overflow")?
    };
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("edge_case_test"),
        digest: WorkflowDigest::from_bytes([0xFC; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count: 10,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;
    let mut run = vb_core::frame::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, slot_count)
        .map_err(|e| e.to_string())?;
    for (i, value) in slots.iter().enumerate() {
        let idx = u16::try_from(i).map_err(|_| "slot index overflow")?;
        run.write_slot(SlotIdx::new(idx), *value)
            .map_err(|e| e.to_string())?;
    }
    let (value, _taint) =
        vb_core::engine::expr_eval::eval_expr_with_store(&workflow, &run, store, ExprIdx::new(0))
            .map_err(|e| e.to_string())?;
    Ok(value)
}

// ---------------------------------------------------------------------------
// eval_empty edge cases
// Missing: empty symbol, empty object, non-empty symbol, non-empty object,
//          bool input
// ---------------------------------------------------------------------------

mod empty_edge_cases {
    use super::*;

    #[test]
    fn empty_returns_true_when_symbol_is_empty_string() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let empty_sym = store.insert_symbol("").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(empty_sym)).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(result, Ok(()));
        // Stack top should be Bool(true)
        let top = stack.pop().expect("pop");
        assert_eq!(top, SlotValue::Bool(true));
    }

    #[test]
    fn empty_returns_true_when_object_has_no_fields() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let empty_obj = store
            .insert_object(Vec::<vb_core::value_store::ObjectField>::new().into_boxed_slice())
            .expect("insert");
        push_value(&mut stack, SlotValue::Object(empty_obj)).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(result, Ok(()));
        let top = stack.pop().expect("pop");
        assert_eq!(top, SlotValue::Bool(true));
    }

    #[test]
    fn empty_returns_false_when_symbol_has_characters() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let sym = store.insert_symbol("x").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(sym)).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(result, Ok(()));
        let top = stack.pop().expect("pop");
        assert_eq!(top, SlotValue::Bool(false));
    }

    #[test]
    fn empty_returns_false_when_object_has_fields() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let obj = store
            .insert_object(
                vec![vb_core::value_store::ObjectField {
                    key: SymbolId::new(0),
                    value: SlotValue::I64(1),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .expect("insert");
        push_value(&mut stack, SlotValue::Object(obj)).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(result, Ok(()));
        let top = stack.pop().expect("pop");
        assert_eq!(top, SlotValue::Bool(false));
    }

    #[test]
    fn empty_rejects_bool_input() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::Bool(true)).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text, list, object, or null".to_string(),
                found: "boolean".to_string(),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// eval_unique edge cases
// Missing: all unique, single element, all duplicates
// ---------------------------------------------------------------------------

mod unique_edge_cases {
    use super::*;

    #[test]
    fn unique_returns_original_list_when_all_elements_unique() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        let list_id = match result {
            Ok(SlotValue::List(id)) => id,
            other => panic!("expected List, got {other:?}"),
        };
        let items = store.list(list_id).expect("lookup");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], SlotValue::I64(1));
        assert_eq!(items[1], SlotValue::I64(2));
        assert_eq!(items[2], SlotValue::I64(3));
    }

    #[test]
    fn unique_handles_single_element_list() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(42)].into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        let list_id = match result {
            Ok(SlotValue::List(id)) => id,
            other => panic!("expected List, got {other:?}"),
        };
        let items = store.list(list_id).expect("lookup");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], SlotValue::I64(42));
    }

    #[test]
    fn unique_handles_all_duplicates() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![
                    SlotValue::I64(5),
                    SlotValue::I64(5),
                    SlotValue::I64(5),
                    SlotValue::I64(5),
                ]
                .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        let list_id = match result {
            Ok(SlotValue::List(id)) => id,
            other => panic!("expected List, got {other:?}"),
        };
        let items = store.list(list_id).expect("lookup");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], SlotValue::I64(5));
    }
}

// ---------------------------------------------------------------------------
// eval_contains edge cases
// Missing: non-symbol needle, empty haystack, empty needle
// ---------------------------------------------------------------------------

mod contains_edge_cases {
    use super::*;

    #[test]
    fn contains_rejects_non_symbol_needle() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let hay = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(hay)).expect("push");
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        let result = eval_contains(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn contains_returns_false_when_haystack_is_empty() {
        let mut store = ValueStore::new();
        let empty = store.insert_symbol("").expect("insert");
        let needle = store.insert_symbol("a").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Contains,
            ],
            vec![],
            vec![ConstValue::Symbol(empty), ConstValue::Symbol(needle)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }

    #[test]
    fn contains_returns_true_when_needle_is_empty_string() {
        let mut store = ValueStore::new();
        let hay = store.insert_symbol("hello").expect("insert");
        let empty = store.insert_symbol("").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Contains,
            ],
            vec![],
            vec![ConstValue::Symbol(hay), ConstValue::Symbol(empty)],
            &mut store,
        );
        // Empty string is a substring of any string
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }
}

// ---------------------------------------------------------------------------
// eval_starts_with edge cases
// Missing: non-symbol text, non-symbol prefix, empty prefix, prefix=text,
//          prefix longer than text
// ---------------------------------------------------------------------------

mod starts_with_edge_cases {
    use super::*;

    #[test]
    fn starts_with_rejects_non_symbol_text() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let prefix = store.insert_symbol("a").expect("insert");
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        push_value(&mut stack, SlotValue::Symbol(prefix)).expect("push");
        let result = eval_starts_with(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn starts_with_rejects_non_symbol_prefix() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(text)).expect("push");
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        let result = eval_starts_with(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn starts_with_returns_true_when_prefix_is_empty_string() {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        let empty = store.insert_symbol("").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::StartsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(empty)],
            &mut store,
        );
        // Every string starts with empty prefix
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn starts_with_returns_true_when_prefix_equals_text() {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::StartsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(text)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn starts_with_returns_false_when_prefix_is_longer_than_text() {
        let mut store = ValueStore::new();
        let short = store.insert_symbol("hi").expect("insert");
        let long = store.insert_symbol("hello").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::StartsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(short), ConstValue::Symbol(long)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }
}

// ---------------------------------------------------------------------------
// eval_ends_with edge cases
// Missing: non-symbol text, non-symbol suffix, empty suffix, suffix=text,
//          suffix longer than text
// ---------------------------------------------------------------------------

mod ends_with_edge_cases {
    use super::*;

    #[test]
    fn ends_with_rejects_non_symbol_text() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let suffix = store.insert_symbol("a").expect("insert");
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        push_value(&mut stack, SlotValue::Symbol(suffix)).expect("push");
        let result = eval_ends_with(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn ends_with_rejects_non_symbol_suffix() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(text)).expect("push");
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        let result = eval_ends_with(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn ends_with_returns_true_when_suffix_is_empty_string() {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        let empty = store.insert_symbol("").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::EndsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(empty)],
            &mut store,
        );
        // Every string ends with empty suffix
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn ends_with_returns_true_when_suffix_equals_text() {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::EndsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(text)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn ends_with_returns_false_when_suffix_is_longer_than_text() {
        let mut store = ValueStore::new();
        let short = store.insert_symbol("hi").expect("insert");
        let long = store.insert_symbol("hello").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::EndsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(short), ConstValue::Symbol(long)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }
}

// ---------------------------------------------------------------------------
// eval_has edge cases
// Missing: non-list first operand
// ---------------------------------------------------------------------------

mod has_edge_cases {
    use super::*;

    #[test]
    fn has_rejects_non_list_first_operand() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        push_value(&mut stack, SlotValue::I64(1)).expect("push");
        let result = eval_has(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "list".to_string(),
                found: "number".to_string(),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// eval_append edge cases
// Missing: empty list input, item of various types, non-mutation verification
// ---------------------------------------------------------------------------

mod append_edge_cases {
    use super::*;

    #[test]
    fn append_handles_empty_list_input() {
        let mut store = ValueStore::new();
        let empty_list = store
            .insert_list(vec![].into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Append,
            ],
            vec![SlotValue::List(empty_list)],
            vec![ConstValue::I64(1)],
            &mut store,
        );
        let list_id = match result {
            Ok(SlotValue::List(id)) => id,
            other => panic!("expected List, got {other:?}"),
        };
        let items = store.list(list_id).expect("lookup");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], SlotValue::I64(1));
    }

    #[test]
    fn append_item_of_various_types() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .expect("insert");
        let sym = store.insert_symbol("hello").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Append,
            ],
            vec![SlotValue::List(list)],
            vec![ConstValue::Symbol(sym)],
            &mut store,
        );
        let list_id = match result {
            Ok(SlotValue::List(id)) => id,
            other => panic!("expected List, got {other:?}"),
        };
        let items = store.list(list_id).expect("lookup");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], SlotValue::I64(1));
        assert_eq!(items[1], SlotValue::Symbol(sym));
    }

    #[test]
    fn append_returns_new_list_does_not_mutate_original() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .expect("insert");
        let original_id = list;
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Append,
            ],
            vec![SlotValue::List(list)],
            vec![ConstValue::I64(2)],
            &mut store,
        );
        // Result is a new list with [1, 2]
        let new_list_id = match result {
            Ok(SlotValue::List(id)) => id,
            other => panic!("expected List, got {other:?}"),
        };
        let new_items = store.list(new_list_id).expect("lookup");
        assert_eq!(new_items.len(), 2);
        assert_eq!(new_items[0], SlotValue::I64(1));
        assert_eq!(new_items[1], SlotValue::I64(2));
        // Original list is unchanged
        let original_items = store.list(original_id).expect("lookup");
        assert_eq!(original_items.len(), 1);
        assert_eq!(original_items[0], SlotValue::I64(1));
    }
}

// ---------------------------------------------------------------------------
// eval_append_if edge cases
// Missing: empty list + true, empty list + false, non-bool condition
// ---------------------------------------------------------------------------

mod append_if_edge_cases {
    use super::*;

    #[test]
    fn append_if_handles_empty_list_with_true_condition() {
        let mut store = ValueStore::new();
        let empty_list = store
            .insert_list(vec![].into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::AppendIf,
            ],
            vec![SlotValue::List(empty_list)],
            vec![ConstValue::I64(1), ConstValue::Bool(true)],
            &mut store,
        );
        let list_id = match result {
            Ok(SlotValue::List(id)) => id,
            other => panic!("expected List, got {other:?}"),
        };
        let items = store.list(list_id).expect("lookup");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], SlotValue::I64(1));
    }

    #[test]
    fn append_if_handles_empty_list_with_false_condition() {
        let mut store = ValueStore::new();
        let empty_list = store
            .insert_list(vec![].into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::AppendIf,
            ],
            vec![SlotValue::List(empty_list)],
            vec![ConstValue::I64(1), ConstValue::Bool(false)],
            &mut store,
        );
        let list_id = match result {
            Ok(SlotValue::List(id)) => id,
            other => panic!("expected List, got {other:?}"),
        };
        let items = store.list(list_id).expect("lookup");
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn append_if_rejects_non_bool_condition() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .expect("insert");
        push_value(&mut stack, SlotValue::List(list)).expect("push");
        push_value(&mut stack, SlotValue::I64(2)).expect("push");
        push_value(&mut stack, SlotValue::I64(1)).expect("push"); // non-bool condition
        let result = eval_append_if(&mut stack, &mut store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "boolean".to_string(),
                found: "number".to_string(),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// eval_sum edge cases
// Missing: non-list input, non-i64 in list, single element, negative numbers
// ---------------------------------------------------------------------------

mod sum_edge_cases {
    use super::*;

    #[test]
    fn sum_rejects_non_list_input() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        let result = eval_sum(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "list".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn sum_rejects_list_containing_non_i64_values() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let sym = store.insert_symbol("hello").expect("insert");
        let list = store
            .insert_list(vec![SlotValue::I64(1), SlotValue::Symbol(sym)].into_boxed_slice())
            .expect("insert");
        push_value(&mut stack, SlotValue::List(list)).expect("push");
        let result = eval_sum(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "number".to_string(),
                found: "symbol".to_string(),
            })
        );
    }

    #[test]
    fn sum_handles_single_element_list() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(42)].into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(42)));
    }

    #[test]
    fn sum_handles_negative_numbers() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![
                    SlotValue::I64(-5),
                    SlotValue::I64(10),
                    SlotValue::I64(-3),
                ]
                .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(2))); // -5 + 10 + -3 = 2
    }
}

// ---------------------------------------------------------------------------
// eval_length edge cases (LETHAL #2 — absent from original plan)
// Missing: type mismatch on i64, type mismatch on bool (number input already tested
//          as part of the original plan)
// ---------------------------------------------------------------------------

mod length_edge_cases {
    use super::*;

    #[test]
    fn length_returns_zero_for_empty_symbol() {
        let mut store = ValueStore::new();
        let empty = store.insert_symbol("").expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Length],
            vec![],
            vec![ConstValue::Symbol(empty)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(0)));
    }

    #[test]
    fn length_returns_zero_for_empty_list() {
        let mut store = ValueStore::new();
        let list = store.insert_list(vec![].into_boxed_slice()).expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(0)));
    }

    #[test]
    fn length_returns_zero_for_empty_object() {
        let mut store = ValueStore::new();
        let obj = store
            .insert_object(Vec::<vb_core::value_store::ObjectField>::new().into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length],
            vec![SlotValue::Object(obj)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(0)));
    }

    #[test]
    fn length_rejects_number_input() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        let result = eval_length(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text, list, or object".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn length_rejects_bool_input() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::Bool(true)).expect("push");
        let result = eval_length(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "text, list, or object".to_string(),
                found: "boolean".to_string(),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// eval_count edge cases (LETHAL #2 — absent from original plan)
// Missing: type mismatch on non-list (OOB already tested)
// ---------------------------------------------------------------------------

mod count_edge_cases {
    use super::*;

    #[test]
    fn count_returns_zero_for_empty_list() {
        let mut store = ValueStore::new();
        let list = store.insert_list(vec![].into_boxed_slice()).expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Count],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(0)));
    }

    #[test]
    fn count_rejects_non_list_input() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        let result = eval_count(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "list".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn count_rejects_symbol_input() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let sym = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(sym)).expect("push");
        let result = eval_count(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "list".to_string(),
                found: "symbol".to_string(),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// eval_merge edge cases (LETHAL #1 — found at ops.rs:136, not ops_text_list.rs)
// Missing all scenarios: no overlap, left-only keys, right-only keys,
//                         both OOB
// ---------------------------------------------------------------------------

mod merge_edge_cases {
    use super::*;

    fn make_object(
        store: &mut ValueStore,
        fields: Vec<(SymbolId, i64)>,
    ) -> ObjectId {
        let object_fields: Vec<_> = fields
            .into_iter()
            .map(|(key, val)| vb_core::value_store::ObjectField {
                key,
                value: SlotValue::I64(val),
                taint: Taint::Clean,
            })
            .collect();
        store
            .insert_object(object_fields.into_boxed_slice())
            .expect("insert_object")
    }

    #[test]
    fn merge_combines_two_objects_with_no_overlapping_keys() {
        let mut store = ValueStore::new();
        let key_a = store.insert_symbol("a").expect("insert");
        let key_b = store.insert_symbol("b").expect("insert");
        let obj1 = make_object(&mut store, vec![(key_a, 1)]);
        let obj2 = make_object(&mut store, vec![(key_b, 2)]);
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Merge,
            ],
            vec![SlotValue::Object(obj1), SlotValue::Object(obj2)],
            vec![],
            &mut store,
        );
        let merged_id = match result {
            Ok(SlotValue::Object(id)) => id,
            other => panic!("expected Object, got {other:?}"),
        };
        let fields = store.object(merged_id).expect("lookup");
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn merge_left_object_overwrites_right_on_conflict() {
        let mut store = ValueStore::new();
        let key_a = store.insert_symbol("a").expect("insert");
        let obj1 = make_object(&mut store, vec![(key_a, 1)]);
        let obj2 = make_object(&mut store, vec![(key_a, 99)]);
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Merge,
            ],
            vec![SlotValue::Object(obj1), SlotValue::Object(obj2)],
            vec![],
            &mut store,
        );
        let merged_id = match result {
            Ok(SlotValue::Object(id)) => id,
            other => panic!("expected Object, got {other:?}"),
        };
        let fields = store.object(merged_id).expect("lookup");
        assert_eq!(fields.len(), 1);
        // Right side (obj2) overwrites left side (obj1) — merge inserts left then overlays right
        // The implementation: iterate right_fields, overwrite or push. So right wins.
        assert_eq!(fields[0].value, SlotValue::I64(99));
    }

    #[test]
    fn merge_returns_error_when_left_object_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let key_a = store.insert_symbol("a").expect("insert");
        let obj2 = make_object(&mut store, vec![(key_a, 2)]);
        push_value(&mut stack, SlotValue::Object(ObjectId::new(9999))).expect("push");
        push_value(&mut stack, SlotValue::Object(obj2)).expect("push");
        let result = eval_merge(&mut stack, &mut store);
        assert_eq!(
            result,
            Err(EngineError::ObjectOutOfBounds {
                object: ObjectId::new(9999)
            })
        );
    }

    #[test]
    fn merge_returns_error_when_right_object_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let key_a = store.insert_symbol("a").expect("insert");
        let obj1 = make_object(&mut store, vec![(key_a, 1)]);
        push_value(&mut stack, SlotValue::Object(obj1)).expect("push");
        push_value(&mut stack, SlotValue::Object(ObjectId::new(9999))).expect("push");
        let result = eval_merge(&mut stack, &mut store);
        assert_eq!(
            result,
            Err(EngineError::ObjectOutOfBounds {
                object: ObjectId::new(9999)
            })
        );
    }

    #[test]
    fn merge_returns_error_when_left_is_not_object() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let key_a = store.insert_symbol("a").expect("insert");
        let obj2 = make_object(&mut store, vec![(key_a, 2)]);
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        push_value(&mut stack, SlotValue::Object(obj2)).expect("push");
        let result = eval_merge(&mut stack, &mut store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "object".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn merge_returns_error_when_right_is_not_object() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let key_a = store.insert_symbol("a").expect("insert");
        let obj1 = make_object(&mut store, vec![(key_a, 1)]);
        push_value(&mut stack, SlotValue::Object(obj1)).expect("push");
        push_value(&mut stack, SlotValue::Bool(true)).expect("push");
        let result = eval_merge(&mut stack, &mut store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "object".to_string(),
                found: "boolean".to_string(),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// Boundary: starts_with OOB on prefix, ends_with OOB on suffix (already have
// haystack/text OOB; add prefix/suffix OOB variants)
// ---------------------------------------------------------------------------

mod text_ops_oob_edge_cases {
    use super::*;

    #[test]
    fn starts_with_returns_error_when_prefix_symbol_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(text)).expect("push");
        push_value(&mut stack, SlotValue::Symbol(SymbolId::new(99))).expect("push");
        let result = eval_starts_with(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::SymbolOutOfBounds {
                symbol: SymbolId::new(99)
            })
        );
    }

    #[test]
    fn ends_with_returns_error_when_suffix_symbol_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(text)).expect("push");
        push_value(&mut stack, SlotValue::Symbol(SymbolId::new(99))).expect("push");
        let result = eval_ends_with(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::SymbolOutOfBounds {
                symbol: SymbolId::new(99)
            })
        );
    }

    #[test]
    fn contains_returns_error_when_needle_symbol_out_of_bounds() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let hay = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(hay)).expect("push");
        push_value(&mut stack, SlotValue::Symbol(SymbolId::new(99))).expect("push");
        let result = eval_contains(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::SymbolOutOfBounds {
                symbol: SymbolId::new(99)
            })
        );
    }
}

// ---------------------------------------------------------------------------
// eval_exists edge cases (entirely absent from original plan)
// ---------------------------------------------------------------------------

mod exists_edge_cases {
    use super::*;

    #[test]
    fn exists_returns_false_when_input_is_null() {
        let mut store = ValueStore::new();
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Exists],
            vec![],
            vec![ConstValue::Null],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }

    #[test]
    fn exists_returns_false_when_object_has_no_fields() {
        let mut store = ValueStore::new();
        let empty_obj = store
            .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Exists],
            vec![SlotValue::Object(empty_obj)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }

    #[test]
    fn exists_returns_true_when_object_has_fields() {
        let mut store = ValueStore::new();
        let sym = store.insert_symbol("key").expect("insert");
        let obj = store
            .insert_object(
                vec![ObjectField {
                    key: sym,
                    value: SlotValue::I64(1),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Exists],
            vec![SlotValue::Object(obj)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn exists_rejects_number_input() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        let result = eval_exists(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "object or null".to_string(),
                found: "number".to_string(),
            })
        );
    }

    #[test]
    fn exists_rejects_text_input() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let sym = store.insert_symbol("hello").expect("insert");
        push_value(&mut stack, SlotValue::Symbol(sym)).expect("push");
        let result = eval_exists(&mut stack, &store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "object or null".to_string(),
                found: "symbol".to_string(),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// eval_has: matching / non-matching / empty list edge cases
// ---------------------------------------------------------------------------

mod has_more_edge_cases {
    use super::*;

    #[test]
    fn has_returns_true_when_item_exists_in_list() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]
                    .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Has,
            ],
            vec![SlotValue::List(list)],
            vec![ConstValue::I64(2)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn has_returns_false_when_item_not_in_list() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]
                    .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Has,
            ],
            vec![SlotValue::List(list)],
            vec![ConstValue::I64(99)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }

    #[test]
    fn has_returns_false_when_list_is_empty() {
        let mut store = ValueStore::new();
        let empty_list = store
            .insert_list(vec![].into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Has,
            ],
            vec![SlotValue::List(empty_list)],
            vec![ConstValue::I64(1)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }
}

// ---------------------------------------------------------------------------
// eval_empty: null and empty list additional coverage
// ---------------------------------------------------------------------------

mod empty_more_edge_cases {
    use super::*;

    #[test]
    fn empty_returns_true_when_input_is_null() {
        let mut stack = ExprStack::new(4).expect("valid");
        let store = ValueStore::new();
        push_value(&mut stack, SlotValue::Null).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(result, Ok(()));
        let top = stack.pop().expect("pop");
        assert_eq!(top, SlotValue::Bool(true));
    }

    #[test]
    fn empty_returns_true_when_list_has_no_items() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let empty_list = store
            .insert_list(vec![].into_boxed_slice())
            .expect("insert");
        push_value(&mut stack, SlotValue::List(empty_list)).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(result, Ok(()));
        let top = stack.pop().expect("pop");
        assert_eq!(top, SlotValue::Bool(true));
    }

    #[test]
    fn empty_returns_false_when_list_has_items() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        let list = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .expect("insert");
        push_value(&mut stack, SlotValue::List(list)).expect("push");
        let result = eval_empty(&mut stack, &store);
        assert_eq!(result, Ok(()));
        let top = stack.pop().expect("pop");
        assert_eq!(top, SlotValue::Bool(false));
    }
}

// ---------------------------------------------------------------------------
// eval_unique: empty list edge case
// ---------------------------------------------------------------------------

mod unique_more_edge_cases {
    use super::*;

    #[test]
    fn unique_returns_empty_list_when_input_is_empty() {
        let mut store = ValueStore::new();
        let empty_list = store
            .insert_list(vec![].into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique],
            vec![SlotValue::List(empty_list)],
            vec![],
            &mut store,
        );
        let list_id = match result {
            Ok(SlotValue::List(id)) => id,
            other => panic!("expected List, got {other:?}"),
        };
        let items = store.list(list_id).expect("lookup");
        assert_eq!(items.len(), 0);
    }
}

// ---------------------------------------------------------------------------
// eval_sum: empty list edge case
// ---------------------------------------------------------------------------

mod sum_more_edge_cases {
    use super::*;

    #[test]
    fn sum_returns_zero_when_list_is_empty() {
        let mut store = ValueStore::new();
        let empty_list = store
            .insert_list(vec![].into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum],
            vec![SlotValue::List(empty_list)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(0)));
    }

    #[test]
    fn sum_handles_multiple_non_negative_numbers() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![
                    SlotValue::I64(100),
                    SlotValue::I64(200),
                    SlotValue::I64(300),
                ]
                .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(600)));
    }
}

// ---------------------------------------------------------------------------
// eval_length: non-empty input edge cases
// ---------------------------------------------------------------------------

mod length_more_edge_cases {
    use super::*;

    #[test]
    fn length_returns_correct_count_for_non_empty_symbol() {
        let mut store = ValueStore::new();
        let sym = store.insert_symbol("hello world").expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadConst(ConstIdx::new(0)), ExprOp::Length],
            vec![],
            vec![ConstValue::Symbol(sym)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(11)));
    }

    #[test]
    fn length_returns_correct_count_for_non_empty_list() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![
                    SlotValue::I64(1),
                    SlotValue::I64(2),
                    SlotValue::I64(3),
                    SlotValue::I64(4),
                ]
                .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(4)));
    }

    #[test]
    fn length_returns_correct_count_for_non_empty_object() {
        let mut store = ValueStore::new();
        let key1 = store.insert_symbol("a").expect("insert");
        let key2 = store.insert_symbol("b").expect("insert");
        let obj = store
            .insert_object(
                vec![
                    ObjectField {
                        key: key1,
                        value: SlotValue::I64(1),
                        taint: Taint::Clean,
                    },
                    ObjectField {
                        key: key2,
                        value: SlotValue::I64(2),
                        taint: Taint::Clean,
                    },
                ]
                .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length],
            vec![SlotValue::Object(obj)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(2)));
    }
}

// ---------------------------------------------------------------------------
// eval_count: non-empty list edge cases
// ---------------------------------------------------------------------------

mod count_more_edge_cases {
    use super::*;

    #[test]
    fn count_matches_list_length_for_non_empty_list() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)]
                    .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Count],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(3)));
    }

    #[test]
    fn count_includes_duplicates_in_total() {
        let mut store = ValueStore::new();
        let list = store
            .insert_list(
                vec![
                    SlotValue::I64(1),
                    SlotValue::I64(1),
                    SlotValue::I64(1),
                    SlotValue::I64(2),
                ]
                .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Count],
            vec![SlotValue::List(list)],
            vec![],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::I64(4)));
    }
}

// ---------------------------------------------------------------------------
// eval_append: non-list input rejection
// ---------------------------------------------------------------------------

mod append_more_edge_cases {
    use super::*;

    #[test]
    fn append_rejects_non_list_first_operand() {
        let mut stack = ExprStack::new(4).expect("valid");
        let mut store = ValueStore::new();
        push_value(&mut stack, SlotValue::I64(42)).expect("push");
        push_value(&mut stack, SlotValue::I64(1)).expect("push");
        let result = eval_append(&mut stack, &mut store);
        assert_eq!(
            result,
            Err(EngineError::TypeMismatch {
                expected: "list".to_string(),
                found: "number".to_string(),
            })
        );
    }
}

// ---------------------------------------------------------------------------
// eval_starts_with: actual matching behavior edge cases
// ---------------------------------------------------------------------------

mod starts_with_more_edge_cases {
    use super::*;

    #[test]
    fn starts_with_returns_true_when_prefix_matches_beginning() {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello world").expect("insert");
        let prefix = store.insert_symbol("hello").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::StartsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(prefix)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn starts_with_returns_false_when_prefix_does_not_match() {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        let prefix = store.insert_symbol("world").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::StartsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(prefix)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }
}

// ---------------------------------------------------------------------------
// eval_ends_with: actual matching behavior edge cases
// ---------------------------------------------------------------------------

mod ends_with_more_edge_cases {
    use super::*;

    #[test]
    fn ends_with_returns_true_when_suffix_matches_ending() {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello world").expect("insert");
        let suffix = store.insert_symbol("world").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::EndsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(suffix)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn ends_with_returns_false_when_suffix_does_not_match() {
        let mut store = ValueStore::new();
        let text = store.insert_symbol("hello").expect("insert");
        let suffix = store.insert_symbol("world").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::EndsWith,
            ],
            vec![],
            vec![ConstValue::Symbol(text), ConstValue::Symbol(suffix)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }
}

// ---------------------------------------------------------------------------
// eval_contains: actual matching behavior edge cases
// ---------------------------------------------------------------------------

mod contains_more_edge_cases {
    use super::*;

    #[test]
    fn contains_returns_true_when_needle_is_found_in_haystack() {
        let mut store = ValueStore::new();
        let hay = store.insert_symbol("hello world").expect("insert");
        let needle = store.insert_symbol("lo wo").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Contains,
            ],
            vec![],
            vec![ConstValue::Symbol(hay), ConstValue::Symbol(needle)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn contains_returns_false_when_needle_is_not_in_haystack() {
        let mut store = ValueStore::new();
        let hay = store.insert_symbol("hello").expect("insert");
        let needle = store.insert_symbol("xyz").expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::LoadConst(ConstIdx::new(1)),
                ExprOp::Contains,
            ],
            vec![],
            vec![ConstValue::Symbol(hay), ConstValue::Symbol(needle)],
            &mut store,
        );
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }
}

// ---------------------------------------------------------------------------
// eval_merge: empty objects edge cases
// ---------------------------------------------------------------------------

mod merge_more_edge_cases {
    use super::*;

    #[test]
    fn merge_two_empty_objects_returns_empty_object() {
        let mut store = ValueStore::new();
        let empty1 = store
            .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
            .expect("insert");
        let empty2 = store
            .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Merge,
            ],
            vec![SlotValue::Object(empty1), SlotValue::Object(empty2)],
            vec![],
            &mut store,
        );
        let merged_id = match result {
            Ok(SlotValue::Object(id)) => id,
            other => panic!("expected Object, got {other:?}"),
        };
        let fields = store.object(merged_id).expect("lookup");
        assert_eq!(fields.len(), 0);
    }

    #[test]
    fn merge_empty_left_with_populated_right_gives_right_fields() {
        let mut store = ValueStore::new();
        let key_a = store.insert_symbol("a").expect("insert");
        let empty = store
            .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
            .expect("insert");
        let right_obj = store
            .insert_object(
                vec![ObjectField {
                    key: key_a,
                    value: SlotValue::I64(42),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Merge,
            ],
            vec![SlotValue::Object(empty), SlotValue::Object(right_obj)],
            vec![],
            &mut store,
        );
        let merged_id = match result {
            Ok(SlotValue::Object(id)) => id,
            other => panic!("expected Object, got {other:?}"),
        };
        let fields = store.object(merged_id).expect("lookup");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].value, SlotValue::I64(42));
    }

    #[test]
    fn merge_populated_left_with_empty_right_gives_left_fields() {
        let mut store = ValueStore::new();
        let key_a = store.insert_symbol("a").expect("insert");
        let left_obj = store
            .insert_object(
                vec![ObjectField {
                    key: key_a,
                    value: SlotValue::I64(99),
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .expect("insert");
        let empty = store
            .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
            .expect("insert");
        let result = eval_ops_with_slots(
            vec![
                ExprOp::LoadSlot(SlotIdx::new(0)),
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::Merge,
            ],
            vec![SlotValue::Object(left_obj), SlotValue::Object(empty)],
            vec![],
            &mut store,
        );
        let merged_id = match result {
            Ok(SlotValue::Object(id)) => id,
            other => panic!("expected Object, got {other:?}"),
        };
        let fields = store.object(merged_id).expect("lookup");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].value, SlotValue::I64(99));
    }
}
