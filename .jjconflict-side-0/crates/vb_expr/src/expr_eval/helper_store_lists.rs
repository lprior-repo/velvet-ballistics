use super::environment::{expect_bool, expect_list};
use crate::{ExprError, ExprResult};
use vb_core::value_store::ValueStore;
use vb_core::{SlotValue, Taint};

pub(super) fn eval_append_tainted(
    list: &SlotValue,
    item: &SlotValue,
    taint: Taint,
    condition: Option<&SlotValue>,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*list)?;
    let append = match condition {
        Some(flag) => expect_bool(*flag)?,
        None => true,
    };
    store
        .list(list_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("list:{list_id:?}"),
        })?;
    let (values, taints) = store
        .prepare_list_append(list_id, append.then_some((*item, taint)))
        .map_err(|_| ExprError::IntegerOverflow)?;
    let result = store
        .insert_list_with_taint(values, taints)
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::List(result))
}
pub(super) fn eval_helper_append_with_store(
    list: &SlotValue,
    item: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    eval_append_tainted(list, item, Taint::Clean, None, store)
}
pub(super) fn eval_helper_append_if_with_store(
    list: &SlotValue,
    item: &SlotValue,
    condition: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    eval_append_tainted(list, item, Taint::Clean, Some(condition), store)
}
pub(super) fn eval_helper_unique_with_store(
    value: &SlotValue,
    store: &mut ValueStore,
) -> ExprResult<SlotValue> {
    let list_id = expect_list(*value)?;
    store
        .list(list_id)
        .map_err(|_| ExprError::InvalidReference {
            reference: format!("list:{list_id:?}"),
        })?;
    let (values, taints) = store
        .prepare_list_unique(list_id)
        .map_err(|_| ExprError::IntegerOverflow)?;
    let result = store
        .insert_list_with_taint(values, taints)
        .map_err(|_| ExprError::IntegerOverflow)?;
    Ok(SlotValue::List(result))
}
