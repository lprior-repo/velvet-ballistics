#![forbid(unsafe_code)]
//! Type enforcement utilities for expression evaluation.

use vb_core::SlotValue;
use vb_core::ids::{ListId, ObjectId, SymbolId};

use crate::ExprResult;

pub(crate) fn expect_bool(value: SlotValue) -> ExprResult<bool> {
    match value {
        SlotValue::Bool(b) => Ok(b),
        other => Err(crate::ExprError::TypeMismatch {
            expected: "boolean".into(),
            found: other.type_name().into(),
        }),
    }
}

pub(crate) fn expect_i64(value: SlotValue) -> ExprResult<i64> {
    match value {
        SlotValue::I64(n) => Ok(n),
        other => Err(crate::ExprError::TypeMismatch {
            expected: "number".into(),
            found: other.type_name().into(),
        }),
    }
}

pub(crate) fn expect_symbol(value: SlotValue) -> ExprResult<SymbolId> {
    match value {
        SlotValue::Symbol(id) => Ok(id),
        other => Err(crate::ExprError::TypeMismatch {
            expected: "text".into(),
            found: other.type_name().into(),
        }),
    }
}

pub(crate) fn expect_list(value: SlotValue) -> ExprResult<ListId> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(crate::ExprError::TypeMismatch {
            expected: "list".into(),
            found: other.type_name().into(),
        }),
    }
}

pub(crate) fn expect_object(value: SlotValue) -> ExprResult<ObjectId> {
    match value {
        SlotValue::Object(id) => Ok(id),
        other => Err(crate::ExprError::TypeMismatch {
            expected: "object".into(),
            found: other.type_name().into(),
        }),
    }
}
