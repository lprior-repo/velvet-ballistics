use super::{ExprType, type_name};
use crate::parser::{ExprHelper, helper_arity, helper_name};
use crate::{ExprError, ExprResult};

/// Check position-specific type domains for helper arguments.
/// Arity must already be verified. Unknown types pass at all positions.
pub(super) fn check_helper_positions(helper: ExprHelper, arg_types: &[ExprType]) -> ExprResult<()> {
    match helper {
        ExprHelper::Exists => Ok(()),
        ExprHelper::Sum | ExprHelper::Unique => check_first_list(helper, arg_types),
        ExprHelper::Append => check_first_list(helper, arg_types),
        ExprHelper::AppendIf => check_append_if(helper, arg_types),
        ExprHelper::Merge => check_two_objects(helper, arg_types),
        ExprHelper::StartsWith | ExprHelper::EndsWith => check_two_text(helper, arg_types),
        ExprHelper::Has => check_object_text(helper, arg_types),
        ExprHelper::Length | ExprHelper::Count | ExprHelper::Empty | ExprHelper::Contains => Ok(()),
    }
}

/// Return the documented output type for a helper.
pub(super) fn helper_return_type(helper: ExprHelper) -> ExprType {
    match helper {
        ExprHelper::Exists
        | ExprHelper::StartsWith
        | ExprHelper::EndsWith
        | ExprHelper::Has
        | ExprHelper::Contains
        | ExprHelper::Empty => ExprType::Bool,
        ExprHelper::Length | ExprHelper::Sum | ExprHelper::Count => ExprType::I64,
        ExprHelper::Append | ExprHelper::AppendIf | ExprHelper::Unique => ExprType::List,
        ExprHelper::Merge => ExprType::Object,
    }
}

fn helper_arity_error(helper: ExprHelper, actual: usize) -> ExprError {
    ExprError::HelperArityMismatch {
        helper: helper_name(helper).into(),
        expected: helper_arity(helper),
        actual,
    }
}

fn check_first_list(helper: ExprHelper, arg_types: &[ExprType]) -> ExprResult<()> {
    match arg_types.first() {
        Some(ExprType::List) | Some(ExprType::Unknown) => Ok(()),
        Some(ty) => Err(ExprError::TypeMismatch {
            expected: "list".into(),
            found: type_name(*ty),
        }),
        None => Err(helper_arity_error(helper, 0)),
    }
}

fn check_append_if(helper: ExprHelper, arg_types: &[ExprType]) -> ExprResult<()> {
    match arg_types.first() {
        Some(ExprType::List) | Some(ExprType::Unknown) => {}
        Some(ty) => {
            return Err(ExprError::TypeMismatch {
                expected: "list".into(),
                found: type_name(*ty),
            });
        }
        None => return Err(helper_arity_error(helper, 0)),
    }
    match arg_types.get(2) {
        Some(ExprType::Bool) | Some(ExprType::Unknown) => Ok(()),
        Some(ty) => Err(ExprError::TypeMismatch {
            expected: "boolean".into(),
            found: type_name(*ty),
        }),
        None => Err(helper_arity_error(helper, arg_types.len())),
    }
}

fn check_two_objects(helper: ExprHelper, arg_types: &[ExprType]) -> ExprResult<()> {
    match arg_types.first() {
        Some(ExprType::Object) | Some(ExprType::Unknown) => {}
        Some(ty) => {
            return Err(ExprError::TypeMismatch {
                expected: "object".into(),
                found: type_name(*ty),
            });
        }
        None => return Err(helper_arity_error(helper, 0)),
    }
    match arg_types.get(1) {
        Some(ExprType::Object) | Some(ExprType::Unknown) => Ok(()),
        Some(ty) => Err(ExprError::TypeMismatch {
            expected: "object".into(),
            found: type_name(*ty),
        }),
        None => Err(helper_arity_error(helper, 1)),
    }
}

fn check_two_text(helper: ExprHelper, arg_types: &[ExprType]) -> ExprResult<()> {
    match arg_types.first() {
        Some(ExprType::Text) | Some(ExprType::Unknown) => {}
        Some(ty) => {
            return Err(ExprError::TypeMismatch {
                expected: "text".into(),
                found: type_name(*ty),
            });
        }
        None => return Err(helper_arity_error(helper, 0)),
    }
    match arg_types.get(1) {
        Some(ExprType::Text) | Some(ExprType::Unknown) => Ok(()),
        Some(ty) => Err(ExprError::TypeMismatch {
            expected: "text".into(),
            found: type_name(*ty),
        }),
        None => Err(helper_arity_error(helper, 1)),
    }
}

fn check_object_text(helper: ExprHelper, arg_types: &[ExprType]) -> ExprResult<()> {
    match arg_types.first() {
        Some(ExprType::Object) | Some(ExprType::Unknown) => {}
        Some(ty) => {
            return Err(ExprError::TypeMismatch {
                expected: "object".into(),
                found: type_name(*ty),
            });
        }
        None => return Err(helper_arity_error(helper, 0)),
    }
    match arg_types.get(1) {
        Some(ExprType::Text) | Some(ExprType::Unknown) => Ok(()),
        Some(ty) => Err(ExprError::TypeMismatch {
            expected: "text".into(),
            found: type_name(*ty),
        }),
        None => Err(helper_arity_error(helper, 1)),
    }
}
