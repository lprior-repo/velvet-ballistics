#![forbid(unsafe_code)]
//! Public API helper evaluation.

use vb_core::value_store::ValueStore;
use vb_core::SlotValue;
use crate::parser::ExprHelper;

use crate::ExprResult;

use super::super::impls::{
    eval_helper_append_if_with_store, eval_helper_append_with_store, eval_helper_contains_with_store,
    eval_helper_count_with_store, eval_helper_empty_with_store, eval_helper_ends_with_with_store,
    eval_helper_exists_with_store, eval_helper_has_with_store, eval_helper_length_with_store,
    eval_helper_merge_with_store, eval_helper_starts_with_with_store, eval_helper_sum_with_store,
    eval_helper_unique_with_store,
};
use super::args::{one_arg, three_args, two_args};

/// Evaluates helper functions with full `ValueStore` access.
///
/// Unlike [`eval_helper`], this variant can resolve opaque handles (`List`,
/// `Object`, `Symbol`) through the store, enabling complete evaluation of
/// helpers like `Empty`, `Unique`, `Length`, `Contains`, etc.
pub fn eval_helper_with_store(
    helper: ExprHelper,
    args: &[SlotValue],
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    match helper {
        ExprHelper::Exists => {
            let value = one_arg(args, helper)?;
            eval_helper_exists_with_store(value, store)
        }
        ExprHelper::Length => {
            let value = one_arg(args, helper)?;
            eval_helper_length_with_store(value, store)
        }
        ExprHelper::Empty => {
            let value = one_arg(args, helper)?;
            eval_helper_empty_with_store(value, store)
        }
        ExprHelper::Count => {
            let value = one_arg(args, helper)?;
            eval_helper_count_with_store(value, store)
        }
        ExprHelper::Unique => {
            let value = one_arg(args, helper)?;
            eval_helper_unique_with_store(value, store)
        }
        ExprHelper::Contains => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_contains_with_store(left, right, store)
        }
        ExprHelper::StartsWith => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_starts_with_with_store(left, right, store)
        }
        ExprHelper::EndsWith => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_ends_with_with_store(left, right, store)
        }
        ExprHelper::Has => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_has_with_store(left, right, store)
        }
        ExprHelper::Append => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_append_with_store(left, right, store)
        }
        ExprHelper::AppendIf => {
            let (first, second, third) = three_args(args, helper)?;
            eval_helper_append_if_with_store(first, second, third, store)
        }
        ExprHelper::Merge => {
            let (left, right) = two_args(args, helper)?;
            eval_helper_merge_with_store(left, right, store)
        }
        ExprHelper::Sum => {
            let value = one_arg(args, helper)?;
            eval_helper_sum_with_store(value, store)
        }
    }
}

/// Evaluates helper behavior that is local to scalar/handle values.
///
/// Note: Most helpers require a ValueStore to resolve opaque handles (List, Object, Symbol).
/// This function only supports helpers that work without store access.
/// For full helper evaluation, use [`eval_helper_with_store`].
pub fn eval_helper(helper: ExprHelper, args: &[SlotValue]) -> ExprResult<SlotValue> {
    match helper {
        // Exists works without store - it just checks for Null
        ExprHelper::Exists => {
            let value = one_arg(args, helper)?;
            Ok(SlotValue::Bool(!matches!(**value, SlotValue::Null)))
        }
        // Empty works for Null without store (returns true), but errors for other types
        ExprHelper::Empty => {
            let value = one_arg(args, helper)?;
            match **value {
                SlotValue::Null => Ok(SlotValue::Bool(true)),
                SlotValue::F64(_) => Err(crate::ExprError::TypeMismatch {
                    expected: "list, text, object, or null".into(),
                    found: "number".into(),
                }),
                SlotValue::List(_) => Err(crate::ExprError::TypeMismatch {
                    expected: "value-store context required for list emptiness check".into(),
                    found: "list handle without store".into(),
                }),
                ref other => Err(crate::ExprError::TypeMismatch {
                    expected: "list or null".into(),
                    found: other.type_name().into(),
                }),
            }
        }
        // Length and Count: need store for List/Null, but return type error for non-list
        ExprHelper::Length | ExprHelper::Count => {
            let value = one_arg(args, helper)?;
            match **value {
                SlotValue::F64(_) => Err(crate::ExprError::TypeMismatch {
                    expected: "list, text, or object".into(),
                    found: "number".into(),
                }),
                SlotValue::List(_) | SlotValue::Null => Err(crate::ExprError::TypeMismatch {
                    expected: "value-store context required for list length".into(),
                    found: "list handle without store".into(),
                }),
                ref other => Err(crate::ExprError::TypeMismatch {
                    expected: "list".into(),
                    found: other.type_name().into(),
                }),
            }
        }
        // Unique: need store for List, but return type error for non-list
        ExprHelper::Unique => {
            let value = one_arg(args, helper)?;
            match **value {
                SlotValue::List(_) => Err(crate::ExprError::TypeMismatch {
                    expected: "value-store context required for list deduplication".into(),
                    found: "list handle without store".into(),
                }),
                ref other => Err(crate::ExprError::TypeMismatch {
                    expected: "list".into(),
                    found: other.type_name().into(),
                }),
            }
        }
        // Contains: need store for list/text operations
        ExprHelper::Contains => {
            let (left, right) = two_args(args, helper)?;
            if matches!(***left, SlotValue::F64(_)) || matches!(***right, SlotValue::F64(_)) {
                return Err(crate::ExprError::TypeMismatch {
                    expected: "list, text, or object".into(),
                    found: "number".into(),
                });
            }
            Err(crate::ExprError::TypeMismatch {
                expected: "value-store context required for list contains check".into(),
                found: "list handle without store".into(),
            })
        }
        // StartsWith/EndsWith: need store for text operations
        ExprHelper::StartsWith | ExprHelper::EndsWith => {
            two_args(args, helper)?;
            Err(crate::ExprError::TypeMismatch {
                expected: "value-store context required for text operations".into(),
                found: "symbol handle without store".into(),
            })
        }
        // Has: need store for object field lookup
        ExprHelper::Has => {
            two_args(args, helper)?;
            Err(crate::ExprError::TypeMismatch {
                expected: "value-store context required for object field lookup".into(),
                found: "object handle without store".into(),
            })
        }
        // Append: need store for list append
        ExprHelper::Append => {
            two_args(args, helper)?;
            Err(crate::ExprError::TypeMismatch {
                expected: "value-store context required for list append".into(),
                found: "list handle without store".into(),
            })
        }
        // AppendIf: need store for conditional list append
        ExprHelper::AppendIf => {
            three_args(args, helper)?;
            Err(crate::ExprError::TypeMismatch {
                expected: "value-store context required for list append".into(),
                found: "list handle without store".into(),
            })
        }
        // Merge: need store for object merge
        ExprHelper::Merge => {
            two_args(args, helper)?;
            Err(crate::ExprError::TypeMismatch {
                expected: "value-store context required for object merge".into(),
                found: "object handle without store".into(),
            })
        }
        // Sum: need store for list sum
        ExprHelper::Sum => {
            one_arg(args, helper)?;
            Err(crate::ExprError::TypeMismatch {
                expected: "value-store context required for list sum".into(),
                found: "list handle without store".into(),
            })
        }
    }
}
