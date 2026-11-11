#[path = "support/contains_three_routes.rs"]
mod contains_three_routes;

use contains_three_routes::{ContainsFixture, assert_routes, fixture_with_store, frame};
use vb_core::replay::ReplayError;
use vb_core::{EngineError, ListId, SlotValue, SymbolId, ValueStore};
use vb_expr::ExprError;

fn fixture(items: Vec<SlotValue>) -> Result<ContainsFixture, String> {
    fixture_with_store(ValueStore::new(), items)
}

fn assert_route_errors(
    fixture: &mut ContainsFixture,
    needle: SlotValue,
    direct_expected: ExprError,
    hot_expected: EngineError,
) -> Result<(), String> {
    let args = [fixture.receiver, needle];
    let direct = vb_expr::eval::eval_helper_with_store(
        vb_expr::eval::ExprHelper::Contains,
        &args,
        &mut fixture.store,
    );
    assert_eq!(direct, Err(direct_expected));
    assert_hot_error(fixture, needle, hot_expected)?;
    assert_replay_error(fixture, needle)
}

fn assert_hot_error(
    fixture: &mut ContainsFixture,
    needle: SlotValue,
    expected: EngineError,
) -> Result<(), String> {
    let run = frame(&fixture.plan, fixture.receiver, needle)?;
    let result = vb_core::engine::eval_expr_with_store(
        &fixture.plan,
        &run,
        &mut fixture.store,
        vb_core::ExprIdx::new(0),
    );
    assert_eq!(result, Err(expected));
    Ok(())
}

fn assert_replay_error(fixture: &mut ContainsFixture, needle: SlotValue) -> Result<(), String> {
    let mut run = frame(&fixture.plan, fixture.receiver, needle)?;
    let step = vb_core::StepIdx::new(1);
    let node = fixture
        .plan
        .node(step)
        .ok_or_else(|| "missing choose node".to_owned())?;
    let result =
        vb_core::replay::step::replay_step(node, &mut run, &mut fixture.store, &fixture.plan);
    assert!(
        matches!(result, Err(ReplayError::ExpressionEvalFailed { step: actual }) if actual == step)
    );
    assert_eq!(run.pc(), step);
    Ok(())
}

#[test]
fn null_is_found_in_list_on_all_routes() -> Result<(), String> {
    let mut fixture = fixture(vec![SlotValue::Null])?;
    assert_routes(&mut fixture, SlotValue::Null, true)
}

#[test]
fn identical_raw_invalid_list_handle_is_found_on_all_routes() -> Result<(), String> {
    let invalid = SlotValue::List(ListId::new(u32::MAX));
    let mut fixture = fixture(vec![invalid])?;
    assert_routes(&mut fixture, invalid, true)
}

#[test]
fn distinct_raw_invalid_list_handle_is_not_found_on_all_routes() -> Result<(), String> {
    let stored = SlotValue::List(ListId::new(u32::MAX));
    let absent = SlotValue::List(ListId::new(u32::MAX - 1));
    assert_ne!(stored, absent);
    let mut fixture = fixture(vec![stored])?;
    assert_routes(&mut fixture, absent, false)
}

#[test]
fn singleton_list_missing_value_is_false_on_all_routes() -> Result<(), String> {
    let mut fixture = fixture(vec![SlotValue::I64(11)])?;
    assert_routes(&mut fixture, SlotValue::I64(7), false)
}

#[test]
fn null_receiver_wins_over_invalid_symbol_needle_on_all_routes() -> Result<(), String> {
    let mut fixture = fixture(Vec::new())?;
    fixture.receiver = SlotValue::Null;
    let needle = SlotValue::Symbol(SymbolId::new(u32::MAX));
    assert_route_errors(
        &mut fixture,
        needle,
        ExprError::TypeMismatch {
            expected: "list".into(),
            found: "null".into(),
        },
        EngineError::TypeMismatch {
            expected: "list",
            found: "null",
        },
    )
}

#[test]
fn invalid_list_receiver_wins_over_invalid_symbol_needle_on_all_routes() -> Result<(), String> {
    let mut fixture = fixture(Vec::new())?;
    let invalid = ListId::new(u32::MAX);
    fixture.receiver = SlotValue::List(invalid);
    let needle = SlotValue::Symbol(SymbolId::new(u32::MAX));
    assert_route_errors(
        &mut fixture,
        needle,
        ExprError::InvalidReference {
            reference: format!("list:{invalid:?}"),
        },
        EngineError::ListOutOfBounds { list: invalid },
    )
}

#[test]
fn empty_list_invalid_handle_needle_is_false_on_all_routes() -> Result<(), String> {
    let mut fixture = fixture(Vec::new())?;
    assert_routes(&mut fixture, SlotValue::List(ListId::new(u32::MAX)), false)
}

#[test]
fn invalid_stored_handle_scalar_needle_is_false_on_all_routes() -> Result<(), String> {
    let mut fixture = fixture(vec![SlotValue::List(ListId::new(u32::MAX))])?;
    assert_routes(&mut fixture, SlotValue::I64(7), false)
}
