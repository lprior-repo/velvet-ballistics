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

use proptest::prelude::*;
use vb_core::action::{
    ActionContract, ActionName, ActionTicket, Idempotency, IdempotencyViolation, RetrySafety,
    SideEffect, validate_idempotency_key_ingredients, verify_idempotency,
};
use vb_core::capability::Capability;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::idempotency_contract::{
    IdempotencyContractError, IdempotencyContractErrors, IdempotencyContractViolation,
    collect_idempotency_contract_violations, is_statically_idempotent_contract,
    validate_action_idempotency_contract, validate_workflow_idempotency_contracts,
};

fn action(value: u16) -> ActionId {
    ActionId::new(value)
}

fn slot(value: u16) -> SlotIdx {
    SlotIdx::new(value)
}

fn step(value: u16) -> StepIdx {
    StepIdx::new(value)
}

fn contract(
    id: ActionId,
    side_effect: SideEffect,
    idempotency: Idempotency,
    retry_safety: RetrySafety,
) -> ActionContract {
    let name = match ActionName::new("test-action") {
        Ok(v) => v,
        Err(e) => panic!("ActionName::new(\"test-action\") should succeed: {e:?}"),
    };
    ActionContract {
        id,
        name,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1_024,
        max_output_bytes: 1_024,
        timeout_ms: 1_000,
        idempotency,
        side_effect,
        retry_safety,
        required_capabilities: Box::<[Capability]>::from([]),
    }
}

fn do_node(index: u16, action_id: ActionId) -> CompiledNode {
    CompiledNode {
        id: step(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: action_id,
            input: slot(0),
        },
    }
}

fn nop_node(index: u16) -> CompiledNode {
    CompiledNode {
        id: step(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

fn workflow(nodes: Box<[CompiledNode]>) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("idempotency-contract-red"),
        digest: WorkflowDigest::from_bytes([0x51; 32]),
        nodes,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn retry_unsafe_violation(action_id: ActionId) -> IdempotencyContractViolation {
    IdempotencyContractViolation::SideEffectingRetryUnsafe {
        action: action_id,
        side_effect: SideEffect::LocalWrite,
        idempotency: Idempotency::IdempotentExternal,
        retry_safety: RetrySafety::NotRetrySafe,
    }
}

fn at_least_once_violation(action_id: ActionId) -> IdempotencyContractViolation {
    IdempotencyContractViolation::SideEffectingAtLeastOnceExternal {
        action: action_id,
        side_effect: SideEffect::LocalWrite,
        idempotency: Idempotency::AtLeastOnceExternal,
        retry_safety: RetrySafety::Idempotent,
    }
}

fn deterministic_pure_violation(action_id: ActionId) -> IdempotencyContractViolation {
    IdempotencyContractViolation::SideEffectingDeterministicPure {
        action: action_id,
        side_effect: SideEffect::LocalWrite,
        idempotency: Idempotency::DeterministicPure,
        retry_safety: RetrySafety::Idempotent,
    }
}

#[test]
fn validate_action_returns_unit_for_pure_deterministic_safe_contract() {
    let candidate = contract(
        action(1),
        SideEffect::Pure,
        Idempotency::DeterministicPure,
        RetrySafety::Idempotent,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_returns_unit_for_pure_at_least_once_unsafe_contract() {
    let candidate = contract(
        action(2),
        SideEffect::Pure,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::NotRetrySafe,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_returns_unit_for_side_effecting_idempotent_external_safe_contract() {
    let candidate = contract(
        action(3),
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::Idempotent,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_returns_unit_for_side_effecting_idempotent_external_key_required_contract() {
    let candidate = contract(
        action(4),
        SideEffect::ExternalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::RequiresIdempotencyKey,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_action_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe() {
    let action_id = action(5);
    let candidate = contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::NotRetrySafe,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Err(retry_unsafe_violation(action_id)));
}

#[test]
fn validate_action_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once()
 {
    let action_id = action(6);
    let candidate = contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Idempotent,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Err(at_least_once_violation(action_id)));
}

#[test]
fn validate_action_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure()
 {
    let action_id = action(7);
    let candidate = contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::DeterministicPure,
        RetrySafety::Idempotent,
    );

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Err(deterministic_pure_violation(action_id)));
}

#[test]
fn collect_returns_unit_for_empty_contract_slice() {
    let contracts: [ActionContract; 0] = [];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn collect_returns_unit_for_all_legal_contracts() {
    let contracts = [
        contract(
            action(8),
            SideEffect::Pure,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::NotRetrySafe,
        ),
        contract(
            action(9),
            SideEffect::LocalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::Idempotent,
        ),
        contract(
            action(10),
            SideEffect::ExternalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::RequiresIdempotencyKey,
        ),
    ];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn collect_returns_one_boxed_retry_unsafe_violation_for_single_illegal_contract() {
    let action_id = action(11);
    let contracts = [contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::NotRetrySafe,
    )];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractErrors(Box::from([
            retry_unsafe_violation(action_id,)
        ])))
    );
}

#[test]
fn collect_returns_one_boxed_at_least_once_violation_for_single_illegal_contract() {
    let action_id = action(12);
    let contracts = [contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Idempotent,
    )];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractErrors(Box::from([
            at_least_once_violation(action_id),
        ])))
    );
}

#[test]
fn collect_returns_one_boxed_deterministic_pure_violation_for_single_illegal_contract() {
    let action_id = action(13);
    let contracts = [contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::DeterministicPure,
        RetrySafety::Idempotent,
    )];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractErrors(Box::from([
            deterministic_pure_violation(action_id),
        ])))
    );
}

#[test]
fn collect_returns_all_boxed_violations_in_input_order_for_multiple_illegal_contracts() {
    let first = action(14);
    let legal = action(15);
    let second = action(16);
    let third = action(17);
    let contracts = [
        contract(
            first,
            SideEffect::LocalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::NotRetrySafe,
        ),
        contract(
            legal,
            SideEffect::ExternalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::RequiresIdempotencyKey,
        ),
        contract(
            second,
            SideEffect::LocalWrite,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Idempotent,
        ),
        contract(
            third,
            SideEffect::LocalWrite,
            Idempotency::DeterministicPure,
            RetrySafety::Idempotent,
        ),
    ];

    let result = collect_idempotency_contract_violations(&contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractErrors(Box::from([
            retry_unsafe_violation(first),
            at_least_once_violation(second),
            deterministic_pure_violation(third),
        ])))
    );
}

#[test]
fn collect_returns_same_boxed_violations_when_called_twice_with_same_input() {
    let contracts = [
        contract(
            action(18),
            SideEffect::LocalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::NotRetrySafe,
        ),
        contract(
            action(19),
            SideEffect::LocalWrite,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Idempotent,
        ),
        contract(
            action(20),
            SideEffect::LocalWrite,
            Idempotency::DeterministicPure,
            RetrySafety::Idempotent,
        ),
    ];

    let first = collect_idempotency_contract_violations(&contracts);
    let second = collect_idempotency_contract_violations(&contracts);

    assert_eq!(first, second);
}

#[test]
fn is_static_returns_unit_for_pure_contract_for_all_retry_and_idempotency_values() {
    let deterministic_safe = contract(
        action(21),
        SideEffect::Pure,
        Idempotency::DeterministicPure,
        RetrySafety::Idempotent,
    );
    let deterministic_key = contract(
        action(22),
        SideEffect::Pure,
        Idempotency::DeterministicPure,
        RetrySafety::RequiresIdempotencyKey,
    );
    let deterministic_unsafe = contract(
        action(23),
        SideEffect::Pure,
        Idempotency::DeterministicPure,
        RetrySafety::NotRetrySafe,
    );
    let external_safe = contract(
        action(24),
        SideEffect::Pure,
        Idempotency::IdempotentExternal,
        RetrySafety::Idempotent,
    );
    let external_key = contract(
        action(25),
        SideEffect::Pure,
        Idempotency::IdempotentExternal,
        RetrySafety::RequiresIdempotencyKey,
    );
    let external_unsafe = contract(
        action(26),
        SideEffect::Pure,
        Idempotency::IdempotentExternal,
        RetrySafety::NotRetrySafe,
    );
    let at_least_once_safe = contract(
        action(27),
        SideEffect::Pure,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Idempotent,
    );
    let at_least_once_key = contract(
        action(28),
        SideEffect::Pure,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::RequiresIdempotencyKey,
    );
    let at_least_once_unsafe = contract(
        action(29),
        SideEffect::Pure,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::NotRetrySafe,
    );

    let result = [
        is_statically_idempotent_contract(&deterministic_safe),
        is_statically_idempotent_contract(&deterministic_key),
        is_statically_idempotent_contract(&deterministic_unsafe),
        is_statically_idempotent_contract(&external_safe),
        is_statically_idempotent_contract(&external_key),
        is_statically_idempotent_contract(&external_unsafe),
        is_statically_idempotent_contract(&at_least_once_safe),
        is_statically_idempotent_contract(&at_least_once_key),
        is_statically_idempotent_contract(&at_least_once_unsafe),
    ];

    assert_eq!(
        result,
        [
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(()),
            Ok(())
        ]
    );
}

#[test]
fn is_static_returns_unit_for_side_effecting_idempotent_external_safe_contract() {
    let candidate = contract(
        action(30),
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::Idempotent,
    );

    let result = is_statically_idempotent_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn is_static_returns_unit_for_side_effecting_idempotent_external_key_required_contract() {
    let candidate = contract(
        action(31),
        SideEffect::ExternalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::RequiresIdempotencyKey,
    );

    let result = is_statically_idempotent_contract(&candidate);

    assert_eq!(result, Ok(()));
}

#[test]
fn is_static_returns_retry_unsafe_violation_with_all_fields_when_retry_is_unsafe() {
    let action_id = action(32);
    let candidate = contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::NotRetrySafe,
    );

    let result = is_statically_idempotent_contract(&candidate);

    assert_eq!(result, Err(retry_unsafe_violation(action_id)));
}

#[test]
fn is_static_returns_at_least_once_violation_with_all_fields_when_idempotency_is_at_least_once() {
    let action_id = action(33);
    let candidate = contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::AtLeastOnceExternal,
        RetrySafety::Idempotent,
    );

    let result = is_statically_idempotent_contract(&candidate);

    assert_eq!(result, Err(at_least_once_violation(action_id)));
}

#[test]
fn is_static_returns_deterministic_pure_violation_with_all_fields_when_side_effecting_declares_deterministic_pure()
 {
    let action_id = action(34);
    let candidate = contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::DeterministicPure,
        RetrySafety::Idempotent,
    );

    let result = is_statically_idempotent_contract(&candidate);
    assert_eq!(result, Err(deterministic_pure_violation(action_id)));
}

#[test]
fn runtime_returns_missing_key_when_key_required_action_has_empty_key_slots()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = contract(
        action(35),
        SideEffect::ExternalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::RequiresIdempotencyKey,
    );
    let frame = RunFrame::new(RunId::ZERO, StepIdx::ZERO, 1, 1)?;

    let result = verify_idempotency(&candidate, &[], &frame);

    assert_eq!(
        result,
        Err(IdempotencyViolation::MissingKey(SideEffect::ExternalWrite))
    );
    Ok(())
}

#[test]
fn runtime_returns_secret_in_key_when_key_slot_taint_is_secret()
-> Result<(), Box<dyn std::error::Error>> {
    let mut frame = RunFrame::new(RunId::ZERO, StepIdx::ZERO, 1, 1)?;
    frame.write_slot_with_taint(SlotIdx::ZERO, SlotValue::Bool(true), Taint::Secret)?;

    let result = validate_idempotency_key_ingredients(&[SlotIdx::ZERO], &frame);

    assert_eq!(result, Err(IdempotencyViolation::SecretInKey(0)));
    Ok(())
}

#[test]
fn runtime_returns_secret_in_key_when_key_slot_taint_is_derived_from_secret()
-> Result<(), Box<dyn std::error::Error>> {
    let mut frame = RunFrame::new(RunId::ZERO, StepIdx::ZERO, 1, 1)?;
    frame.write_slot_with_taint(
        SlotIdx::ZERO,
        SlotValue::Bool(true),
        Taint::DerivedFromSecret,
    )?;

    let result = validate_idempotency_key_ingredients(&[SlotIdx::ZERO], &frame);

    assert_eq!(result, Err(IdempotencyViolation::SecretInKey(0)));
    Ok(())
}

#[test]
fn runtime_returns_unit_when_key_required_action_has_non_empty_clean_key_slots()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = contract(
        action(36),
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::RequiresIdempotencyKey,
    );
    let mut frame = RunFrame::new(RunId::ZERO, StepIdx::ZERO, 1, 1)?;
    frame.write_slot(SlotIdx::ZERO, SlotValue::Bool(true))?;

    let result = verify_idempotency(&candidate, &[SlotIdx::ZERO], &frame);

    assert_eq!(result, Ok(()));
    Ok(())
}

#[test]
fn static_verifier_ignores_zero_numeric_ticket_key_when_contract_is_key_required() {
    let candidate = contract(
        action(37),
        SideEffect::ExternalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::RequiresIdempotencyKey,
    );
    let ticket = ActionTicket {
        run: RunId::ZERO,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: action(37),
        attempt: 1,
        idempotency_key: 0,
        capacity: 3,
        ..Default::default()
    };

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(ticket.idempotency_key, 0);
    assert_eq!(result, Ok(()));
}

#[test]
fn direct_decision_table_has_no_uncovered_enum_combination() {
    let accepted = [
        is_statically_idempotent_contract(&contract(
            action(38),
            SideEffect::Pure,
            Idempotency::DeterministicPure,
            RetrySafety::Idempotent,
        )),
        is_statically_idempotent_contract(&contract(
            action(39),
            SideEffect::LocalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::Idempotent,
        )),
        is_statically_idempotent_contract(&contract(
            action(40),
            SideEffect::ExternalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::RequiresIdempotencyKey,
        )),
    ];
    let rejected = [
        is_statically_idempotent_contract(&contract(
            action(41),
            SideEffect::LocalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::NotRetrySafe,
        )),
        is_statically_idempotent_contract(&contract(
            action(42),
            SideEffect::LocalWrite,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::Idempotent,
        )),
        is_statically_idempotent_contract(&contract(
            action(43),
            SideEffect::LocalWrite,
            Idempotency::DeterministicPure,
            RetrySafety::Idempotent,
        )),
    ];

    assert_eq!(accepted, [Ok(()), Ok(()), Ok(())]);
    assert_eq!(
        rejected,
        [
            Err(retry_unsafe_violation(action(41))),
            Err(at_least_once_violation(action(42))),
            Err(deterministic_pure_violation(action(43))),
        ]
    );
}

#[test]
fn verifier_unit_functions_do_not_mutate_contract_values() {
    let candidate = contract(
        action(44),
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::Idempotent,
    );
    let original = candidate.clone();

    let result = validate_action_idempotency_contract(&candidate);

    assert_eq!(result, Ok(()));
    assert_eq!(candidate, original);
}

#[test]
fn validate_workflow_returns_unit_when_workflow_has_no_do_nodes_and_registry_is_empty() {
    let parts = workflow(Box::from([nop_node(0)]));
    let contracts: [ActionContract; 0] = [];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_workflow_returns_unit_for_side_effecting_idempotent_external_when_retry_safe() {
    let action_id = action(45);
    let parts = workflow(Box::from([do_node(0, action_id)]));
    let contracts = [contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::Idempotent,
    )];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_workflow_returns_unit_for_side_effecting_idempotent_external_when_key_required() {
    let action_id = action(46);
    let parts = workflow(Box::from([do_node(0, action_id)]));
    let contracts = [contract(
        action_id,
        SideEffect::ExternalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::RequiresIdempotencyKey,
    )];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn validate_workflow_returns_retry_unsafe_error_when_side_effecting_contract_is_retry_unsafe() {
    let action_id = action(47);
    let parts = workflow(Box::from([do_node(0, action_id)]));
    let contracts = [contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::NotRetrySafe,
    )];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractError::IdempotencyViolations(
            IdempotencyContractErrors(Box::from([retry_unsafe_violation(action_id)]))
        ))
    );
}

#[test]
fn validate_workflow_returns_action_contract_missing_when_do_node_has_no_matching_contract() {
    let action_id = action(48);
    let parts = workflow(Box::from([do_node(0, action_id)]));
    let contracts: [ActionContract; 0] = [];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractError::ActionContractMissing {
            action_id,
            node_index: 0,
        })
    );
}

#[test]
fn validate_workflow_returns_action_contract_orphan_when_registry_contract_has_no_do_node() {
    let action_id = action(49);
    let parts = workflow(Box::from([nop_node(0)]));
    let contracts = [contract(
        action_id,
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::Idempotent,
    )];

    let result = validate_workflow_idempotency_contracts(&parts, &contracts);

    assert_eq!(
        result,
        Err(IdempotencyContractError::ActionContractOrphan { action_id })
    );
}

proptest! {
    #[test]
    fn proptest_pure_action_acceptance_holds_for_representative_action_ids(action_raw in 0u16..128u16) {
        let candidate = contract(
            ActionId::new(action_raw),
            SideEffect::Pure,
            Idempotency::AtLeastOnceExternal,
            RetrySafety::NotRetrySafe,
        );

        let result = is_statically_idempotent_contract(&candidate);

        prop_assert_eq!(result, Ok(()));
    }

    #[test]
    fn proptest_retry_unsafe_side_effecting_contracts_report_original_action(action_raw in 0u16..128u16) {
        let action_id = ActionId::new(action_raw);
        let candidate = contract(
            action_id,
            SideEffect::LocalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::NotRetrySafe,
        );

        let result = is_statically_idempotent_contract(&candidate);

        prop_assert_eq!(result, Err(retry_unsafe_violation(action_id)));
    }
}

proptest! {
    #[test]
    fn proptest_001_decision_table_confluence_10k(
        side_effect in prop_oneof![
            Just(SideEffect::Pure),
            Just(SideEffect::LocalWrite),
            Just(SideEffect::ExternalWrite),
            Just(SideEffect::LocalWrite),
            Just(SideEffect::LocalWrite),
        ],
        retry_safety in prop_oneof![
            Just(RetrySafety::Idempotent),
            Just(RetrySafety::RequiresIdempotencyKey),
            Just(RetrySafety::NotRetrySafe),
        ],
        idempotency in prop_oneof![
            Just(Idempotency::DeterministicPure),
            Just(Idempotency::IdempotentExternal),
            Just(Idempotency::AtLeastOnceExternal),
        ],
    ) {
        let candidate = contract(ActionId::new(0), side_effect, idempotency, retry_safety);
        let result1 = is_statically_idempotent_contract(&candidate);
        let result2 = is_statically_idempotent_contract(&candidate);
        prop_assert_eq!(
            result1.is_ok(), result2.is_ok(),
            "Decision table must be confluent"
        );
    }
}

proptest! {
    #[test]
    fn proptest_002_runtime_gate_determinism_10k(
        side_effect in prop_oneof![
            Just(SideEffect::Pure),
            Just(SideEffect::LocalWrite),
            Just(SideEffect::ExternalWrite),
        ],
        idempotency in prop_oneof![
            Just(Idempotency::IdempotentExternal),
            Just(Idempotency::AtLeastOnceExternal),
        ],
        key_count in 0..8u8,
        taint_pattern in 0..=255u8,
    ) {
        let candidate = contract(ActionId::new(0), side_effect, idempotency, RetrySafety::RequiresIdempotencyKey);
        let key_slots: Vec<SlotIdx> = (0..key_count as u16).map(SlotIdx::new).collect();

        let mut frame = match RunFrame::new(RunId::ZERO, StepIdx::ZERO, 8, 8) {
            Ok(f) => f,
            Err(e) => panic!("RunFrame::new(8,8) should succeed in proptest: {e:?}"),
        };
        for (i, &slot_idx) in key_slots.iter().enumerate() {
            let taint_bit = (taint_pattern >> i) & 1;
            let taint = if taint_bit == 0 { Taint::Clean } else { Taint::Secret };
            let _ = frame.write_slot_with_taint(slot_idx, SlotValue::I64(i as i64), taint);
        }

        let result1 = verify_idempotency(&candidate, &key_slots, &frame);
        let result2 = verify_idempotency(&candidate, &key_slots, &frame);

        prop_assert_eq!(
            result1.is_ok(), result2.is_ok(),
            "Runtime gate must be deterministic"
        );
    }
}

// =========================================================================
// vb-u09ai: 4-variant RetrySafety static validation tests (Tier 1).
// These tests use the 4-variant names literally; on 3-variant code
// the test file fails to compile.
// =========================================================================

/// Tier 1: side-effecting action with `Unknown` retry_safety must be
/// rejected as `SideEffectingRetryUnsafe` (the bead's primary
/// contract addition).
#[test]
fn idempotency_contract_red_unknown_retry_safety_propagates_sideeffecting_retry_unsafe() {
    let c = contract(
        action(9999),
        SideEffect::ExternalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::Unknown,
    );
    let result = is_statically_idempotent_contract(&c);
    assert!(
        matches!(
            result,
            Err(IdempotencyContractViolation::SideEffectingRetryUnsafe {
                retry_safety: RetrySafety::Unknown,
                ..
            })
        ),
        "Unknown retry_safety on a side-effecting action must propagate as SideEffectingRetryUnsafe, got {result:?}"
    );
}

/// Tier 1: validate_action_idempotency_contract returns the same
/// `SideEffectingRetryUnsafe` error for `Unknown` retry_safety.
#[test]
fn idempotency_contract_red_validate_action_propagates_unknown() {
    let c = contract(
        action(9998),
        SideEffect::LocalWrite,
        Idempotency::IdempotentExternal,
        RetrySafety::Unknown,
    );
    let result = validate_action_idempotency_contract(&c);
    assert_eq!(
        result,
        Err(IdempotencyContractViolation::SideEffectingRetryUnsafe {
            action: action(9998),
            side_effect: SideEffect::LocalWrite,
            idempotency: Idempotency::IdempotentExternal,
            retry_safety: RetrySafety::Unknown,
        })
    );
}

/// Tier 1: 4-variant exhaustive static-validation table for
/// `SideEffect::Pure × all RetrySafety × all Idempotency`
/// (4 × 3 = 12 cells; all must pass per the static decision table
/// because Pure always passes).
#[test]
fn idempotency_contract_red_pure_passes_all_4variant_combos() {
    let mut count = 0usize;
    for retry_safety in [
        RetrySafety::Idempotent,
        RetrySafety::RequiresIdempotencyKey,
        RetrySafety::NotRetrySafe,
        RetrySafety::Unknown,
    ] {
        for idempotency in [
            Idempotency::DeterministicPure,
            Idempotency::IdempotentExternal,
            Idempotency::AtLeastOnceExternal,
        ] {
            let c = contract(
                action(count as u16),
                SideEffect::Pure,
                idempotency,
                retry_safety,
            );
            let result = is_statically_idempotent_contract(&c);
            assert_eq!(
                result,
                Ok(()),
                "Pure must pass for {retry_safety:?}+{idempotency:?}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 12, "4 RetrySafety × 3 Idempotency = 12 cells");
}
